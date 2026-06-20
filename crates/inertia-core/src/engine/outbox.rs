use std::sync::Arc;

use libp2p::PeerId;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::content::{ContentEnvelope, DeliveryStatus};
use crate::error::{CoreError, CoreResult};
use crate::p2p::{P2pEvent, P2pNode};
use crate::store_handle::StoreHandle;

pub async fn run_p2p_event_loop(
    mut events: mpsc::UnboundedReceiver<P2pEvent>,
    store: StoreHandle,
    p2p: Arc<Mutex<Option<P2pNode>>>,
) {
    while let Some(event) = events.recv().await {
        if let P2pEvent::PeerConnected(peer_id) = event {
            info!(%peer_id, "peer connected — flushing pending outbox");
            if let Err(e) = flush_outbox_for_peer(&store, &p2p, peer_id).await {
                warn!(error = %e, "outbox flush on peer connect failed");
            }
            if let Err(e) = super::blobs::request_missing_blobs_for_peer(&store, &p2p, peer_id).await
            {
                warn!(error = %e, "blob sync on peer connect failed");
            }
            continue;
        }

        super::blobs::handle_p2p_event(event, &store, &p2p).await;
    }
}

async fn flush_outbox_for_peer(
    store: &StoreHandle,
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    peer_id: PeerId,
) -> CoreResult<()> {
    let peer_id_str = peer_id.to_string();
    let contacts = store.with(|s| s.list_contacts()).await?;
    let recipient_ids: Vec<String> = contacts
        .iter()
        .filter(|c| c.peer_id.as_deref() == Some(peer_id_str.as_str()))
        .map(|c| c.id.clone())
        .collect();

    if recipient_ids.is_empty() {
        return Ok(());
    }

    let entries = store.with(|s| s.list_outbox()).await?;
    for entry in entries {
        if !recipient_ids.contains(&entry.recipient_id) {
            continue;
        }
        if !matches!(
            entry.status,
            DeliveryStatus::Pending | DeliveryStatus::Failed
        ) {
            continue;
        }
        if let Err(e) = deliver_outbox_entry(
            store,
            p2p,
            &entry.content_id,
            &entry.recipient_id,
            false,
        )
        .await
        {
            warn!(
                content_id = %entry.content_id,
                recipient_id = %entry.recipient_id,
                error = %e,
                "auto outbox delivery failed"
            );
        }
    }
    Ok(())
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
        Ok(()) => Ok(()),
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
        deliver_outbox_entry(
            &self.store,
            &self.p2p,
            content_id,
            recipient_id,
            true,
        )
        .await
    }
}
