use serde::Deserialize;

#[derive(Deserialize)]
pub struct InitIdentityRequest {
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: String,
    pub bio: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateInviteRequest {
    pub web_origin: Option<String>,
}

#[derive(Deserialize)]
pub struct InviteInput {
    pub invite: String,
}

#[derive(Deserialize)]
pub struct AddContactRequest {
    pub id: String,
    pub display_name: String,
    pub signing_pubkey: String,
    pub encryption_pubkey: String,
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub recipient_id: String,
    pub body: String,
}

#[derive(Deserialize)]
pub struct DialRequest {
    pub multiaddr: String,
}

#[derive(Deserialize)]
pub struct FriendRequestBody {
    pub contact_id: String,
    pub multiaddr: String,
}

#[derive(Deserialize)]
pub struct RetryOutboxRequest {
    pub content_id: String,
    pub recipient_id: String,
}

#[derive(Deserialize)]
pub struct StartP2pRequest {
    pub listen_port: Option<u16>,
}

#[derive(Deserialize)]
pub struct CreatePostRequest {
    pub body: String,
    pub media_base64: Option<String>,
}

#[derive(Deserialize)]
pub struct AddCommentRequest {
    pub body: String,
}

#[derive(Deserialize)]
pub struct UploadPhotoRequest {
    pub data_base64: String,
    pub caption: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub feed_history_enabled: Option<bool>,
    #[serde(default)]
    pub p2p_listen_port: Option<u16>,
    #[serde(default)]
    pub relay_multiaddr: Option<String>,
    #[serde(default)]
    pub p2p_announce: Option<String>,
    #[serde(default)]
    pub web_origin: Option<String>,
}
