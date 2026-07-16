//! Thin handle over the swarm actor (`swarm_task`).
//!
//! `P2pNode` holds a command sender and a `watch` receiver. All swarm access
//! goes through the actor's command channel; all state reads come from the
//! watch channel. No method here can block the swarm or miss its events.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use libp2p::{noise, tcp, yamux, Multiaddr, PeerId, SwarmBuilder};
use tokio::sync::{mpsc, oneshot, watch, RwLock};

use crate::content::ContentEnvelope;
use crate::error::{CoreError, CoreResult};
use crate::identity::Identity;
use crate::store_handle::StoreHandle;

use super::behaviour::build_behaviour;
use super::events::P2pEvent;
use super::keypair::load_or_create_keypair;
use super::multiaddr::{
    ensure_peer_id_suffix, is_relay_circuit_multiaddr_str, is_routable_multiaddr,
    peer_id_from_multiaddr,
};
use super::protocol::{
    ArchiveListRequest, BlobChunkRequest, BlobData, BlobRequest, FriendRequest, InertiaRequest,
    InertiaResponse, InviteRedemption, ProfileCommentsRequest, ProfileManifestRequest,
    SendEnvelope,
};
use crate::storage::{ArchiveEntry, ArchiveFolderSummary, ProfileComment, ProfileManifest};
use super::swarm_task::{self, Command, NetState};

#[derive(Clone)]
pub struct P2pNode {
    peer_id: PeerId,
    cmd_tx: mpsc::UnboundedSender<Command>,
    state_rx: watch::Receiver<NetState>,
    store: StoreHandle,
    identity: Arc<RwLock<Identity>>,
}

impl P2pNode {
    pub async fn start(
        store: StoreHandle,
        identity: Arc<RwLock<Identity>>,
        listen_addr: Multiaddr,
        relay_multiaddrs: Vec<String>,
        event_tx: mpsc::UnboundedSender<P2pEvent>,
    ) -> CoreResult<Self> {
        let data_dir = store.with(|s| Ok(s.data_dir().to_path_buf())).await?;
        let local_key = load_or_create_keypair(&data_dir)?;
        let peer_id = local_key.public().to_peer_id();

        let mut swarm = SwarmBuilder::with_existing_identity(local_key.clone())
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| CoreError::P2p(e.to_string()))?
            .with_relay_client(noise::Config::new, yamux::Config::default)
            .map_err(|e| CoreError::P2p(e.to_string()))?
            .with_behaviour(|key, relay_client| build_behaviour(key, relay_client))
            .map_err(|e| CoreError::P2p(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(300)))
            .build();

        swarm
            .listen_on(listen_addr)
            .map_err(|e| CoreError::P2p(e.to_string()))?;

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (state_tx, state_rx) = watch::channel(NetState::default());

        swarm_task::spawn(
            swarm,
            peer_id,
            cmd_rx,
            cmd_tx.clone(),
            state_tx,
            store.clone(),
            Arc::clone(&identity),
            event_tx,
            relay_multiaddrs,
        );

