use std::path::{Path, PathBuf};

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
    pub sort_order: i32,
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
    }
}

fn parse_content_type(s: &str) -> ContentType {
    match s {
        "post" => ContentType::Post,
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

pub struct Store {
    conn: Connection,
    blob_dir: PathBuf,
}

impl Store {
    pub fn open(data_dir: impl AsRef<Path>) -> CoreResult<Self> {
        let data_dir = data_dir.as_ref();
        std::fs::create_dir_all(data_dir)?;
        let blob_dir = data_dir.join("blobs");
        std::fs::create_dir_all(&blob_dir)?;

        let conn = Connection::open(data_dir.join("inertia.db"))?;
        let store = Self { conn, blob_dir };
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
        self.ensure_inbox_media_ref_column()?;
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
                 (id, signing_pubkey, encryption_pubkey, phone_hash, display_name, signing_key, encryption_secret)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    identity.signing_pubkey,
                    identity.encryption_pubkey,
                    identity.phone_hash,
                    identity.display_name,
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
            "SELECT signing_pubkey, encryption_pubkey, phone_hash, display_name, signing_key, encryption_secret
             FROM identity WHERE id = 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let identity = Identity::from_persisted(
                row.get("signing_pubkey")?,
                row.get("encryption_pubkey")?,
                row.get("phone_hash")?,
                row.get("display_name")?,
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
        self.conn.execute(
            "INSERT OR REPLACE INTO contacts
             (id, phone_hash, display_name, peer_id, signing_pubkey, encryption_pubkey, last_seen, connection_state)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                contact.id,
                contact.phone_hash,
                contact.display_name,
                contact.peer_id,
                contact.signing_pubkey,
                contact.encryption_pubkey,
                contact.last_seen.map(|t| t.to_rfc3339()),
                connection_state_str(contact.connection_state),
            ],
        )?;
        Ok(())
    }

    pub fn list_contacts(&self) -> CoreResult<Vec<Contact>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, phone_hash, display_name, peer_id, signing_pubkey, encryption_pubkey, last_seen, connection_state
             FROM contacts ORDER BY display_name",
        )?;
        let rows = stmt.query_map([], |row| {
            let state_str: String = row.get("connection_state")?;
            let connection_state = parse_connection_state(&state_str);
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
            "INSERT INTO profile_photos (id, blob_hash, caption, sort_order, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                photo.id,
                photo.blob_hash,
                photo.caption,
                photo.sort_order,
                photo.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_profile_photos(&self) -> CoreResult<Vec<ProfilePhoto>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, blob_hash, caption, sort_order, created_at
             FROM profile_photos ORDER BY sort_order, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProfilePhoto {
                id: row.get("id")?,
                blob_hash: row.get("blob_hash")?,
                caption: row.get("caption")?,
                sort_order: row.get("sort_order")?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
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
