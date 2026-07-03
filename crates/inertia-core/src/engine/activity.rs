use std::collections::VecDeque;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::{broadcast, Mutex};

use crate::content::{ContentType, DeliveryStatus};
use crate::p2p::P2pEvent;
use crate::store_handle::StoreHandle;

const MAX_EVENTS: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub struct P2pActivityEvent {
    pub at: DateTime<Utc>,
    pub kind: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Real-time UI event pushed over SSE (activity log row + optional message payload).
#[derive(Debug, Clone, Serialize)]
pub struct P2pUiEvent {
    pub at: DateTime<Utc>,
    pub kind: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
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

    pub fn push(&mut self, kind: impl Into<String>, detail: impl Into<String>) -> P2pUiEvent {
        self.push_with_content_type(kind, detail, None)
    }

    pub fn push_with_content_type(
        &mut self,
        kind: impl Into<String>,
        detail: impl Into<String>,
        content_type: Option<String>,
    ) -> P2pUiEvent {
        let kind = kind.into();
        let detail = detail.into();
        let now = Utc::now();
        self.last_activity_at = Some(now);
        self.events.push_front(P2pActivityEvent {
            at: now,
            kind: kind.clone(),
            detail: detail.clone(),
            content_type: content_type.clone(),
        });
        while self.events.len() > MAX_EVENTS {
            self.events.pop_back();
        }
        P2pUiEvent {
            at: now,
            kind,
            detail,
            sender_id: None,
            contact_id: None,
            content_id: None,
            body: None,
            content_type,
            expires_at: None,
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

pub fn emit_ui_event(tx: &broadcast::Sender<P2pUiEvent>, event: P2pUiEvent) {
    let _ = tx.send(event);
}

pub async fn log_p2p_event(
    activity: &Arc<Mutex<ActivityLog>>,
    store: &StoreHandle,
    event: &P2pEvent,
    ui_event_tx: &broadcast::Sender<P2pUiEvent>,
) {
    let detail = match event {
        P2pEvent::PeerConnected(peer_id) => {
            format!("Connected to {}", contact_label(store, &peer_id.to_string()).await)
        }
        P2pEvent::PeerDisconnected(peer_id) => {
            format!("{} disconnected", contact_label(store, &peer_id.to_string()).await)
        }
        P2pEvent::MessageReceived {
            sender_id,
            body,
            ..
        } => {
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

    let ui_event = match event {
        P2pEvent::MessageReceived {
            sender_id,
            body,
            content_id,
            content_type,
            contact_id,
        } => {
            let expires_at = store
                .with(|s| s.list_inbox())
                .await
                .ok()
                .and_then(|inbox| {
                    inbox
                        .into_iter()
                        .find(|row| row.content_id == *content_id)
                        .map(|row| row.expires_at)
                });
            let content_type_str = content_type_to_str(*content_type).to_string();
            let mut log = activity.lock().await;
            let base = log.push_with_content_type(kind, detail, Some(content_type_str.clone()));
            P2pUiEvent {
                sender_id: Some(sender_id.clone()),
                contact_id: contact_id.clone(),
                content_id: Some(content_id.clone()),
                body: Some(body.clone()),
                content_type: Some(content_type_str),
                expires_at,
                ..base
            }
        }
        P2pEvent::DeliveryAcked {
            content_id,
            peer_id,
        } => {
            let contact_id =
                delivery_ack_contact_id(store, content_id, &peer_id.to_string()).await;
            let mut log = activity.lock().await;
            let base = log.push_with_content_type(kind, detail, Some("message".to_string()));
            P2pUiEvent {
                contact_id,
                content_id: Some(content_id.clone()),
                content_type: Some("message".to_string()),
                ..base
            }
        }
        _ => activity.lock().await.push(kind, detail),
    };
    emit_ui_event(ui_event_tx, ui_event);
}

fn content_type_to_str(content_type: ContentType) -> &'static str {
    match content_type {
        ContentType::Message => "message",
        ContentType::Post => "post",
        ContentType::Comment => "comment",
    }
}

async fn delivery_ack_contact_id(
    store: &StoreHandle,
    content_id: &str,
    peer_id: &str,
) -> Option<String> {
    let from_outbox = store
        .with(|s| s.list_outbox())
        .await
        .ok()
        .and_then(|entries| {
            entries
                .into_iter()
                .find(|entry| entry.content_id == content_id)
                .map(|entry| entry.recipient_id)
        });
    if from_outbox.is_some() {
        return from_outbox;
    }
    contact_id_for_peer(store, peer_id).await
}

async fn contact_id_for_peer(store: &StoreHandle, peer_id: &str) -> Option<String> {
    store
        .with(|s| s.list_contacts())
        .await
        .ok()
        .and_then(|contacts| {
            contacts
                .into_iter()
                .find(|contact| contact.peer_id.as_deref() == Some(peer_id))
                .map(|contact| contact.id)
        })
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
                .filter(|e| {
                    matches!(
                        e.status,
                        DeliveryStatus::Pending | DeliveryStatus::Sent | DeliveryStatus::Failed
                    )
                })
                .count()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::content::ContentType;
    use crate::p2p::P2pEvent;
    use crate::store_handle::StoreHandle;
    use tempfile::tempdir;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn log_p2p_event_broadcasts_message_payload() {
        let dir = tempdir().expect("tempdir");
        let store = StoreHandle::open(dir.path()).expect("open store");
        let activity = Arc::new(Mutex::new(ActivityLog::new()));
        let (tx, mut rx) = broadcast::channel(4);

        let event = P2pEvent::MessageReceived {
            sender_id: "signer-abc".into(),
            body: "hello there".into(),
            content_id: "content-123".into(),
            content_type: ContentType::Message,
            contact_id: Some("contact-456".into()),
        };

        log_p2p_event(&activity, &store, &event, &tx).await;

        let ui = rx.recv().await.expect("ui event");
        assert_eq!(ui.kind, "message_received");
        assert_eq!(ui.sender_id.as_deref(), Some("signer-abc"));
        assert_eq!(ui.contact_id.as_deref(), Some("contact-456"));
        assert_eq!(ui.content_id.as_deref(), Some("content-123"));
        assert_eq!(ui.body.as_deref(), Some("hello there"));
        assert_eq!(ui.content_type.as_deref(), Some("message"));
    }

    #[tokio::test]
    async fn log_p2p_event_broadcasts_delivery_ack_payload() {
        let dir = tempdir().expect("tempdir");
        let store = StoreHandle::open(dir.path()).expect("open store");
        let activity = Arc::new(Mutex::new(ActivityLog::new()));
        let (tx, mut rx) = broadcast::channel(4);

        let event = P2pEvent::DeliveryAcked {
            content_id: "content-789".into(),
            peer_id: libp2p::PeerId::random(),
        };

        log_p2p_event(&activity, &store, &event, &tx).await;

        let ui = rx.recv().await.expect("ui event");
        assert_eq!(ui.kind, "delivery_acked");
        assert_eq!(ui.content_id.as_deref(), Some("content-789"));
        assert_eq!(ui.content_type.as_deref(), Some("message"));
    }

    #[test]
    fn activity_push_returns_ui_event_without_payload() {
        let mut log = ActivityLog::new();
        let event = log.push("dial", "Connecting…");
        assert_eq!(event.kind, "dial");
        assert!(event.sender_id.is_none());
        assert!(event.content_id.is_none());
    }
}
