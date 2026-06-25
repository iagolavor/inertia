use serde::{Deserialize, Serialize};

use crate::content::ContentEnvelope;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequest {
    pub display_name: String,
    pub phone_hash: Option<String>,
    pub signing_pubkey: String,
    pub encryption_pubkey: String,
    pub peer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendAccept {
    pub contact_id: String,
    pub display_name: String,
    pub phone_hash: Option<String>,
    pub signing_pubkey: String,
    pub encryption_pubkey: String,
    pub peer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteRedemption {
    pub invite_nonce: String,
    pub display_name: String,
    pub signing_pubkey: String,
    pub encryption_pubkey: String,
    pub peer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEnvelope {
    pub envelope: ContentEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryAck {
    pub content_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobRequest {
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobData {
    pub hash: String,
    /// Raw image bytes (JPEG/PNG/WebP). Serialized as base64 in JSON.
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobChunkRequest {
    pub root_hash: String,
    pub chunk_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobChunkData {
    pub root_hash: String,
    pub chunk_index: u32,
    pub data: Vec<u8>,
}

/// Reserved for friend-graph multi-source seeding (Phase C).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobHave {
    pub root_hash: String,
    pub chunk_bitmap: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InertiaRequest {
    FriendRequest(FriendRequest),
    FriendAccept(FriendAccept),
    InviteRedemption(InviteRedemption),
    SendEnvelope(SendEnvelope),
    BlobRequest(BlobRequest),
    /// Sender pushes blob bytes after envelope delivery ack.
    BlobPush(BlobData),
    BlobChunkRequest(BlobChunkRequest),
    /// Reserved — advertise held chunks to friends (Phase C).
    BlobHave(BlobHave),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InertiaResponse {
    Ok,
    FriendAccept(FriendAccept),
    DeliveryAck(DeliveryAck),
    BlobData(BlobData),
    BlobNotFound,
    BlobChunkData(BlobChunkData),
    BlobChunkNotFound,
    Error(String),
}

pub const PROTOCOL_NAME: &str = "/inertia/1.0.0";
