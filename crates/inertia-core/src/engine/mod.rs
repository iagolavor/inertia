mod backup;
mod contacts;
mod feed;
mod identity;
mod invite;
mod messaging;
mod outbox;
mod p2p;
mod profile;
mod settings;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex, RwLock};

use crate::content::DeliveryStatus;
use crate::error::CoreResult;
use crate::expiry::ExpiryScheduler;
use crate::identity::Identity;
use crate::p2p::{P2pEvent, P2pNode};
use crate::storage::ProfilePhoto;
use crate::store_handle::StoreHandle;

pub use outbox::deliver_outbox_entry;

/// Default libp2p TCP listen port when `INERTIA_P2P_LISTEN_PORT` is unset.
pub const DEFAULT_P2P_LISTEN_PORT: u16 = 4784;

/// High-level facade over storage, identity, expiry, and P2P networking.
pub struct Engine {
    pub store: StoreHandle,
    pub identity: Arc<RwLock<Identity>>,
    pub(crate) p2p: Arc<Mutex<Option<P2pNode>>>,
    _expiry_handle: Option<tokio::task::JoinHandle<()>>,
    _p2p_event_task: tokio::task::JoinHandle<()>,
    pub(crate) event_tx: mpsc::UnboundedSender<P2pEvent>,
}

impl Engine {
    pub async fn open(data_dir: impl AsRef<Path>) -> CoreResult<Self> {
        let store = StoreHandle::open(data_dir)?;
        let identity = match store.with(|s| s.load_identity()).await? {
            Some(loaded) => Arc::new(RwLock::new(loaded)),
            None => Arc::new(RwLock::new(Identity::generate(""))),
        };

        let expiry = ExpiryScheduler::new(store.clone(), Duration::from_secs(300));
        let expiry_handle = Some(expiry.spawn());

        let (event_tx, p2p_events) = mpsc::unbounded_channel();
        let p2p = Arc::new(Mutex::new(None));
        let p2p_for_events = Arc::clone(&p2p);
        let store_for_events = store.clone();
        let p2p_event_task = tokio::spawn(async move {
            outbox::run_p2p_event_loop(p2p_events, store_for_events, p2p_for_events).await;
        });

        let engine = Self {
            store,
            identity,
            p2p,
            _expiry_handle: expiry_handle,
            _p2p_event_task: p2p_event_task,
            event_tx,
        };

        if engine.identity.read().await.is_initialized() {
            engine.ensure_p2p_started().await?;
        }

        Ok(engine)
    }

    pub async fn list_outbox(&self) -> CoreResult<Vec<crate::storage::OutboxEntry>> {
        self.store.with(|store| store.list_outbox()).await
    }

    pub async fn list_inbox(&self) -> CoreResult<Vec<crate::storage::InboxEntry>> {
        self.store.with(|store| store.list_inbox()).await
    }

    pub async fn run_expiry_sweep(&self) -> CoreResult<crate::storage::PurgeReport> {
        self.store.with(|store| store.purge_expired()).await
    }
}

pub fn default_p2p_listen_port() -> u16 {
    p2p_listen_port_from_env().unwrap_or(DEFAULT_P2P_LISTEN_PORT)
}

pub async fn probe_relay_tcp(multiaddr: &str) -> bool {
    p2p::relay_tcp_reachable(multiaddr).await
}

pub(crate) fn p2p_listen_port_from_env() -> Option<u16> {
    std::env::var("INERTIA_P2P_LISTEN_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&port| port > 0)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct P2pStatus {
    pub running: bool,
    pub peer_id: Option<String>,
    pub listen_addresses: Vec<String>,
    pub connected_peer_ids: Vec<String>,
    /// True when `INERTIA_RELAY` or settings relay multiaddr is set.
    pub relay_configured: bool,
    /// Peer id extracted from the configured relay multiaddr.
    pub relay_peer_id: Option<String>,
    /// libp2p session to the relay peer is up.
    pub relay_connected: bool,
    /// TCP connect probe to the relay host:port (None if relay not configured).
    pub relay_tcp_reachable: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PublishPhotoResult {
    pub photo: ProfilePhoto,
    pub content_id: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InviteResponse {
    pub link: String,
    pub payload: String,
    pub safety_code: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub display_name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InvitePreview {
    pub display_name: String,
    pub signing_pubkey: String,
    pub safety_code: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub peer_id: Option<String>,
    pub multiaddrs: Vec<String>,
    pub relay_multiaddr: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversationMessage {
    pub content_id: String,
    pub body: String,
    pub at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub is_own: bool,
    pub delivery_status: Option<DeliveryStatus>,
}