        Ok(Self {
            peer_id,
            cmd_tx,
            state_rx,
            store,
            identity,
        })
    }

    fn send_command(&self, cmd: Command) -> CoreResult<()> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| CoreError::P2p("p2p task stopped".into()))
    }

    /// Register relays with the actor and open inbound circuit listeners on the
    /// connected ones. Relays connecting later get listeners automatically.
    pub async fn ensure_relay_circuits(&self, relay_multiaddrs: &[String]) {
        let _ = self.send_command(Command::EnsureRelayCircuits {
            relays: relay_multiaddrs.to_vec(),
        });
    }

    pub fn peer_id_string(&self) -> String {
        self.peer_id.to_string()
    }

    pub async fn listen_addresses(&self) -> Vec<Multiaddr> {
        self.state_rx.borrow().listen_addrs.clone()
    }

    pub async fn routable_listen_addresses(&self) -> Vec<String> {
        let peer_id = self.peer_id.to_string();
        let state = self.state_rx.borrow();
        let mut addrs: Vec<String> = state
            .external_addrs
            .iter()
            .chain(state.listen_addrs.iter())
            .filter(|addr| is_routable_multiaddr(addr))
            .map(|addr| ensure_peer_id_suffix(addr, &peer_id))
            .filter(|addr| is_relay_circuit_multiaddr_str(addr))
            .collect();
        drop(state);
        addrs.sort();
        addrs.dedup();
        addrs.sort_by_key(|addr| !addr.contains("/p2p-circuit/"));
        addrs
    }

    pub async fn connected_peer_ids(&self) -> Vec<String> {
        self.state_rx
            .borrow()
            .connected
            .iter()
            .map(|peer_id| peer_id.to_string())
            .collect()
    }

    /// True when at least one of the given relay peer ids holds a circuit reservation.
    pub async fn has_relay_reservation(&self, relay_peer_ids: &[String]) -> bool {
        let wanted = parse_peer_ids(relay_peer_ids);
        if wanted.is_empty() {
            return true;
        }
        let state = self.state_rx.borrow();
        wanted.iter().any(|id| state.reservations.contains(id))
    }

    /// Wait until a configured relay accepts our circuit reservation (inbound via relay).
    /// Event-driven: returns within milliseconds of the reservation being granted.
    pub async fn wait_for_relay_reservation(
        &self,
        relay_peer_ids: &[String],
        timeout: Duration,
    ) -> bool {
        let wanted = parse_peer_ids(relay_peer_ids);
        if wanted.is_empty() {
            return true;
        }
        let mut rx = self.state_rx.clone();
        tokio::time::timeout(
            timeout,
            rx.wait_for(|state| wanted.iter().any(|id| state.reservations.contains(id))),
        )
        .await
        .map(|result| result.is_ok())
        .unwrap_or(false)
    }

    /// Wait until any of the given peers has a live connection.
    pub async fn wait_for_any_connected(
        &self,
        peer_ids: &[String],
        timeout: Duration,
    ) -> bool {
        let wanted = parse_peer_ids(peer_ids);
        if wanted.is_empty() {
            return true;
        }
        let mut rx = self.state_rx.clone();
        tokio::time::timeout(
            timeout,
            rx.wait_for(|state| wanted.iter().any(|id| state.connected.contains(id))),
        )
        .await
        .map(|result| result.is_ok())
        .unwrap_or(false)
    }

    pub async fn dial(&self, addr: Multiaddr) -> CoreResult<()> {
        let (tx, rx) = oneshot::channel();
        self.send_command(Command::Dial { addr, reply: tx })?;
        rx.await
            .map_err(|_| CoreError::P2p("p2p task stopped".into()))?
    }

    /// Queue an outbound request; the response arrives on the returned channel.
    fn send_request(
        &self,
        peer: PeerId,
        request: InertiaRequest,
    ) -> CoreResult<oneshot::Receiver<InertiaResponse>> {
        let (tx, rx) = oneshot::channel();
        self.send_command(Command::SendRequest {
            peer,
            request,
            reply: Some(tx),
        })?;
        Ok(rx)
    }

    pub async fn redeem_invite(
        &self,
        peer_id: PeerId,
        redemption: InviteRedemption,
    ) -> CoreResult<()> {
        let rx = self.send_request(peer_id, InertiaRequest::InviteRedemption(redemption))?;
        let response = tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| {
                CoreError::Invite("inviter did not respond in time - are they online?".into())
            })?
            .map_err(|_| CoreError::P2p("invite redemption channel closed".into()))?;

        match response {
            InertiaResponse::Ok => Ok(()),
            InertiaResponse::Error(msg) => Err(CoreError::Invite(msg)),
            _ => Err(CoreError::Invite(
                "unexpected response to invite redemption".into(),
            )),
        }
    }

    pub async fn send_friend_request(&self, contact_id: &str, addr: Multiaddr) -> CoreResult<()> {
        let identity = self.identity.read().await;
        let req = FriendRequest {
            display_name: identity.display_name.clone(),
            phone_hash: identity.phone_hash.clone(),
            signing_pubkey: identity.signing_pubkey.clone(),
            encryption_pubkey: identity.encryption_pubkey.clone(),
            peer_id: self.peer_id_string(),
        };
        drop(identity);

        let peer_id = peer_id_from_multiaddr(&addr)?;
        self.dial(addr).await?;
        self.send_command(Command::SendRequest {
            peer: peer_id,
            request: InertiaRequest::FriendRequest(req),
            reply: None,
        })?;

        self.store
            .with_mut(|store| {
                if let Ok(mut contact) = store.get_contact(contact_id) {
                    contact.peer_id = Some(peer_id.to_string());
                    store.upsert_contact(&contact)?;
                }
                Ok(())
            })
            .await?;

        Ok(())
    }

    pub async fn send_envelope_to_peer(
        &self,
        peer_id: PeerId,
        envelope: ContentEnvelope,
    ) -> CoreResult<()> {
        self.send_command(Command::SendRequest {
            peer: peer_id,
            request: InertiaRequest::SendEnvelope(SendEnvelope { envelope }),
            reply: None,
        })
    }

    pub async fn has_direct_connection(&self, peer_id: PeerId) -> bool {
        self.state_rx.borrow().direct.contains(&peer_id)
    }

    pub async fn wait_for_direct(&self, peer_id: PeerId, timeout: Duration) -> bool {
        let mut rx = self.state_rx.clone();
        tokio::time::timeout(timeout, rx.wait_for(|state| state.direct.contains(&peer_id)))
            .await
            .map(|result| result.is_ok())
            .unwrap_or(false)
    }

    pub async fn request_chunk_from_peer(
        &self,
        peer_id: PeerId,
        root_hash: &str,
        chunk_index: u32,
        expected_hash: &str,
    ) -> CoreResult<()> {
        let rx = self.send_request(
            peer_id,
            InertiaRequest::BlobChunkRequest(BlobChunkRequest {
                root_hash: root_hash.to_string(),
                chunk_index,
            }),
        )?;
        let response = tokio::time::timeout(Duration::from_secs(45), rx)
            .await
            .map_err(|_| CoreError::P2p("chunk request timed out".into()))?
            .map_err(|_| CoreError::P2p("chunk request channel closed".into()))?;

        match response {
            InertiaResponse::BlobChunkData(chunk) => {
                self.store
                    .with_mut(|s| {
                        s.store_chunk_verified(
                            &chunk.root_hash,
                            chunk.chunk_index,
                            expected_hash,
                            &chunk.data,
                        )
                    })
                    .await?;
                Ok(())
            }
            InertiaResponse::BlobChunkNotFound => Err(CoreError::P2p(format!(
                "peer missing chunk {root_hash}#{chunk_index}"
            ))),
            InertiaResponse::Error(msg) => Err(CoreError::P2p(msg)),
            other => Err(CoreError::P2p(format!(
                "unexpected chunk response: {other:?}"
            ))),
        }
    }

    pub async fn request_blob_from_peer(&self, peer_id: PeerId, hash: &str) -> CoreResult<()> {
        let rx = self.send_request(
            peer_id,
            InertiaRequest::BlobRequest(BlobRequest {
                hash: hash.to_string(),
            }),
        )?;
        let response = tokio::time::timeout(Duration::from_secs(60), rx)
            .await
            .map_err(|_| CoreError::P2p("blob request timed out".into()))?
            .map_err(|_| CoreError::P2p("blob request channel closed".into()))?;

        match response {
            InertiaResponse::BlobData(blob) => {
                self.store
                    .with_mut(|s| s.store_blob_verified(&blob.hash, &blob.data))
                    .await?;
                Ok(())
            }
            InertiaResponse::BlobNotFound => Err(CoreError::P2p(format!(
                "peer does not have blob {hash}"
            ))),
            InertiaResponse::Error(msg) => Err(CoreError::P2p(msg)),
            other => Err(CoreError::P2p(format!(
                "unexpected blob response: {other:?}"
            ))),
        }
    }

    pub async fn request_profile_manifest_from_peer(
        &self,
        peer_id: PeerId,
    ) -> CoreResult<ProfileManifest> {
        let rx = self.send_request(
            peer_id,
            InertiaRequest::ProfileManifest(ProfileManifestRequest {}),
        )?;
        let response = tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| CoreError::P2p("profile manifest request timed out".into()))?
            .map_err(|_| CoreError::P2p("profile manifest channel closed".into()))?;

        match response {
            InertiaResponse::ProfileManifest(manifest) => Ok(manifest),
            InertiaResponse::Error(msg) => Err(CoreError::P2p(msg)),
            other => Err(CoreError::P2p(format!(
                "unexpected profile manifest response: {other:?}"
            ))),
        }
    }

    pub async fn request_profile_comments_from_peer(
        &self,
        peer_id: PeerId,
        profile_item_id: &str,
    ) -> CoreResult<Vec<ProfileComment>> {
        let rx = self.send_request(
            peer_id,
            InertiaRequest::ProfileComments(ProfileCommentsRequest {
                profile_item_id: profile_item_id.to_string(),
            }),
        )?;
        let response = tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| CoreError::P2p("profile comments request timed out".into()))?
            .map_err(|_| CoreError::P2p("profile comments channel closed".into()))?;

        match response {
            InertiaResponse::ProfileComments(comments) => Ok(comments),
            InertiaResponse::Error(msg) => Err(CoreError::P2p(msg)),
            other => Err(CoreError::P2p(format!(
                "unexpected profile comments response: {other:?}"
            ))),
        }
    }

    pub async fn request_archive_list_from_peer(
        &self,
        peer_id: PeerId,
        folder_id: &str,
    ) -> CoreResult<(
        ArchiveFolderSummary,
        Vec<ArchiveEntry>,
        Vec<crate::content::MediaManifest>,
    )> {
        let rx = self.send_request(
            peer_id,
            InertiaRequest::ArchiveList(ArchiveListRequest {
                folder_id: folder_id.to_string(),
            }),
        )?;
        let response = tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| CoreError::P2p("archive list request timed out".into()))?
            .map_err(|_| CoreError::P2p("archive list channel closed".into()))?;

        match response {
            InertiaResponse::ArchiveList {
                folder,
                entries,
                manifests,
            } => Ok((folder, entries, manifests)),
            InertiaResponse::ArchiveNotFound => Err(CoreError::ContentNotFound(folder_id.into())),
            InertiaResponse::Error(msg) => Err(CoreError::P2p(msg)),
            other => Err(CoreError::P2p(format!(
                "unexpected archive list response: {other:?}"
            ))),
        }
    }

    pub async fn push_blob_to_peer(
        &self,
        peer_id: PeerId,
        hash: &str,
        data: &[u8],
    ) -> CoreResult<()> {
        let rx = self.send_request(
            peer_id,
            InertiaRequest::BlobPush(BlobData {
                hash: hash.to_string(),
                data: data.to_vec(),
            }),
        )?;
        let response = tokio::time::timeout(Duration::from_secs(60), rx)
            .await
            .map_err(|_| CoreError::P2p("blob push timed out".into()))?
            .map_err(|_| CoreError::P2p("blob push channel closed".into()))?;

        match response {
            InertiaResponse::Ok => Ok(()),
            InertiaResponse::Error(msg) => Err(CoreError::P2p(msg)),
            other => Err(CoreError::P2p(format!(
                "unexpected blob push response: {other:?}"
            ))),
        }
    }
}

fn parse_peer_ids(peer_ids: &[String]) -> HashSet<PeerId> {
    peer_ids
        .iter()
        .filter_map(|id| id.parse::<PeerId>().ok())
        .collect()
}
