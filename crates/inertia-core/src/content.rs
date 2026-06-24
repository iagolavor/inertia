use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::identity::{MESSAGE_TTL_SECS, POST_TTL_SECS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Message,
    Post,
    Comment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryStatus {
    Pending,
    Sent,
    Failed,
    Delivered,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentEnvelope {
    pub id: String,
    pub author_signing_pubkey: String,
    pub author_encryption_pubkey: String,
    pub content_type: ContentType,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ciphertext: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    pub body: String,
    pub thread_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Photo,
    Video,
}

/// Chunked video metadata (512 KiB chunks). `root_hash` content-addresses the manifest body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaManifest {
    pub root_hash: String,
    pub kind: MediaKind,
    pub mime: String,
    pub total_bytes: u64,
    pub chunk_size: u32,
    pub chunk_hashes: Vec<String>,
    pub thumb_hash: String,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostPayload {
    pub body: String,
    pub media_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_kind: Option<MediaKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<MediaManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentPayload {
    pub post_id: String,
    pub body: String,
}

impl ContentEnvelope {
    pub fn new_message(
        author_signing_pubkey: String,
        author_encryption_pubkey: String,
        ciphertext: Vec<u8>,
        signature: Vec<u8>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            author_signing_pubkey,
            author_encryption_pubkey,
            content_type: ContentType::Message,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(MESSAGE_TTL_SECS),
            ciphertext,
            signature,
        }
    }

    pub fn new_post(
        author_signing_pubkey: String,
        author_encryption_pubkey: String,
        ciphertext: Vec<u8>,
        signature: Vec<u8>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            author_signing_pubkey,
            author_encryption_pubkey,
            content_type: ContentType::Post,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(POST_TTL_SECS),
            ciphertext,
            signature,
        }
    }

    pub fn new_comment(
        author_signing_pubkey: String,
        author_encryption_pubkey: String,
        ciphertext: Vec<u8>,
        signature: Vec<u8>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            author_signing_pubkey,
            author_encryption_pubkey,
            content_type: ContentType::Comment,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(POST_TTL_SECS),
            ciphertext,
            signature,
        }
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&SigningPayload {
            id: &self.id,
            author_signing_pubkey: &self.author_signing_pubkey,
            author_encryption_pubkey: &self.author_encryption_pubkey,
            content_type: self.content_type,
            created_at: self.created_at,
            expires_at: self.expires_at,
            ciphertext: &self.ciphertext,
        })
        .unwrap_or_default()
    }
}

#[derive(Serialize)]
struct SigningPayload<'a> {
    id: &'a str,
    author_signing_pubkey: &'a str,
    author_encryption_pubkey: &'a str,
    content_type: ContentType,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    ciphertext: &'a [u8],
}
