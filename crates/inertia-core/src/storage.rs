use std::collections::HashMap;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::content::{ContentType, DeliveryStatus};
use crate::error::{CoreError, CoreResult};
use crate::identity::{encode_hex, Identity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub phone_hash: Option<String>,
    pub display_name: String,
    pub peer_id: Option<String>,
    pub signing_pubkey: String,
    pub encryption_pubkey: String,
    pub last_seen: Option<DateTime<Utc>>,
    pub connection_state: ConnectionState,
    #[serde(default)]
    pub multiaddrs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Online,
    Offline,
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub content_id: String,
    pub recipient_id: String,
    pub status: DeliveryStatus,
    pub expires_at: DateTime<Utc>,
    pub retry_count: u32,
    pub ciphertext: Vec<u8>,
    pub content_type: ContentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxEntry {
    pub content_id: String,
    pub sender_id: String,
    pub received_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub body: String,
    pub media_ref: Option<String>,
    pub content_type: ContentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPost {
    pub content_id: String,
    pub body: String,
    pub media_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePhoto {
    pub id: String,
    pub blob_hash: String,
    pub caption: Option<String>,
    pub content_id: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostComment {
    pub id: String,
    pub post_id: String,
    pub author_id: String,
    pub author_name: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub content_id: String,
    pub author_id: String,
    pub author_name: String,
    pub body: String,
    pub media_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_own: bool,
    pub is_archived: bool,
    pub comment_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub feed_history_enabled: bool,
    pub p2p_listen_port: u16,
    pub relay_multiaddr: Option<String>,
    pub p2p_announce: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedFeedItem {
    pub content_id: String,
    pub author_id: String,
    pub author_name: String,
    pub body: String,
    pub media_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_own: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedBackup {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub items: Vec<ArchivedFeedItem>,
    pub blobs: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedRestoreReport {
    pub items_imported: usize,
    pub blobs_imported: usize,
}

const FEED_HISTORY_KEY: &str = "feed_history_enabled";
const P2P_LISTEN_PORT_KEY: &str = "p2p_listen_port";
const RELAY_MULTIADDR_KEY: &str = "relay_multiaddr";
const P2P_ANNOUNCE_KEY: &str = "p2p_announce";
const ARCHIVED_EXPIRES_AT: &str = "2099-01-01T00:00:00+00:00";

fn archived_expires_at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(ARCHIVED_EXPIRES_AT)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now() + chrono::Duration::days(365 * 10))
}

impl ArchivedFeedItem {
    pub fn to_feed_item(&self) -> FeedItem {
        FeedItem {
            content_id: self.content_id.clone(),
            author_id: self.author_id.clone(),
            author_name: self.author_name.clone(),
            body: self.body.clone(),
            media_ref: self.media_ref.clone(),
            created_at: self.created_at,
            expires_at: archived_expires_at(),
            is_own: self.is_own,
            is_archived: true,
            comment_count: 0,
        }
    }
}

impl From<&FeedItem> for ArchivedFeedItem {
    fn from(item: &FeedItem) -> Self {
        Self {
            content_id: item.content_id.clone(),
            author_id: item.author_id.clone(),
            author_name: item.author_name.clone(),
            body: item.body.clone(),
            media_ref: item.media_ref.clone(),
            created_at: item.created_at,
            is_own: item.is_own,
        }
    }
}

fn status_str(status: DeliveryStatus) -> &'static str {
    match status {
        DeliveryStatus::Pending => "pending",
        DeliveryStatus::Failed => "failed",
        DeliveryStatus::Delivered => "delivered",
        DeliveryStatus::Expired => "expired",
    }
}

fn parse_status(s: &str) -> DeliveryStatus {
    match s {
        "failed" => DeliveryStatus::Failed,
        "delivered" => DeliveryStatus::Delivered,
        "expired" => DeliveryStatus::Expired,
        _ => DeliveryStatus::Pending,
    }
}

fn content_type_str(t: ContentType) -> &'static str {
    match t {
        ContentType::Message => "message",
        ContentType::Post => "post",
        ContentType::Comment => "comment",
    }
}

fn parse_content_type(s: &str) -> ContentType {
    match s {
        "post" => ContentType::Post,
        "comment" => ContentType::Comment,
        _ => ContentType::Message,
    }
}

fn connection_state_str(s: ConnectionState) -> &'static str {
    match s {
        ConnectionState::Online => "online",
        ConnectionState::Offline => "offline",
        ConnectionState::Unreachable => "unreachable",
    }
}

fn parse_connection_state(s: &str) -> ConnectionState {
    match s {
        "online" => ConnectionState::Online,
        "unreachable" => ConnectionState::Unreachable,
        _ => ConnectionState::Offline,
    }
}

fn encode_multiaddrs(addrs: &[String]) -> String {
    serde_json::to_string(addrs).unwrap_or_else(|_| "[]".to_string())
}

fn decode_multiaddrs(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn merge_multiaddr_lists(existing: &[String], new: &[String]) -> Vec<String> {
    let mut out = existing.to_vec();
    for addr in new {
        if !out.contains(addr) {
            out.push(addr.clone());
        }
    }
    out
}

pub struct Store {
    conn: Connection,
    data_dir: PathBuf,
    blob_dir: PathBuf,
}

impl Store {
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn open(data_dir: impl AsRef<Path>) -> CoreResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        let blob_dir = data_dir.join("blobs");
        std::fs::create_dir_all(&blob_dir)?;

        let conn = Connection::open(data_dir.join("inertia.db"))?;
        let store = Self {
            conn,
            data_dir,
            blob_dir,
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS identity (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                signing_pubkey TEXT NOT NULL,
                encryption_pubkey TEXT NOT NULL,
                phone_hash TEXT,
                display_name TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                phone_hash TEXT,
                display_name TEXT NOT NULL,
                peer_id TEXT,
                signing_pubkey TEXT NOT NULL,
                encryption_pubkey TEXT NOT NULL,
                last_seen TEXT,
                connection_state TEXT NOT NULL DEFAULT 'offline'
            );

            CREATE TABLE IF NOT EXISTS outbox (
                content_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                status TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                ciphertext BLOB NOT NULL,
                content_type TEXT NOT NULL,
                envelope_json TEXT NOT NULL,
                PRIMARY KEY (content_id, recipient_id)
            );

            CREATE TABLE IF NOT EXISTS inbox (
                content_id TEXT PRIMARY KEY,
                sender_id TEXT NOT NULL,
                received_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                read_at TEXT,
                body TEXT NOT NULL,
                content_type TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS delivery_acks (
                content_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                acked_at TEXT NOT NULL,
                PRIMARY KEY (content_id, recipient_id)
            );

            CREATE TABLE IF NOT EXISTS issued_invites (
                nonce TEXT PRIMARY KEY,
                expires_at TEXT NOT NULL,
                consumed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS redeemed_invites (
                nonce TEXT PRIMARY KEY,
                issuer_signing_pubkey TEXT NOT NULL,
                redeemed_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS local_posts (
                content_id TEXT PRIMARY KEY,
                body TEXT NOT NULL,
                media_ref TEXT,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS profile_photos (
                id TEXT PRIMARY KEY,
                blob_hash TEXT NOT NULL,
                caption TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            ",
        )?;
        self.ensure_identity_key_columns()?;
        self.ensure_identity_bio_column()?;
        self.ensure_inbox_media_ref_column()?;
        self.ensure_feed_archive_tables()?;
        self.ensure_post_comments_table()?;
        self.ensure_profile_photo_content_id_column()?;
        self.ensure_contact_multiaddrs_column()?;
        Ok(())
    }

    fn ensure_contact_multiaddrs_column(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(contacts)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();
        if !cols.iter().any(|c| c == "multiaddrs") {
            self.conn.execute(
                "ALTER TABLE contacts ADD COLUMN multiaddrs TEXT NOT NULL DEFAULT '[]'",
                [],
            )?;
        }
        Ok(())
    }

    fn ensure_post_comments_table(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS post_comments (
                id TEXT PRIMARY KEY,
                post_id TEXT NOT NULL,
                author_id TEXT NOT NULL,
                author_name TEXT NOT NULL,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_post_comments_post_id ON post_comments(post_id);
            ",
        )?;
        Ok(())
    }

    fn ensure_profile_photo_content_id_column(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(profile_photos)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();

        if !cols.iter().any(|c| c == "content_id") {
            self.conn
                .execute("ALTER TABLE profile_photos ADD COLUMN content_id TEXT", [])?;
        }
        Ok(())
    }

    fn ensure_feed_archive_tables(&self) -> CoreResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS feed_archive (
                content_id TEXT PRIMARY KEY,
                author_id TEXT NOT NULL,
                author_name TEXT NOT NULL,
                body TEXT NOT NULL,
                media_ref TEXT,
                created_at TEXT NOT NULL,
                is_own INTEGER NOT NULL DEFAULT 0
            );
            ",
        )?;
        Ok(())
    }

    fn ensure_inbox_media_ref_column(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(inbox)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();

        if !cols.iter().any(|c| c == "media_ref") {
            self.conn
                .execute("ALTER TABLE inbox ADD COLUMN media_ref TEXT", [])?;
        }
        Ok(())
    }

    fn ensure_identity_key_columns(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(identity)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();

        if !cols.iter().any(|c| c == "signing_key") {
            self.conn
                .execute("ALTER TABLE identity ADD COLUMN signing_key TEXT", [])?;
        }
        if !cols.iter().any(|c| c == "encryption_secret") {
            self.conn
                .execute("ALTER TABLE identity ADD COLUMN encryption_secret TEXT", [])?;
        }
        Ok(())
    }

    fn ensure_identity_bio_column(&self) -> CoreResult<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(identity)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>("name"))?
            .filter_map(Result::ok)
            .collect();

        if !cols.iter().any(|c| c == "bio") {
            self.conn
                .execute("ALTER TABLE identity ADD COLUMN bio TEXT NOT NULL DEFAULT ''", [])?;
        }
        Ok(())
    }

    pub fn update_identity_profile(
        &self,
        display_name: &str,
        bio: &str,
    ) -> CoreResult<()> {
        if display_name.trim().is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }
        let updated = self.conn.execute(
            "UPDATE identity SET display_name = ?1, bio = ?2 WHERE id = 1",
            params![display_name.trim(), bio.trim()],
        )?;
        if updated == 0 {
            return Err(CoreError::IdentityNotInitialized);
        }
        Ok(())
    }

    pub fn has_profile(&self) -> CoreResult<bool> {
        Ok(self.load_identity()?.is_some())
    }

    pub fn create_identity(&self, identity: &Identity) -> CoreResult<()> {
        if self.has_profile()? {
            return Err(CoreError::ProfileAlreadyExists);
        }

        let signing_key = identity.signing_key()?.to_bytes();
        let encryption_secret = identity.encryption_secret()?.to_bytes();

        self.conn
            .execute(
                "INSERT INTO identity
                 (id, signing_pubkey, encryption_pubkey, phone_hash, display_name, bio, signing_key, encryption_secret)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    identity.signing_pubkey,
                    identity.encryption_pubkey,
                    identity.phone_hash,
                    identity.display_name,
                    identity.bio,
                    encode_hex(signing_key),
                    encode_hex(encryption_secret),
                ],
            )
            .map_err(|e| {
                if matches!(
                    e,
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error {
                            code: rusqlite::ErrorCode::ConstraintViolation,
                            ..
                        },
                        _
                    )
                ) {
                    CoreError::ProfileAlreadyExists
                } else {
                    CoreError::Database(e)
                }
            })?;
        Ok(())
    }

    pub fn load_identity(&self) -> CoreResult<Option<Identity>> {
        let mut stmt = self.conn.prepare(
            "SELECT signing_pubkey, encryption_pubkey, phone_hash, display_name, bio, signing_key, encryption_secret
             FROM identity WHERE id = 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let identity = Identity::from_persisted(
                row.get("signing_pubkey")?,
                row.get("encryption_pubkey")?,
                row.get("phone_hash")?,
                row.get("display_name")?,
                row.get("bio").unwrap_or_default(),
                row.get("signing_key")?,
                row.get("encryption_secret")?,
            )?;
            if identity.is_initialized() {
                return Ok(Some(identity));
            }
        }
        Ok(None)
    }

    pub fn upsert_contact(&self, contact: &Contact) -> CoreResult<()> {
        let mut contact = contact.clone();
        if let Ok(existing) = self.get_contact(&contact.id) {
            contact.multiaddrs =
                merge_multiaddr_lists(&existing.multiaddrs, &contact.multiaddrs);
        }
        self.conn.execute(
            "INSERT OR REPLACE INTO contacts
             (id, phone_hash, display_name, peer_id, signing_pubkey, encryption_pubkey, last_seen, connection_state, multiaddrs)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                contact.id,
                contact.phone_hash,
                contact.display_name,
                contact.peer_id,
                contact.signing_pubkey,
                contact.encryption_pubkey,
                contact.last_seen.map(|t| t.to_rfc3339()),
                connection_state_str(contact.connection_state),
                encode_multiaddrs(&contact.multiaddrs),
            ],
        )?;
        Ok(())
    }

    pub fn merge_contact_multiaddrs_by_peer_id(
        &self,
        peer_id: &str,
        new_addrs: &[String],
    ) -> CoreResult<()> {
        for mut contact in self.list_contacts()? {
            if contact.peer_id.as_deref() == Some(peer_id) {
                contact.multiaddrs = merge_multiaddr_lists(&contact.multiaddrs, new_addrs);
                self.upsert_contact(&contact)?;
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn list_contacts(&self) -> CoreResult<Vec<Contact>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, phone_hash, display_name, peer_id, signing_pubkey, encryption_pubkey, last_seen, connection_state, multiaddrs
             FROM contacts ORDER BY display_name",
        )?;
        let rows = stmt.query_map([], |row| {
            let state_str: String = row.get("connection_state")?;
            let connection_state = parse_connection_state(&state_str);
            let multiaddrs_raw: String = row.get("multiaddrs").unwrap_or_else(|_| "[]".into());
            Ok(Contact {
                id: row.get("id")?,
                phone_hash: row.get("phone_hash")?,
                display_name: row.get("display_name")?,
                peer_id: row.get("peer_id")?,
                signing_pubkey: row.get("signing_pubkey")?,
                encryption_pubkey: row.get("encryption_pubkey")?,
                last_seen: row
                    .get::<_, Option<String>>("last_seen")?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                connection_state,
                multiaddrs: decode_multiaddrs(&multiaddrs_raw),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn get_contact(&self, id: &str) -> CoreResult<Contact> {
        let contacts = self.list_contacts()?;
        contacts
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| CoreError::ContactNotFound(id.to_string()))
    }

    pub fn get_contact_by_phone_hash(&self, phone_hash: &str) -> CoreResult<Option<Contact>> {
        Ok(self
            .list_contacts()?
            .into_iter()
            .find(|c| c.phone_hash.as_deref() == Some(phone_hash)))
    }

    pub fn insert_outbox(&self, entry: &OutboxEntry, envelope_json: &str) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO outbox
             (content_id, recipient_id, status, expires_at, retry_count, ciphertext, content_type, envelope_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.content_id,
                entry.recipient_id,
                status_str(entry.status),
                entry.expires_at.to_rfc3339(),
                entry.retry_count,
                entry.ciphertext,
                content_type_str(entry.content_type),
                envelope_json,
            ],
        )?;
        Ok(())
    }

    pub fn list_outbox(&self) -> CoreResult<Vec<OutboxEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, recipient_id, status, expires_at, retry_count, ciphertext, content_type
             FROM outbox ORDER BY expires_at",
        )?;
        let rows = stmt.query_map([], |row| {
            let status: String = row.get("status")?;
            let content_type: String = row.get("content_type")?;
            Ok(OutboxEntry {
                content_id: row.get("content_id")?,
                recipient_id: row.get("recipient_id")?,
                status: parse_status(&status),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                retry_count: row.get("retry_count")?,
                ciphertext: row.get("ciphertext")?,
                content_type: parse_content_type(&content_type),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn get_outbox_envelope(&self, content_id: &str, recipient_id: &str) -> CoreResult<String> {
        let mut stmt = self.conn.prepare(
            "SELECT envelope_json FROM outbox WHERE content_id = ?1 AND recipient_id = ?2",
        )?;
        let mut rows = stmt.query(params![content_id, recipient_id])?;
        if let Some(row) = rows.next()? {
            Ok(row.get("envelope_json")?)
        } else {
            Err(CoreError::ContentNotFound(format!(
                "{content_id}:{recipient_id}"
            )))
        }
    }

    pub fn update_outbox_status(
        &self,
        content_id: &str,
        recipient_id: &str,
        status: DeliveryStatus,
    ) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE outbox SET status = ?1 WHERE content_id = ?2 AND recipient_id = ?3",
            params![status_str(status), content_id, recipient_id],
        )?;
        Ok(())
    }

    pub fn increment_outbox_retry(&self, content_id: &str, recipient_id: &str) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE outbox SET retry_count = retry_count + 1, status = 'pending'
             WHERE content_id = ?1 AND recipient_id = ?2",
            params![content_id, recipient_id],
        )?;
        Ok(())
    }

    pub fn insert_inbox(&self, entry: &InboxEntry) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO inbox
             (content_id, sender_id, received_at, expires_at, read_at, body, media_ref, content_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.content_id,
                entry.sender_id,
                entry.received_at.to_rfc3339(),
                entry.expires_at.to_rfc3339(),
                entry.read_at.map(|t| t.to_rfc3339()),
                entry.body,
                entry.media_ref,
                content_type_str(entry.content_type),
            ],
        )?;
        Ok(())
    }

    pub fn list_inbox(&self) -> CoreResult<Vec<InboxEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, sender_id, received_at, expires_at, read_at, body, media_ref, content_type
             FROM inbox ORDER BY received_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let content_type: String = row.get("content_type")?;
            Ok(InboxEntry {
                content_id: row.get("content_id")?,
                sender_id: row.get("sender_id")?,
                received_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("received_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                read_at: row
                    .get::<_, Option<String>>("read_at")?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                content_type: parse_content_type(&content_type),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn record_ack(&self, content_id: &str, recipient_id: &str) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO delivery_acks (content_id, recipient_id, acked_at)
             VALUES (?1, ?2, ?3)",
            params![content_id, recipient_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn insert_local_post(&self, post: &LocalPost) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO local_posts (content_id, body, media_ref, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                post.content_id,
                post.body,
                post.media_ref,
                post.created_at.to_rfc3339(),
                post.expires_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_local_posts(&self) -> CoreResult<Vec<LocalPost>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, body, media_ref, created_at, expires_at
             FROM local_posts ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(LocalPost {
                content_id: row.get("content_id")?,
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn list_inbox_posts(&self) -> CoreResult<Vec<InboxEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, sender_id, received_at, expires_at, read_at, body, media_ref, content_type
             FROM inbox WHERE content_type = 'post' ORDER BY received_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(InboxEntry {
                content_id: row.get("content_id")?,
                sender_id: row.get("sender_id")?,
                received_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("received_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                read_at: row
                    .get::<_, Option<String>>("read_at")?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                content_type: ContentType::Post,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn insert_profile_photo(&self, photo: &ProfilePhoto) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO profile_photos (id, blob_hash, caption, content_id, sort_order, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                photo.id,
                photo.blob_hash,
                photo.caption,
                photo.content_id,
                photo.sort_order,
                photo.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn update_profile_photo_content_id(
        &self,
        photo_id: &str,
        content_id: &str,
    ) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE profile_photos SET content_id = ?1 WHERE id = ?2",
            params![content_id, photo_id],
        )?;
        Ok(())
    }

    pub fn list_profile_photos(&self) -> CoreResult<Vec<ProfilePhoto>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, blob_hash, caption, content_id, sort_order, created_at
             FROM profile_photos ORDER BY sort_order, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProfilePhoto {
                id: row.get("id")?,
                blob_hash: row.get("blob_hash")?,
                caption: row.get("caption")?,
                content_id: row.get("content_id")?,
                sort_order: row.get("sort_order")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn insert_post_comment(&self, comment: &PostComment) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO post_comments
             (id, post_id, author_id, author_name, body, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                comment.id,
                comment.post_id,
                comment.author_id,
                comment.author_name,
                comment.body,
                comment.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_post_comments(&self, post_id: &str) -> CoreResult<Vec<PostComment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, post_id, author_id, author_name, body, created_at
             FROM post_comments WHERE post_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([post_id], |row| {
            Ok(PostComment {
                id: row.get("id")?,
                post_id: row.get("post_id")?,
                author_id: row.get("author_id")?,
                author_name: row.get("author_name")?,
                body: row.get("body")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn count_post_comments(&self, post_id: &str) -> CoreResult<u32> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM post_comments WHERE post_id = ?1",
            params![post_id],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }

    pub fn get_local_post(&self, content_id: &str) -> CoreResult<Option<LocalPost>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, body, media_ref, created_at, expires_at
             FROM local_posts WHERE content_id = ?1",
        )?;
        let mut rows = stmt.query([content_id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(LocalPost {
                content_id: row.get("content_id")?,
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            }));
        }
        Ok(None)
    }

    pub fn store_blob(&self, data: &[u8]) -> CoreResult<String> {
        let hash = encode_hex(Sha256::digest(data));
        let path = self.blob_path(&hash);
        if !path.exists() {
            std::fs::write(&path, data)?;
        }
        Ok(hash)
    }

    pub fn read_blob(&self, hash: &str) -> CoreResult<Vec<u8>> {
        let path = self.blob_path(hash);
        if !path.exists() {
            return Err(CoreError::ContentNotFound(hash.to_string()));
        }
        Ok(std::fs::read(&path)?)
    }

    pub fn get_settings(&self) -> CoreResult<AppSettings> {
        Ok(AppSettings {
            feed_history_enabled: self.get_bool_setting(FEED_HISTORY_KEY)?.unwrap_or(false),
            p2p_listen_port: self
                .get_string_setting(P2P_LISTEN_PORT_KEY)?
                .and_then(|s| s.parse().ok())
                .filter(|&port| port > 0)
                .unwrap_or(4784),
            relay_multiaddr: self
                .get_string_setting(RELAY_MULTIADDR_KEY)?
                .filter(|s| !s.trim().is_empty()),
            p2p_announce: self
                .get_string_setting(P2P_ANNOUNCE_KEY)?
                .filter(|s| !s.trim().is_empty()),
        })
    }

    pub fn set_feed_history_enabled(&self, enabled: bool) -> CoreResult<()> {
        self.set_string_setting(
            FEED_HISTORY_KEY,
            if enabled { "true" } else { "false" },
        )
    }

    pub fn update_connection_settings(
        &self,
        p2p_listen_port: Option<u16>,
        relay_multiaddr: Option<Option<String>>,
        p2p_announce: Option<Option<String>>,
    ) -> CoreResult<()> {
        if let Some(port) = p2p_listen_port.filter(|&p| p > 0) {
            self.set_string_setting(P2P_LISTEN_PORT_KEY, &port.to_string())?;
        }
        if let Some(relay) = relay_multiaddr {
            match relay.filter(|s| !s.trim().is_empty()) {
                Some(value) => self.set_string_setting(RELAY_MULTIADDR_KEY, value.trim())?,
                None => self.delete_setting(RELAY_MULTIADDR_KEY)?,
            }
        }
        if let Some(announce) = p2p_announce {
            match announce.filter(|s| !s.trim().is_empty()) {
                Some(value) => self.set_string_setting(P2P_ANNOUNCE_KEY, value.trim())?,
                None => self.delete_setting(P2P_ANNOUNCE_KEY)?,
            }
        }
        Ok(())
    }

    fn get_bool_setting(&self, key: &str) -> CoreResult<Option<bool>> {
        Ok(self
            .get_string_setting(key)?
            .map(|value| value == "true"))
    }

    fn get_string_setting(&self, key: &str) -> CoreResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get::<_, String>("value")?))
        } else {
            Ok(None)
        }
    }

    fn set_string_setting(&self, key: &str, value: &str) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    fn delete_setting(&self, key: &str) -> CoreResult<()> {
        self.conn.execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn upsert_feed_archive(&self, item: &ArchivedFeedItem) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO feed_archive
             (content_id, author_id, author_name, body, media_ref, created_at, is_own)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(content_id) DO UPDATE SET
               author_id = excluded.author_id,
               author_name = excluded.author_name,
               body = excluded.body,
               media_ref = excluded.media_ref,
               created_at = excluded.created_at,
               is_own = excluded.is_own",
            params![
                item.content_id,
                item.author_id,
                item.author_name,
                item.body,
                item.media_ref,
                item.created_at.to_rfc3339(),
                item.is_own as i32,
            ],
        )?;
        Ok(())
    }

    pub fn try_archive_feed_item(&self, item: &ArchivedFeedItem) -> CoreResult<()> {
        if self.get_settings()?.feed_history_enabled {
            self.upsert_feed_archive(item)?;
        }
        Ok(())
    }

    pub fn list_feed_archive(&self) -> CoreResult<Vec<ArchivedFeedItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, author_id, author_name, body, media_ref, created_at, is_own
             FROM feed_archive ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ArchivedFeedItem {
                content_id: row.get("content_id")?,
                author_id: row.get("author_id")?,
                author_name: row.get("author_name")?,
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                is_own: row.get::<_, i32>("is_own")? != 0,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn export_feed_backup(&self) -> CoreResult<FeedBackup> {
        let items = self.list_feed_archive()?;
        let mut blobs = HashMap::new();
        for item in &items {
            if let Some(ref hash) = item.media_ref {
                if blobs.contains_key(hash) {
                    continue;
                }
                if let Ok(data) = self.read_blob(hash) {
                    blobs.insert(hash.clone(), BASE64.encode(data));
                }
            }
        }
        Ok(FeedBackup {
            version: 1,
            exported_at: Utc::now(),
            items,
            blobs,
        })
    }

    pub fn import_feed_backup(&self, backup: &FeedBackup) -> CoreResult<FeedRestoreReport> {
        if backup.version != 1 {
            return Err(CoreError::Invite("unsupported backup version".into()));
        }

        let mut blobs_imported = 0usize;
        for (hash, b64) in &backup.blobs {
            let path = self.blob_path(hash);
            if path.exists() {
                continue;
            }
            let data = BASE64.decode(b64).map_err(|e| CoreError::Crypto(e.to_string()))?;
            std::fs::write(&path, data)?;
            blobs_imported += 1;
        }

        let mut items_imported = 0usize;
        for item in &backup.items {
            let existed = self.conn.query_row(
                "SELECT 1 FROM feed_archive WHERE content_id = ?1",
                params![item.content_id],
                |_| Ok(()),
            );
            if existed.is_ok() {
                continue;
            }
            self.upsert_feed_archive(item)?;
            items_imported += 1;
        }

        Ok(FeedRestoreReport {
            items_imported,
            blobs_imported,
        })
    }

    pub fn purge_expired(&self) -> CoreResult<PurgeReport> {
        let now = Utc::now().to_rfc3339();
        let outbox = self.conn.execute(
            "DELETE FROM outbox WHERE expires_at < ?1",
            params![now],
        )?;
        let inbox = self.conn.execute(
            "DELETE FROM inbox WHERE expires_at < ?1",
            params![now],
        )?;
        let local_posts = self.conn.execute(
            "DELETE FROM local_posts WHERE expires_at < ?1",
            params![now],
        )?;
        let invites = self.conn.execute(
            "DELETE FROM issued_invites WHERE expires_at < ?1 AND consumed_at IS NULL",
            params![now],
        )?;
        Ok(PurgeReport {
            outbox,
            inbox,
            local_posts,
            invites,
        })
    }

    pub fn register_issued_invite(&self, nonce: &str, expires_at: DateTime<Utc>) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO issued_invites (nonce, expires_at, consumed_at) VALUES (?1, ?2, NULL)",
            params![nonce, expires_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn consume_issued_invite(&self, nonce: &str) -> CoreResult<()> {
        let mut stmt = self
            .conn
            .prepare("SELECT expires_at, consumed_at FROM issued_invites WHERE nonce = ?1")?;
        let mut rows = stmt.query(params![nonce])?;
        let row = rows
            .next()?
            .ok_or_else(|| CoreError::Invite("unknown invite".into()))?;
        let expires_at: String = row.get("expires_at")?;
        let consumed_at: Option<String> = row.get("consumed_at")?;
        if consumed_at.is_some() {
            return Err(CoreError::Invite("invite already used".into()));
        }
        let expires = DateTime::parse_from_rfc3339(&expires_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| CoreError::Invite(e.to_string()))?;
        if Utc::now() > expires {
            return Err(CoreError::Invite("invite has expired".into()));
        }
        self.conn.execute(
            "UPDATE issued_invites SET consumed_at = ?1 WHERE nonce = ?2",
            params![Utc::now().to_rfc3339(), nonce],
        )?;
        Ok(())
    }

    pub fn is_invite_redeemed_locally(&self, nonce: &str) -> CoreResult<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM redeemed_invites WHERE nonce = ?1")?;
        let mut rows = stmt.query(params![nonce])?;
        Ok(rows.next()?.is_some())
    }

    pub fn mark_invite_redeemed_locally(
        &self,
        nonce: &str,
        issuer_signing_pubkey: &str,
    ) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO redeemed_invites (nonce, issuer_signing_pubkey, redeemed_at)
             VALUES (?1, ?2, ?3)",
            params![nonce, issuer_signing_pubkey, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn blob_path(&self, content_hash: &str) -> PathBuf {
        self.blob_dir.join(content_hash)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PurgeReport {
    pub outbox: usize,
    pub inbox: usize,
    pub local_posts: usize,
    pub invites: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;
    use chrono::Utc;

    #[test]
    fn round_trip_identity_and_contact() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let identity = Identity::generate("Alice");
        store.create_identity(&identity).unwrap();
        let err = store.create_identity(&Identity::generate("Bob")).unwrap_err();
        assert!(matches!(err, CoreError::ProfileAlreadyExists));
        let loaded = store.load_identity().unwrap().expect("identity round trip");
        assert_eq!(loaded.display_name, "Alice");
        assert_eq!(loaded.signing_pubkey, identity.signing_pubkey);
        assert!(loaded.sign(&b"hello"[..]).is_ok());

        let contact = Contact {
            id: "bob".into(),
            phone_hash: None,
            display_name: "Bob".into(),
            peer_id: None,
            signing_pubkey: "sign".into(),
            encryption_pubkey: "enc".into(),
            last_seen: None,
            connection_state: ConnectionState::Offline,
            multiaddrs: Vec::new(),
        };
        store.upsert_contact(&contact).unwrap();
        assert_eq!(store.list_contacts().unwrap().len(), 1);
    }

    #[test]
    fn invite_single_use() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let expires = Utc::now() + chrono::Duration::minutes(15);
        store.register_issued_invite("nonce-1", expires).unwrap();
        store.consume_issued_invite("nonce-1").unwrap();
        let err = store.consume_issued_invite("nonce-1").unwrap_err();
        assert!(err.to_string().contains("already used"));
    }
}
