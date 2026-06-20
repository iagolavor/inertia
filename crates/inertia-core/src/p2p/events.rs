use libp2p::PeerId;

use super::protocol::FriendRequest;

#[derive(Debug, Clone)]
pub enum P2pEvent {
    FriendRequestReceived(FriendRequest),
    MessageReceived { sender_id: String, body: String },
    DeliveryAcked {
        content_id: String,
        peer_id: PeerId,
    },
    BlobNeeded {
        hash: String,
        peer_id: PeerId,
    },
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
}
