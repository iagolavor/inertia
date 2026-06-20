use std::sync::Arc;

use libp2p::PeerId;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::error::CoreResult;
use crate::p2p::{P2pEvent, P2pNode};
use crate::store_handle::StoreHandle;

pub async fn handle_p2p_event(
    event: P2pEvent,
    store: &StoreHandle,
    p2p: &Arc<Mutex<Option<P2pNode>>>,
) {
    match event {
        P2pEvent::DeliveryAcked {
            content_id,
            peer_id,
        } => {
            if let Err(e) = push_blob_after_ack(store, p2p, &content_id, peer_id).await {
                warn!(
                    content_id = %content_id,
                    %peer_id,
                    error = %e,
                    "blob push after delivery ack failed"
                );
            }
        }
        P2pEvent::BlobNeeded { hash, peer_id } => {
            if let Err(e) = request_blob_from_peer(store, p2p, peer_id, &hash).await {
                warn!(%hash, %peer_id, error = %e, "blob pull failed");
            }
        }
        _ => {}
    }
}

pub async fn request_missing_blobs_for_peer(
    store: &StoreHandle,
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    peer_id: PeerId,
) -> CoreResult<()> {
    let peer_id_str = peer_id.to_string();
    let author_key = store
        .with(|s| {
            Ok(s.list_contacts()?
                .into_iter()
                .find(|c| c.peer_id.as_deref() == Some(peer_id_str.as_str()))
                .map(|c| c.signing_pubkey))
        })
        .await?;

    let Some(author_key) = author_key else {
        return Ok(());
    };

    let hashes = store
        .with(|s| s.missing_media_refs_for_author(&author_key))
        .await?;

    for hash in hashes {
        if let Err(e) = request_blob_from_peer(store, p2p, peer_id, &hash).await {
            warn!(%hash, %peer_id, error = %e, "blob pull on connect failed");
        }
    }
    Ok(())
}

async fn push_blob_after_ack(
    store: &StoreHandle,
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    content_id: &str,
    peer_id: PeerId,
) -> CoreResult<()> {
    let media_ref = store
        .with(|s| Ok(s.get_local_post(content_id)?.and_then(|post| post.media_ref)))
        .await?;

    let Some(hash) = media_ref else {
        return Ok(());
    };

    let data = store.with(|s| s.read_blob(&hash)).await?;
    let guard = p2p.lock().await;
    let node = guard
        .as_ref()
        .ok_or_else(|| crate::error::CoreError::P2p("p2p not started".into()))?;
    node.push_blob_to_peer(peer_id, &hash, &data).await?;
    info!(%content_id, %hash, %peer_id, bytes = data.len(), "pushed post blob after ack");
    Ok(())
}

async fn request_blob_from_peer(
    store: &StoreHandle,
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    peer_id: PeerId,
    hash: &str,
) -> CoreResult<()> {
    if store.with(|s| Ok(s.blob_exists(hash))).await? {
        return Ok(());
    }

    let guard = p2p.lock().await;
    let node = guard
        .as_ref()
        .ok_or_else(|| crate::error::CoreError::P2p("p2p not started".into()))?;
    node.request_blob_from_peer(peer_id, hash).await?;
    info!(%hash, %peer_id, "fetched missing blob from peer");
    Ok(())
}
