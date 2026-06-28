mod blobs;
mod contacts;
mod feed;
mod identity;
mod inbox;
mod invites;
mod media;
mod outbox;
mod purge;
mod schema;
mod settings;
mod sql;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::content::{ContentType, DeliveryStatus, MediaKind};
use crate::error::CoreResult;

pub use blobs::MAX_BLOB_BYTES;
pub use media::{CHUNK_SIZE, MAX_THUMB_BYTES, MAX_VIDEO_BYTES};

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
    Reachable,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_kind: Option<MediaKind>,
    #[serde(default)]
    pub media_ready: bool,
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
    pub relay_multiaddrs: Vec<String>,
    pub p2p_announce: Option<String>,
    pub web_origin: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentMessage {
    pub content_id: String,
    pub recipient_id: String,
    pub body: String,
    pub sent_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: DeliveryStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurgeReport {
    pub outbox: usize,
    pub inbox: usize,
    pub local_posts: usize,
    pub invites: usize,
    pub sent_messages: usize,
}

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
            thumb_ref: None,
            media_kind: None,
            media_ready: false,
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

    pub fn blob_path(&self, content_hash: &str) -> PathBuf {
        self.blob_dir.join(content_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CoreError;
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
