use std::collections::VecDeque;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::{broadcast, Mutex};

use crate::content::{ContentType, DeliveryStatus};
use crate::p2p::{P2pEvent, P2pNode};
use crate::store_handle::StoreHandle;

use super::p2p_status::{self, P2pLayerLabels, P2pLayers};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<String>,
    /// Populated on `p2p_status_changed` events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_peer_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_outbox_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dial_in_progress: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<P2pLayers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<P2pLayerLabels>,
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
            post_id: None,
            connected_peer_ids: None,
            tone: None,
            pending_outbox_count: None,
            dial_in_progress: None,
            layers: None,
            labels: None,
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
        P2pEvent::CommentReceived { body, .. } => {
            let preview: String = body.chars().take(40).collect();
            let suffix = if body.chars().count() > 40 { "…" } else { "" };
            format!("New comment: {preview}{suffix}")
        }
    };

    let kind = match event {
        P2pEvent::PeerConnected(_) => "peer_connected",
        P2pEvent::PeerDisconnected(_) => "peer_disconnected",
        P2pEvent::MessageReceived { .. } => "message_received",
        P2pEvent::DeliveryAcked { .. } => "delivery_acked",
        P2pEvent::BlobNeeded { .. } => "blob_sync",
        P2pEvent::FriendRequestReceived(_) => "friend_request",
        P2pEvent::CommentReceived { .. } => "comment_received",
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
        P2pEvent::CommentReceived {
            post_id,
            content_id,
            author_id,
            body,
        } => {
            let mut log = activity.lock().await;
            let base = log.push_with_content_type(kind, detail, Some("comment".to_string()));
            P2pUiEvent {
                sender_id: Some(author_id.clone()),
                content_id: Some(content_id.clone()),
                post_id: Some(post_id.clone()),
                body: Some(body.clone()),
                content_type: Some("comment".to_string()),
                ..base
            }
        }
        P2pEvent::FriendRequestReceived(req) => {
            let mut log = activity.lock().await;
            let base = log.push(kind, detail);
            P2pUiEvent {
                sender_id: Some(req.signing_pubkey.clone()),
                body: Some(req.display_name.clone()),
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

/// Cached relay probe fields from the last full `/p2p/status` snapshot.
#[derive(Debug, Clone, Default)]
pub struct P2pStatusRelayHints {
    pub relay_configured: bool,
    pub relay_peer_ids: Vec<String>,
    pub relay_tcp_reachable: Option<bool>,
}

pub async fn emit_message_sent_ui_event(
    activity: &Arc<Mutex<ActivityLog>>,
    store: &StoreHandle,
    ui_event_tx: &broadcast::Sender<P2pUiEvent>,
    content_id: &str,
    contact_id: &str,
    content_type: ContentType,
) {
    let name = contact_label(store, contact_id).await;
    let content_type_str = content_type_to_str(content_type).to_string();
    let detail = format!(
        "Sent {} to {}",
        short_id(content_id),
        name
    );
    let mut log = activity.lock().await;
    let base = log.push_with_content_type("message_sent", detail, Some(content_type_str.clone()));
    let ui_event = P2pUiEvent {
        contact_id: Some(contact_id.to_string()),
        content_id: Some(content_id.to_string()),
        content_type: Some(content_type_str),
        ..base
    };
    emit_ui_event(ui_event_tx, ui_event);
}

pub async fn emit_p2p_status_changed(
    store: &StoreHandle,
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    activity: &Arc<Mutex<ActivityLog>>,
    relay_hints: &Arc<Mutex<P2pStatusRelayHints>>,
    ui_event_tx: &broadcast::Sender<P2pUiEvent>,
) {
    let relay_hints = relay_hints.lock().await;
    let pending_outbox_count = count_pending_outbox(store).await;
    let activity_snap = activity.lock().await.snapshot();

    let guard = p2p.lock().await;
    let status_core = if let Some(node) = guard.as_ref() {
        let connected_peer_ids = node.connected_peer_ids().await;
        let relay_connected = relay_hints.relay_peer_ids.iter().any(|id| {
            connected_peer_ids.iter().any(|peer| peer == id)
        });
        let friends_online_count = connected_peer_ids
            .iter()
            .filter(|id| !relay_hints.relay_peer_ids.iter().any(|relay_id| relay_id == *id))
            .count();
        (
            true,
            connected_peer_ids,
            relay_connected,
            friends_online_count,
        )
    } else {
        (false, Vec::new(), false, 0)
    };
    drop(guard);

    let (running, connected_peer_ids, relay_connected, friends_online_count) = status_core;

    let layers = p2p_status::build_layers(
        running,
        relay_hints.relay_configured,
        relay_hints.relay_tcp_reachable,
        relay_connected,
        friends_online_count,
        activity_snap.dial_in_progress,
        pending_outbox_count,
    );
    let labels = p2p_status::build_labels(&layers);
    let tone =
        p2p_status::visual_tone_str(p2p_status::visual_tone(&layers)).to_string();

    let mut log = activity.lock().await;
    let base = log.push("p2p_status_changed", labels.headline.clone());
    let ui_event = P2pUiEvent {
        connected_peer_ids: Some(connected_peer_ids),
        tone: Some(tone),
        pending_outbox_count: Some(pending_outbox_count),
        dial_in_progress: Some(activity_snap.dial_in_progress),
        layers: Some(layers),
        labels: Some(labels),
        ..base
    };
    emit_ui_event(ui_event_tx, ui_event);
}

pub async fn refresh_relay_hints_from_store(
    store: &StoreHandle,
    relay_hints: &mut P2pStatusRelayHints,
    relay_tcp_reachable: Option<bool>,
) {
    let relays = store
        .with(|s| s.get_settings())
        .await
        .ok()
        .map(|settings| settings.relay_multiaddrs)
        .unwrap_or_default();
    relay_hints.relay_configured = !relays.is_empty();
    relay_hints.relay_peer_ids = relays
        .iter()
        .filter_map(|relay| super::p2p::peer_id_from_multiaddr_str(relay))
        .collect();
    if let Some(reachable) = relay_tcp_reachable {
        relay_hints.relay_tcp_reachable = Some(reachable);
    }
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

    #[tokio::test]
    async fn emit_message_sent_ui_event_includes_payload() {
        let dir = tempdir().expect("tempdir");
        let store = StoreHandle::open(dir.path()).expect("open store");
        let activity = Arc::new(Mutex::new(ActivityLog::new()));
        let (tx, mut rx) = broadcast::channel(4);

        emit_message_sent_ui_event(
            &activity,
            &store,
            &tx,
            "content-abc",
            "contact-xyz",
            ContentType::Message,
        )
        .await;

        let ui = rx.recv().await.expect("ui event");
        assert_eq!(ui.kind, "message_sent");
        assert_eq!(ui.content_id.as_deref(), Some("content-abc"));
        assert_eq!(ui.contact_id.as_deref(), Some("contact-xyz"));
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
