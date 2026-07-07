use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use libp2p::request_response::{Event, Message, OutboundRequestId};
use libp2p::{dcutr, identify, noise, relay, tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::content::ContentEnvelope;
use crate::error::{CoreError, CoreResult};
use crate::identity::Identity;
use crate::storage::ConnectionState;
use crate::store_handle::StoreHandle;

use super::behaviour::{build_behaviour, InertiaBehaviour, InertiaBehaviourEvent};
use super::events::P2pEvent;
use super::handlers::{
    handle_inbound_request, handle_outbound_response, persist_peer_multiaddrs,
    update_contact_state,
};
use super::keypair::load_or_create_keypair;
use super::multiaddr::{
    ensure_peer_id_suffix, filter_friend_multiaddrs, is_relay_circuit_multiaddr_str,
    is_routable_multiaddr, peer_id_from_multiaddr, relay_circuit_listen_addr,
};
use super::protocol::{
    BlobChunkRequest, BlobData, BlobRequest, FriendRequest, InertiaRequest,
    InertiaResponse, InviteRedemption, SendEnvelope,
};


pub struct P2pNode {
    peer_id: PeerId,
    swarm: Arc<Mutex<Swarm<InertiaBehaviour>>>,
    store: StoreHandle,
    identity: Arc<RwLock<Identity>>,
    event_tx: mpsc::UnboundedSender<P2pEvent>,
    pending_responses:
        Arc<Mutex<HashMap<OutboundRequestId, mpsc::Sender<InertiaResponse>>>>,
    /// Peers with at least one direct (non-relay-circuit) connection.
    peer_direct: Arc<Mutex<std::collections::HashSet<PeerId>>>,
    /// Relay peer ids that accepted a circuit reservation for inbound dials.
    relay_reservations: Arc<Mutex<HashSet<PeerId>>>,
}

impl P2pNode {
    pub async fn start(
        store: StoreHandle,
        identity: Arc<RwLock<Identity>>,
        listen_addr: Multiaddr,
        relay_multiaddrs: Vec<String>,
        event_tx: mpsc::UnboundedSender<P2pEvent>,
    ) -> CoreResult<Self> {
        let data_dir = store
            .with(|s| Ok(s.data_dir().to_path_buf()))
            .await?;
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

        let node = Self {
            peer_id,
            swarm: Arc::new(Mutex::new(swarm)),
            store,
            identity,
            event_tx,
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
            peer_direct: Arc::new(Mutex::new(std::collections::HashSet::new())),
            relay_reservations: Arc::new(Mutex::new(HashSet::new())),
        };

        node.ensure_relay_circuits(&relay_multiaddrs).await;

        node.spawn_event_loop();
        Ok(node)
    }

    pub async fn ensure_relay_circuits(&self, relay_multiaddrs: &[String]) {
        let mut swarm = self.swarm.lock().await;
        for relay in relay_multiaddrs {
            let trimmed = relay.trim();
            if trimmed.is_empty() {
                continue;
            }
            match trimmed.parse::<Multiaddr>() {
                Ok(relay_addr) => {
                    let circuit_addr = relay_circuit_listen_addr(&relay_addr);
                    match swarm.listen_on(circuit_addr.clone()) {
                        Ok(listener_id) => {
                            info!(%circuit_addr, ?listener_id, "listening via relay circuit");
                        }
                        Err(e) => warn!(error = %e, %circuit_addr, "failed to listen via relay circuit"),
                    }
                }
                Err(e) => warn!(relay = %trimmed, error = %e, "invalid relay multiaddr"),
            }
        }
    }

    pub fn peer_id_string(&self) -> String {
        self.peer_id.to_string()
    }

    pub async fn listen_addresses(&self) -> Vec<Multiaddr> {
        let swarm = self.swarm.lock().await;
        swarm.listeners().cloned().collect()
    }

    pub async fn routable_listen_addresses(&self) -> Vec<String> {
        let swarm = self.swarm.lock().await;
        let peer_id = self.peer_id.to_string();
        let mut addrs: Vec<String> = swarm
            .external_addresses()
            .chain(swarm.listeners())
            .filter(|addr| is_routable_multiaddr(addr))
            .map(|addr| ensure_peer_id_suffix(addr, &peer_id))
            .filter(|addr| is_relay_circuit_multiaddr_str(addr))
            .collect();
        addrs.sort();
        addrs.dedup();
        addrs.sort_by_key(|addr| !addr.contains("/p2p-circuit/"));
        addrs
    }

    pub async fn connected_peer_ids(&self) -> Vec<String> {
        let swarm = self.swarm.lock().await;
        swarm
            .connected_peers()
            .map(|peer_id| peer_id.to_string())
            .collect()
    }

    /// True when at least one of the given relay peer ids accepted a circuit reservation.
    pub async fn has_relay_reservation(&self, relay_peer_ids: &[String]) -> bool {
        let wanted: HashSet<PeerId> = relay_peer_ids
            .iter()
            .filter_map(|id| id.parse::<PeerId>().ok())
            .collect();
        if wanted.is_empty() {
            return true;
        }
        let reserved = self.relay_reservations.lock().await;
        wanted.iter().any(|id| reserved.contains(id))
    }

    /// Wait until a configured relay accepts our circuit reservation (inbound via relay).
    pub async fn wait_for_relay_reservation(
        &self,
        relay_peer_ids: &[String],
        timeout: Duration,
    ) -> bool {
        let wanted: HashSet<PeerId> = relay_peer_ids
            .iter()
            .filter_map(|id| id.parse::<PeerId>().ok())
            .collect();
        if wanted.is_empty() {
            return true;
        }
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            {
                let reserved = self.relay_reservations.lock().await;
                if wanted.iter().any(|id| reserved.contains(id)) {
                    return true;
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    pub async fn dial(&self, addr: Multiaddr) -> CoreResult<()> {
        let mut swarm = self.swarm.lock().await;
        swarm
            .dial(addr)
            .map_err(|e| CoreError::P2p(e.to_string()))?;
        Ok(())
    }

    pub async fn redeem_invite(
        &self,
        peer_id: PeerId,
        redemption: InviteRedemption,
    ) -> CoreResult<()> {
        let (tx, mut rx) = mpsc::channel(1);
        let request_id = {
            let mut swarm = self.swarm.lock().await;
            let id = swarm.behaviour_mut().request_response.send_request(
                &peer_id,
                InertiaRequest::InviteRedemption(redemption),
            );
            self.pending_responses.lock().await.insert(id, tx);
            id
        };

        let response = tokio::time::timeout(Duration::from_secs(30), rx.recv())
            .await
            .map_err(|_| {
                CoreError::Invite("inviter did not respond in time — are they online?".into())
            })?
            .ok_or_else(|| CoreError::P2p("invite redemption channel closed".into()))?;

        self.pending_responses.lock().await.remove(&request_id);

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

        let mut swarm = self.swarm.lock().await;
        swarm
            .dial(addr)
            .map_err(|e| CoreError::P2p(e.to_string()))?;

        swarm.behaviour_mut().request_response.send_request(
            &peer_id,
            InertiaRequest::FriendRequest(req),
        );

        drop(swarm);

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
        let mut swarm = self.swarm.lock().await;
        swarm.behaviour_mut().request_response.send_request(
            &peer_id,
            InertiaRequest::SendEnvelope(SendEnvelope { envelope }),
        );
        Ok(())
    }

    pub async fn has_direct_connection(&self, peer_id: PeerId) -> bool {
        self.peer_direct.lock().await.contains(&peer_id)
    }

    pub async fn wait_for_direct(&self, peer_id: PeerId, timeout: Duration) -> bool {
        if self.has_direct_connection(peer_id).await {
            return true;
        }
        let deadline = tokio::time::Instant::now() + timeout;
        while tokio::time::Instant::now() < deadline {
            if self.has_direct_connection(peer_id).await {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        false
    }

    pub async fn request_chunk_from_peer(
        &self,
        peer_id: PeerId,
        root_hash: &str,
        chunk_index: u32,
        expected_hash: &str,
    ) -> CoreResult<()> {
        let (tx, mut rx) = mpsc::channel(1);
        let request_id = {
            let mut swarm = self.swarm.lock().await;
            let id = swarm.behaviour_mut().request_response.send_request(
                &peer_id,
                InertiaRequest::BlobChunkRequest(BlobChunkRequest {
                    root_hash: root_hash.to_string(),
                    chunk_index,
                }),
            );
            self.pending_responses.lock().await.insert(id, tx);
            id
        };

        let response = tokio::time::timeout(Duration::from_secs(45), rx.recv())
            .await
            .map_err(|_| CoreError::P2p("chunk request timed out".into()))?
            .ok_or_else(|| CoreError::P2p("chunk request channel closed".into()))?;

        self.pending_responses.lock().await.remove(&request_id);

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
        let (tx, mut rx) = mpsc::channel(1);
        let request_id = {
            let mut swarm = self.swarm.lock().await;
            let id = swarm.behaviour_mut().request_response.send_request(
                &peer_id,
                InertiaRequest::BlobRequest(BlobRequest {
                    hash: hash.to_string(),
                }),
            );
            self.pending_responses.lock().await.insert(id, tx);
            id
        };

        let response = tokio::time::timeout(Duration::from_secs(60), rx.recv())
            .await
            .map_err(|_| CoreError::P2p("blob request timed out".into()))?
            .ok_or_else(|| CoreError::P2p("blob request channel closed".into()))?;

        self.pending_responses.lock().await.remove(&request_id);

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

    pub async fn push_blob_to_peer(
        &self,
        peer_id: PeerId,
        hash: &str,
        data: &[u8],
    ) -> CoreResult<()> {
        let (tx, mut rx) = mpsc::channel(1);
        let request_id = {
            let mut swarm = self.swarm.lock().await;
            let id = swarm.behaviour_mut().request_response.send_request(
                &peer_id,
                InertiaRequest::BlobPush(BlobData {
                    hash: hash.to_string(),
                    data: data.to_vec(),
                }),
            );
            self.pending_responses.lock().await.insert(id, tx);
            id
        };

        let response = tokio::time::timeout(Duration::from_secs(60), rx.recv())
            .await
            .map_err(|_| CoreError::P2p("blob push timed out".into()))?
            .ok_or_else(|| CoreError::P2p("blob push channel closed".into()))?;

        self.pending_responses.lock().await.remove(&request_id);

        match response {
            InertiaResponse::Ok => Ok(()),
            InertiaResponse::Error(msg) => Err(CoreError::P2p(msg)),
            other => Err(CoreError::P2p(format!(
                "unexpected blob push response: {other:?}"
            ))),
        }
    }

    fn spawn_event_loop(&self) {
        let swarm = Arc::clone(&self.swarm);
        let store = self.store.clone();
        let identity = Arc::clone(&self.identity);
        let event_tx = self.event_tx.clone();
        let pending_responses = Arc::clone(&self.pending_responses);
        let peer_direct = Arc::clone(&self.peer_direct);
        let relay_reservations = Arc::clone(&self.relay_reservations);

        tokio::spawn(async move {
            loop {
                let event = {
                    let mut swarm = swarm.lock().await;
                    StreamExt::next(&mut *swarm).now_or_never().flatten()
                };

                let Some(event) = event else {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                };

                match event {
                    libp2p::swarm::SwarmEvent::ConnectionEstablished {
                        peer_id,
                        endpoint,
                        ..
                    } => {
                        info!(%peer_id, "peer connected");
                        let remote = endpoint.get_remote_address().to_string();
                        if !remote.contains("/p2p-circuit/") {
                            peer_direct.lock().await.insert(peer_id);
                        }
                        if is_relay_circuit_multiaddr_str(&remote) {
                            persist_peer_multiaddrs(&store, &peer_id, &[remote]).await;
                        }
                        let _ = event_tx.send(P2pEvent::PeerConnected(peer_id));
                        update_contact_state(&store, &peer_id, ConnectionState::Online).await;
                    }
                    libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        info!(%peer_id, "peer disconnected");
                        peer_direct.lock().await.remove(&peer_id);
                        let _ = event_tx.send(P2pEvent::PeerDisconnected(peer_id));
                        update_contact_state(&store, &peer_id, ConnectionState::Offline).await;
                    }
                    libp2p::swarm::SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        warn!(?peer_id, error = %error, "outgoing connection failed");
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(
                        InertiaBehaviourEvent::RequestResponse(Event::Message {
                            peer,
                            message,
                        }),
                    ) => {
                        if let Message::Request { request, channel, .. } = message {
                            let response = match handle_inbound_request(
                                &store,
                                &identity,
                                &event_tx,
                                peer,
                                request,
                            )
                            .await
                            {
                                Ok(res) => res,
                                Err(e) => {
                                    warn!(error = %e, "inbound request failed");
                                    InertiaResponse::Error(e.to_string())
                                }
                            };
                            let mut swarm = swarm.lock().await;
                            let _ = swarm
                                .behaviour_mut()
                                .request_response
                                .send_response(channel, response);
                        } else if let Message::Response {
                            response,
                            request_id,
                            ..
                        } = message
                        {
                            if let Some(tx) =
                                pending_responses.lock().await.remove(&request_id)
                            {
                                let _ = tx.send(response).await;
                            } else if let Err(e) = handle_outbound_response(
                                &store,
                                &event_tx,
                                peer,
                                response,
                            )
                            .await
                            {
                                warn!(error = %e, "outbound response handling failed");
                            }
                        }
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(
                        InertiaBehaviourEvent::RequestResponse(Event::OutboundFailure {
                            request_id,
                            error,
                            ..
                        }),
                    ) => {
                        warn!(error = %error, "outbound request failed");
                        if let Some(tx) = pending_responses.lock().await.remove(&request_id) {
                            let _ = tx
                                .send(InertiaResponse::Error(error.to_string()))
                                .await;
                        }
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(
                        InertiaBehaviourEvent::RelayClient(relay::client::Event::ReservationReqAccepted {
                            relay_peer_id,
                            ..
                        }),
                    ) => {
                        relay_reservations.lock().await.insert(relay_peer_id);
                        info!(%relay_peer_id, "relay reservation accepted");
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(
                        InertiaBehaviourEvent::Dcutr(dcutr::Event { remote_peer_id, result }),
                    ) => match result {
                        Ok(connection_id) => {
                            info!(%remote_peer_id, ?connection_id, "direct connection upgrade succeeded");
                            peer_direct.lock().await.insert(remote_peer_id);
                        }
                        Err(error) => {
                            debug!(%remote_peer_id, ?error, "direct connection upgrade failed");
                        }
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(
                        InertiaBehaviourEvent::Identify(identify::Event::Received {
                            peer_id,
                            info,
                            ..
                        }),
                    ) => {
                        let addrs = filter_friend_multiaddrs(
                            &info
                                .listen_addrs
                                .iter()
                                .map(|a| a.to_string())
                                .collect::<Vec<_>>(),
                        );
                        if !addrs.is_empty() {
                            persist_peer_multiaddrs(&store, &peer_id, &addrs).await;
                        }
                    }
                    _ => debug!("swarm event"),
                }
            }
        });
    }
}
