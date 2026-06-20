use std::collections::VecDeque;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::content::DeliveryStatus;
use crate::p2p::P2pEvent;
use crate::store_handle::StoreHandle;

const MAX_EVENTS: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub struct P2pActivityEvent {
    pub at: DateTime<Utc>,
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct P2pActivitySnapshot {
    pub dial_in_progress: bool,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub events: Vec<P2pActivityEvent>,
}

#[derive(Debug)]
pub struct ActivityLog {
    dial_in_progress: bool,
    last_activity_at: Option<DateTime<Utc>>,
    events: VecDeque<P2pActivityEvent>,
}

impl ActivityLog {
    pub fn new() -> Self {
        Self {
            dial_in_progress: false,
            last_activity_at: None,
            events: VecDeque::new(),
        }
    }

    pub fn set_dial_in_progress(&mut self, in_progress: bool) {
        self.dial_in_progress = in_progress;
        if in_progress {
            self.push("dial", "Connecting to relay and friends…");
        }
    }

    pub fn push(&mut self, kind: impl Into<String>, detail: impl Into<String>) {
        let now = Utc::now();
        self.last_activity_at = Some(now);
        self.events.push_front(P2pActivityEvent {
            at: now,
            kind: kind.into(),
            detail: detail.into(),
        });
        while self.events.len() > MAX_EVENTS {
            self.events.pop_back();
        }
    }

    pub fn snapshot(&self) -> P2pActivitySnapshot {
        P2pActivitySnapshot {
            dial_in_progress: self.dial_in_progress,
            last_activity_at: self.last_activity_at,
            events: self.events.iter().cloned().collect(),
        }
    }
}

impl Default for ActivityLog {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn log_p2p_event(
    activity: &Arc<Mutex<ActivityLog>>,
    store: &StoreHandle,
    event: &P2pEvent,
) {
    let detail = match event {
        P2pEvent::PeerConnected(peer_id) => {
            format!("Connected to {}", contact_label(store, &peer_id.to_string()).await)
        }
        P2pEvent::PeerDisconnected(peer_id) => {
            format!("{} disconnected", contact_label(store, &peer_id.to_string()).await)
        }
        P2pEvent::MessageReceived { sender_id, body } => {
            let name = contact_label(store, sender_id).await;
            let preview: String = body.chars().take(40).collect();
            let suffix = if body.chars().count() > 40 { "…" } else { "" };
            format!("{name}: {preview}{suffix}")
        }
        P2pEvent::DeliveryAcked { content_id, peer_id } => {
            format!(
                "Delivered {} to {}",
                short_id(content_id),
                contact_label(store, &peer_id.to_string()).await
            )
        }
        P2pEvent::BlobNeeded { hash, peer_id } => {
            format!(
                "Fetching photo {} from {}",
                short_id(hash),
                contact_label(store, &peer_id.to_string()).await
            )
        }
        P2pEvent::FriendRequestReceived(req) => {
            format!("Friend request from {}", req.display_name)
        }
    };

    let kind = match event {
        P2pEvent::PeerConnected(_) => "peer_connected",
        P2pEvent::PeerDisconnected(_) => "peer_disconnected",
        P2pEvent::MessageReceived { .. } => "message_received",
        P2pEvent::DeliveryAcked { .. } => "delivery_acked",
        P2pEvent::BlobNeeded { .. } => "blob_sync",
        P2pEvent::FriendRequestReceived(_) => "friend_request",
    };

    activity.lock().await.push(kind, detail);
}

async fn contact_label(store: &StoreHandle, peer_or_signing_id: &str) -> String {
    store
        .with(|s| s.list_contacts())
        .await
        .ok()
        .and_then(|contacts| {
            contacts
                .into_iter()
                .find(|c| c.peer_id.as_deref() == Some(peer_or_signing_id) || c.id == peer_or_signing_id || c.signing_pubkey == peer_or_signing_id)
                .map(|c| c.display_name)
        })
        .unwrap_or_else(|| "friend".to_string())
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

pub async fn count_pending_outbox(store: &StoreHandle) -> usize {
    store
        .with(|s| s.list_outbox())
        .await
        .map(|entries| {
            entries
                .iter()
                .filter(|e| matches!(e.status, DeliveryStatus::Pending | DeliveryStatus::Failed))
                .count()
        })
        .unwrap_or(0)
}
