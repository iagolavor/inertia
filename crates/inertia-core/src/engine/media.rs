use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use libp2p::PeerId;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::content::MediaManifest;
use crate::error::{CoreError, CoreResult};
use crate::p2p::P2pNode;

use super::Engine;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaFetchState {
    Idle,
    Fetching,
    Complete,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MediaFetchStatus {
    pub root_hash: String,
    pub state: MediaFetchState,
    pub chunks_done: u32,
    pub chunks_total: u32,
    pub transport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Engine {
    pub async fn media_fetch_status(&self, root_hash: &str) -> CoreResult<MediaFetchStatus> {
        let manifest = self
            .store
            .with(|s| s.get_manifest(root_hash))
            .await?
            .ok_or_else(|| CoreError::ContentNotFound(root_hash.to_string()))?;

        if self
            .store
            .with(|s| Ok(s.media_is_complete(&manifest)))
            .await?
        {
            return Ok(MediaFetchStatus {
                root_hash: root_hash.to_string(),
                state: MediaFetchState::Complete,
                chunks_done: manifest.chunk_hashes.len() as u32,
                chunks_total: manifest.chunk_hashes.len() as u32,
                transport: "local".into(),
                error: None,
            });
        }

        let jobs = self.media_fetches.lock().await;
        if let Some(status) = jobs.get(root_hash) {
            return Ok(status.clone());
        }

        Ok(MediaFetchStatus {
            root_hash: root_hash.to_string(),
            state: MediaFetchState::Idle,
            chunks_done: self.store.with(|s| Ok(s.count_local_chunks(&manifest))).await?,
            chunks_total: manifest.chunk_hashes.len() as u32,
            transport: "unknown".into(),
            error: None,
        })
    }

    pub async fn start_media_fetch(&self, root_hash: &str) -> CoreResult<MediaFetchStatus> {
        let manifest = self
            .store
            .with(|s| s.get_manifest(root_hash))
            .await?
            .ok_or_else(|| CoreError::ContentNotFound(root_hash.to_string()))?;

        if self
            .store
            .with(|s| Ok(s.media_is_complete(&manifest)))
            .await?
        {
            let _ = self
                .store
                .with(|s| s.assemble_media_if_complete(&manifest))
                .await?;
            return self.media_fetch_status(root_hash).await;
        }

        {
            let jobs = self.media_fetches.lock().await;
            if let Some(status) = jobs.get(root_hash) {
                if matches!(status.state, MediaFetchState::Fetching) {
                    return Ok(status.clone());
                }
            }
        }

        let author_peer = self.find_author_peer_for_media(root_hash).await?;
        let store = self.store.clone();
        let p2p = Arc::clone(&self.p2p);
        let media_fetches = Arc::clone(&self.media_fetches);
        let root = root_hash.to_string();
        let manifest_clone = manifest.clone();
        let manifest_for_err = manifest.clone();

        {
            let mut jobs = media_fetches.lock().await;
            jobs.insert(
                root.clone(),
                MediaFetchStatus {
                    root_hash: root.clone(),
                    state: MediaFetchState::Fetching,
                    chunks_done: store
                        .with(|s| Ok(s.count_local_chunks(&manifest_clone)))
                        .await
                        .unwrap_or(0),
                    chunks_total: manifest_clone.chunk_hashes.len() as u32,
                    transport: "unknown".into(),
                    error: None,
                },
            );
        }

        let store_err = store.clone();
        let fetches_err = Arc::clone(&media_fetches);

        tokio::spawn(async move {
            if let Err(e) =
                run_media_fetch(store, p2p, media_fetches, author_peer, manifest_clone).await
            {
                warn!(error = %e, %root, "media fetch failed");
                update_fetch(
                    &fetches_err,
                    &root,
                    MediaFetchState::Failed,
                    Some(e.to_string()),
                    None,
                    &store_err,
                    &manifest_for_err,
                )
                .await;
            }
        });

        self.media_fetch_status(root_hash).await
    }

    async fn find_author_peer_for_media(&self, root_hash: &str) -> CoreResult<PeerId> {
        let author_id = self.find_author_id_for_media(root_hash).await?;
        let contacts = self.store.with(|s| s.list_contacts()).await?;
        let contact = contacts
            .into_iter()
            .find(|c| c.id == author_id || c.signing_pubkey == author_id)
            .ok_or_else(|| CoreError::P2p("author contact not found".into()))?;
        let peer_id_str = contact
            .peer_id
            .ok_or_else(|| CoreError::P2p("author is offline".into()))?;
        peer_id_str
            .parse()
            .map_err(|_| CoreError::P2p("invalid author peer id".into()))
    }

    async fn find_author_id_for_media(&self, root_hash: &str) -> CoreResult<String> {
        let items = self.list_feed().await?;
        if let Some(item) = items.into_iter().find(|i| i.media_ref.as_deref() == Some(root_hash)) {
            return Ok(item.author_id);
        }
        Err(CoreError::ContentNotFound(root_hash.to_string()))
    }
}

async fn run_media_fetch(
    store: crate::store_handle::StoreHandle,
    p2p: Arc<Mutex<Option<P2pNode>>>,
    media_fetches: Arc<Mutex<HashMap<String, MediaFetchStatus>>>,
    author_peer: PeerId,
    manifest: MediaManifest,
) -> CoreResult<()> {
    let transport = {
        let guard = p2p.lock().await;
        let node = guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let _ = node
            .wait_for_direct(author_peer, Duration::from_secs(5))
            .await;
        if node.has_direct_connection(author_peer).await {
            "direct".to_string()
        } else {
            "relay".to_string()
        }
    };

    update_fetch(
        &media_fetches,
        &manifest.root_hash,
        MediaFetchState::Fetching,
        None,
        Some(transport.clone()),
        &store,
        &manifest,
    )
    .await;

    for (index, expected_hash) in manifest.chunk_hashes.iter().enumerate() {
        let index = index as u32;
        if store
            .with(|s| Ok(s.chunk_exists(&manifest.root_hash, index)))
            .await?
        {
            continue;
        }
        let guard = p2p.lock().await;
        let node = guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        node
            .request_chunk_from_peer(author_peer, &manifest.root_hash, index, expected_hash)
            .await?;
        update_fetch(
            &media_fetches,
            &manifest.root_hash,
            MediaFetchState::Fetching,
            None,
            Some(transport.clone()),
            &store,
            &manifest,
        )
        .await;
    }

    store
        .with(|s| s.assemble_media_if_complete(&manifest))
        .await?;

    update_fetch(
        &media_fetches,
        &manifest.root_hash,
        MediaFetchState::Complete,
        None,
        Some(transport),
        &store,
        &manifest,
    )
    .await;
    info!(root = %manifest.root_hash, "media fetch complete");
    Ok(())
}

async fn update_fetch(
    media_fetches: &Arc<Mutex<HashMap<String, MediaFetchStatus>>>,
    root_hash: &str,
    state: MediaFetchState,
    error: Option<String>,
    transport: Option<String>,
    store: &crate::store_handle::StoreHandle,
    manifest: &MediaManifest,
) {
    let chunks_done = store
        .with(|s| Ok(s.count_local_chunks(manifest)))
        .await
        .unwrap_or(0);
    let mut jobs = media_fetches.lock().await;
    jobs.insert(
        root_hash.to_string(),
        MediaFetchStatus {
            root_hash: root_hash.to_string(),
            state,
            chunks_done,
            chunks_total: manifest.chunk_hashes.len() as u32,
            transport: transport.unwrap_or_else(|| "unknown".into()),
            error,
        },
    );
}
