use libp2p::PeerId;

use crate::content::ContentType;

use super::protocol::FriendRequest;

#[derive(Debug, Clone)]
pub enum P2pEvent {
    FriendRequestReceived(FriendRequest),
    MessageReceived {
        sender_id: String,
        body: String,
        content_id: String,
        content_type: ContentType,
        contact_id: Option<String>,
    },
    DeliveryAcked {
        content_id: String,
        peer_id: PeerId,
    },
    BlobNeeded {
        hash: String,
        peer_id: PeerId,
    },
    CommentReceived {
        post_id: String,
        content_id: String,
        author_id: String,
        body: String,
    },
    ProfileCommentReceived {
        profile_item_id: String,
        content_id: String,
        author_id: String,
        body: String,
    },
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
}
