use std::sync::Arc;

use libp2p::PeerId;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{info, warn};

use crate::content::{ContentEnvelope, ContentType, DeliveryStatus};
use crate::error::{CoreError, CoreResult};
use crate::p2p::{P2pEvent, P2pNode};
use crate::store_handle::StoreHandle;

use super::activity::{self, ActivityLog, P2pStatusRelayHints, P2pUiEvent};

pub async fn run_p2p_event_loop(
    mut events: mpsc::UnboundedReceiver<P2pEvent>,
    store: StoreHandle,
    p2p: Arc<Mutex<Option<P2pNode>>>,
    activity: Arc<Mutex<ActivityLog>>,
    relay_hints: Arc<Mutex<P2pStatusRelayHints>>,
    ui_event_tx: broadcast::Sender<P2pUiEvent>,
) {
    while let Some(event) = events.recv().await {
        let emit_status_after = matches!(
            event,
            P2pEvent::PeerConnected(_) | P2pEvent::PeerDisconnected(_)
        );

        activity::log_p2p_event(&activity, &store, &event, &ui_event_tx).await;

        if let P2pEvent::PeerConnected(peer_id) = event {
            info!(%peer_id, "peer connected — flushing pending outbox");
            let flushed = flush_outbox_for_peer(
                &store,
                &p2p,
                &activity,
                &relay_hints,
                &ui_event_tx,
                peer_id,
            )
            .await;
            match flushed {
                Ok(count) if count > 0 => {
                    let ui_event = activity
                        .lock()
                        .await
                        .push("outbox_flush", format!("Sent {count} pending item(s)"));
                    activity::emit_ui_event(&ui_event_tx, ui_event);
                }
                Err(e) => warn!(error = %e, "outbox flush on peer connect failed"),
                _ => {}
            }
            if let Err(e) = super::blobs::request_missing_blobs_for_peer(&store, &p2p, peer_id).await
            {
                warn!(error = %e, "blob sync on peer connect failed");
            }
            activity::emit_p2p_status_changed(
                &store,
                &p2p,
                &activity,
                &relay_hints,
                &ui_event_tx,
            )
            .await;
            continue;
        }

        super::blobs::handle_p2p_event(event, &store, &p2p).await;

        if emit_status_after {
            activity::emit_p2p_status_changed(
                &store,
                &p2p,
                &activity,
                &relay_hints,
                &ui_event_tx,
            )
            .await;
        }
    }
}

async fn flush_outbox_for_peer(
    store: &StoreHandle,
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    activity: &Arc<Mutex<ActivityLog>>,
    _relay_hints: &Arc<Mutex<P2pStatusRelayHints>>,
    ui_event_tx: &broadcast::Sender<P2pUiEvent>,
    peer_id: PeerId,
) -> CoreResult<usize> {
    let peer_id_str = peer_id.to_string();
    let contacts = store.with(|s| s.list_contacts()).await?;
    let recipient_ids: Vec<String> = contacts
        .iter()
        .filter(|c| c.peer_id.as_deref() == Some(peer_id_str.as_str()))
        .map(|c| c.id.clone())
        .collect();

    if recipient_ids.is_empty() {
        return Ok(0);
    }

    let entries = store.with(|s| s.list_outbox()).await?;
    let mut sent = 0usize;
    for entry in entries {
        if !recipient_ids.contains(&entry.recipient_id) {
            continue;
        }
        if !matches!(
            entry.status,
            DeliveryStatus::Pending | DeliveryStatus::Failed | DeliveryStatus::Sent
        ) {
            continue;
        }
        if deliver_outbox_entry(
            store,
            p2p,
            &entry.content_id,
            &entry.recipient_id,
            false,
        )
        .await
        .is_ok()
        {
            activity::emit_message_sent_ui_event(
                activity,
                store,
                ui_event_tx,
                &entry.content_id,
                &entry.recipient_id,
                entry.content_type,
            )
            .await;
            sent += 1;
        } else {
            warn!(
                content_id = %entry.content_id,
                recipient_id = %entry.recipient_id,
                "auto outbox delivery failed"
            );
        }
    }
    Ok(sent)
}

pub async fn deliver_outbox_entry(
    store: &StoreHandle,
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    content_id: &str,
    recipient_id: &str,
    increment_retry: bool,
) -> CoreResult<()> {
    let envelope_json = store
        .with(|s| s.get_outbox_envelope(content_id, recipient_id))
        .await?;
    let recipient = store.with(|s| s.get_contact(recipient_id)).await?;
    let envelope: ContentEnvelope = serde_json::from_str(&envelope_json)?;
    let peer_id = recipient
        .peer_id
        .as_ref()
        .ok_or_else(|| CoreError::P2p("recipient has no peer id".into()))?
        .parse::<PeerId>()
        .map_err(|e| CoreError::P2p(e.to_string()))?;

    if increment_retry {
        store
            .with_mut(|s| s.increment_outbox_retry(content_id, recipient_id))
            .await?;
    }

    let guard = p2p.lock().await;
    let node = guard
        .as_ref()
        .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;

    match node.send_envelope_to_peer(peer_id, envelope).await {
        Ok(()) => {
            store
                .with_mut(|s| {
                    s.update_outbox_status(content_id, recipient_id, DeliveryStatus::Sent)
                })
                .await?;
            Ok(())
        }
        Err(e) => {
            store
                .with_mut(|s| {
                    s.update_outbox_status(content_id, recipient_id, DeliveryStatus::Failed)
                })
                .await?;
            Err(e)
        }
    }
}

use super::Engine;

impl Engine {
    pub async fn retry_outbox(&self, content_id: &str, recipient_id: &str) -> CoreResult<()> {
        let content_type = self
            .store
            .with(|s| s.list_outbox())
            .await?
            .into_iter()
            .find(|entry| entry.content_id == content_id && entry.recipient_id == recipient_id)
            .map(|entry| entry.content_type)
            .unwrap_or(ContentType::Message);

        deliver_outbox_entry(
            &self.store,
            &self.p2p,
            content_id,
            recipient_id,
            true,
        )
        .await?;

        activity::emit_message_sent_ui_event(
            &self.activity,
            &self.store,
            &self.ui_event_tx,
            content_id,
            recipient_id,
            content_type,
        )
        .await;
        activity::emit_p2p_status_changed(
            &self.store,
            &self.p2p,
            &self.activity,
            &self.relay_status_hints,
            &self.ui_event_tx,
        )
        .await;
        Ok(())
    }

    pub(crate) async fn emit_message_sent_ui(
        &self,
        content_id: &str,
        recipient_id: &str,
        content_type: ContentType,
    ) {
        activity::emit_message_sent_ui_event(
            &self.activity,
            &self.store,
            &self.ui_event_tx,
            content_id,
            recipient_id,
            content_type,
        )
        .await;
        activity::emit_p2p_status_changed(
            &self.store,
            &self.p2p,
            &self.activity,
            &self.relay_status_hints,
            &self.ui_event_tx,
        )
        .await;
    }

    pub(crate) async fn emit_p2p_status_changed(&self) {
        activity::emit_p2p_status_changed(
            &self.store,
            &self.p2p,
            &self.activity,
            &self.relay_status_hints,
            &self.ui_event_tx,
        )
        .await;
    }
}
