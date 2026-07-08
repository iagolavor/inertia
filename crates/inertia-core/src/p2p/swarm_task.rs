//! Swarm actor: a single task that exclusively owns the libp2p swarm.
//!
//! No mutex guards the swarm. The facade (`P2pNode`) sends `Command`s over an
//! mpsc channel and reads `NetState` from a `watch` channel. The actor drives
//! `swarm.next()` continuously in a `tokio::select!`, so behaviour events
//! (relay reservations, inbound requests) are processed the moment they arrive.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use libp2p::core::transport::ListenerId;
use libp2p::request_response::{
    Event as ReqResEvent, Message, OutboundRequestId, ResponseChannel,
};
use libp2p::swarm::SwarmEvent;
use libp2p::{dcutr, identify, relay, Multiaddr, PeerId, Swarm};
use tokio::sync::{mpsc, oneshot, watch, RwLock};
use tracing::{debug, info, warn};

use crate::error::{CoreError, CoreResult};
use crate::identity::Identity;
use crate::storage::ConnectionState;
use crate::store_handle::StoreHandle;

use super::behaviour::{InertiaBehaviour, InertiaBehaviourEvent};
use super::events::P2pEvent;
use super::handlers::{
    handle_inbound_request, handle_outbound_response, persist_peer_multiaddrs,
    update_contact_state,
};
use super::multiaddr::{
    filter_friend_multiaddrs, is_confirmed_relay_circuit_listen_addr,
    is_relay_circuit_multiaddr_str, peer_id_from_multiaddr, relay_circuit_listen_addr,
    relay_peer_id_from_circuit_multiaddr,
};
use super::protocol::{InertiaRequest, InertiaResponse};

/// Commands sent from the `P2pNode` facade to the swarm actor.
pub(super) enum Command {
    Dial {
        addr: Multiaddr,
        reply: oneshot::Sender<CoreResult<()>>,
    },
    /// Register relays and open circuit listeners on the ones already connected.
    /// Newly connecting relays get their circuit listener automatically.
    EnsureRelayCircuits { relays: Vec<String> },
    SendRequest {
        peer: PeerId,
        request: InertiaRequest,
        /// When set, the matching response (or outbound failure) is delivered here.
        /// When `None`, responses flow through `handle_outbound_response`.
        reply: Option<oneshot::Sender<InertiaResponse>>,
    },
    SendResponse {
        channel: ResponseChannel<InertiaResponse>,
        response: InertiaResponse,
    },
}

/// Network state published by the actor. Read cheaply via `watch`; never locks the swarm.
#[derive(Clone, Default)]
pub struct NetState {
    pub connected: HashSet<PeerId>,
    /// Relay peer ids that currently hold an inbound circuit reservation for us.
    pub reservations: HashSet<PeerId>,
    /// Peers with at least one direct (non-relay-circuit) connection.
    pub direct: HashSet<PeerId>,
    pub listen_addrs: Vec<Multiaddr>,
    pub external_addrs: Vec<Multiaddr>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    swarm: Swarm<InertiaBehaviour>,
    peer_id: PeerId,
    cmd_rx: mpsc::UnboundedReceiver<Command>,
    cmd_tx: mpsc::UnboundedSender<Command>,
    state_tx: watch::Sender<NetState>,
    store: StoreHandle,
    identity: Arc<RwLock<Identity>>,
    event_tx: mpsc::UnboundedSender<P2pEvent>,
    initial_relays: Vec<String>,
) {
    let mut task = SwarmTask {
        swarm,
        peer_id,
        cmd_rx,
        cmd_tx,
        state_tx,
        store,
        identity,
        event_tx,
        pending: HashMap::new(),
        known_relays: HashMap::new(),
        circuit_listeners: HashMap::new(),
    };
    task.register_relays(&initial_relays);
    tokio::spawn(async move { task.run().await });
}

struct SwarmTask {
    swarm: Swarm<InertiaBehaviour>,
    peer_id: PeerId,
    cmd_rx: mpsc::UnboundedReceiver<Command>,
    /// Handed to spawned inbound-request handlers so they can reply via `SendResponse`.
    cmd_tx: mpsc::UnboundedSender<Command>,
    state_tx: watch::Sender<NetState>,
    store: StoreHandle,
    identity: Arc<RwLock<Identity>>,
    event_tx: mpsc::UnboundedSender<P2pEvent>,
    /// Outbound requests awaiting a response for a facade caller.
    pending: HashMap<OutboundRequestId, oneshot::Sender<InertiaResponse>>,
    /// Relays we keep inbound circuit listeners on, by relay peer id.
    known_relays: HashMap<PeerId, Multiaddr>,
    /// Active circuit listeners mapped to their relay peer id.
    circuit_listeners: HashMap<ListenerId, PeerId>,
}

impl SwarmTask {
    async fn run(&mut self) {
        loop {
            tokio::select! {
                event = futures::StreamExt::next(&mut self.swarm) => {
                    let Some(event) = event else { break };
                    self.handle_swarm_event(event).await;
                }
                cmd = self.cmd_rx.recv() => {
                    let Some(cmd) = cmd else {
                        info!("p2p command channel closed - stopping swarm task");
                        break;
                    };
                    self.handle_command(cmd);
                }
            }
        }
    }

    fn handle_command(&mut self, cmd: Command) {
        match cmd {
            Command::Dial { addr, reply } => {
                let result = self
                    .swarm
                    .dial(addr)
                    .map_err(|e| CoreError::P2p(e.to_string()));
                let _ = reply.send(result);
            }
            Command::EnsureRelayCircuits { relays } => {
                self.register_relays(&relays);
                let connected: Vec<PeerId> = self
                    .known_relays
                    .keys()
                    .filter(|id| self.swarm.is_connected(id))
                    .copied()
                    .collect();
                for relay_peer_id in connected {
                    self.listen_on_relay_circuit(relay_peer_id);
                }
            }
            Command::SendRequest { peer, request, reply } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer, request);
                if let Some(reply) = reply {
                    self.pending.insert(request_id, reply);
                }
            }
            Command::SendResponse { channel, response } => {
                let _ = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, response);
            }
        }
    }

    fn register_relays(&mut self, relays: &[String]) {
        for relay in relays {
            let trimmed = relay.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(relay_addr) = trimmed.parse::<Multiaddr>() else {
                warn!(relay = %trimmed, "invalid relay multiaddr");
                continue;
            };
            let Ok(relay_peer_id) = peer_id_from_multiaddr(&relay_addr) else {
                warn!(relay = %trimmed, "relay multiaddr missing /p2p peer id");
                continue;
            };
            self.known_relays.insert(relay_peer_id, relay_addr);
        }
    }

    /// Open an inbound circuit listener on a connected relay (idempotent).
    fn listen_on_relay_circuit(&mut self, relay_peer_id: PeerId) {
        if self
            .circuit_listeners
            .values()
            .any(|peer| *peer == relay_peer_id)
        {
            return;
        }
        let Some(relay_addr) = self.known_relays.get(&relay_peer_id).cloned() else {
            return;
        };
        let circuit_addr = relay_circuit_listen_addr(&relay_addr);
        match self.swarm.listen_on(circuit_addr.clone()) {
            Ok(listener_id) => {
                info!(%circuit_addr, ?listener_id, "listening via relay circuit");
                self.circuit_listeners.insert(listener_id, relay_peer_id);
            }
            Err(e) => warn!(error = %e, %circuit_addr, "failed to listen via relay circuit"),
        }
    }

    fn mark_reservation(&mut self, relay_peer_id: PeerId, source: &str) {
        let mut inserted = false;
        self.state_tx.send_modify(|state| {
            inserted = state.reservations.insert(relay_peer_id);
        });
        if inserted {
            info!(%relay_peer_id, source, "relay reservation active - inbound circuits enabled");
        }
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<InertiaBehaviourEvent>) {
        match event {
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                info!(%peer_id, "peer connected");
                let remote = endpoint.get_remote_address().to_string();
                let direct = !remote.contains("/p2p-circuit/");
                self.state_tx.send_modify(|state| {
                    state.connected.insert(peer_id);
                    if direct {
                        state.direct.insert(peer_id);
                    }
                });
                // Self-heal: (re)open our inbound circuit listener whenever a
                // known relay session comes up.
                if self.known_relays.contains_key(&peer_id) {
                    self.listen_on_relay_circuit(peer_id);
                }
                if is_relay_circuit_multiaddr_str(&remote) {
                    persist_peer_multiaddrs(&self.store, &peer_id, &[remote]).await;
                }
                let _ = self.event_tx.send(P2pEvent::PeerConnected(peer_id));
                update_contact_state(&self.store, &peer_id, ConnectionState::Online).await;
            }
            SwarmEvent::ConnectionClosed { peer_id, num_established, .. } => {
                if num_established > 0 {
                    debug!(%peer_id, num_established, "connection closed - others remain");
                    return;
                }
                info!(%peer_id, "peer disconnected");
                self.state_tx.send_modify(|state| {
                    state.connected.remove(&peer_id);
                    state.direct.remove(&peer_id);
                    state.reservations.remove(&peer_id);
                });
                if self.known_relays.contains_key(&peer_id) {
                    self.circuit_listeners.retain(|_, relay| *relay != peer_id);
                    info!(%peer_id, "relay session lost - will re-reserve circuit on reconnect");
                }
                let _ = self.event_tx.send(P2pEvent::PeerDisconnected(peer_id));
                update_contact_state(&self.store, &peer_id, ConnectionState::Offline).await;
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                warn!(?peer_id, error = %error, "outgoing connection failed");
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                self.state_tx.send_modify(|state| {
                    if !state.listen_addrs.contains(&address) {
                        state.listen_addrs.push(address.clone());
                    }
                });
                if is_confirmed_relay_circuit_listen_addr(&address, &self.peer_id) {
                    if let Some(relay_peer_id) = relay_peer_id_from_circuit_multiaddr(&address) {
                        self.mark_reservation(relay_peer_id, "circuit listen addr");
                    }
                }
            }
            SwarmEvent::ExpiredListenAddr { address, .. } => {
                self.state_tx.send_modify(|state| {
                    state.listen_addrs.retain(|addr| addr != &address);
                });
            }
            SwarmEvent::ListenerClosed { listener_id, .. } => {
                self.on_circuit_listener_gone(listener_id, "listener closed");
            }
            SwarmEvent::ListenerError { listener_id, error } => {
                debug!(?listener_id, error = %error, "listener error");
                self.on_circuit_listener_gone(listener_id, "listener error");
            }
            SwarmEvent::ExternalAddrConfirmed { address } => {
                self.state_tx.send_modify(|state| {
                    if !state.external_addrs.contains(&address) {
                        state.external_addrs.push(address.clone());
                    }
                });
                if is_confirmed_relay_circuit_listen_addr(&address, &self.peer_id) {
                    if let Some(relay_peer_id) = relay_peer_id_from_circuit_multiaddr(&address) {
                        self.mark_reservation(relay_peer_id, "external addr confirmed");
                    }
                }
            }
            SwarmEvent::ExternalAddrExpired { address } => {
                self.state_tx.send_modify(|state| {
                    state.external_addrs.retain(|addr| addr != &address);
                });
            }
            SwarmEvent::Behaviour(InertiaBehaviourEvent::RelayClient(
                relay::client::Event::ReservationReqAccepted { relay_peer_id, .. },
            )) => {
                self.mark_reservation(relay_peer_id, "reservation accepted");
            }
            SwarmEvent::Behaviour(InertiaBehaviourEvent::RequestResponse(
                ReqResEvent::Message { peer, message },
            )) => match message {
                Message::Request { request, channel, .. } => {
                    // Offload to a task so slow store access never stalls the swarm.
                    let store = self.store.clone();
                    let identity = Arc::clone(&self.identity);
                    let event_tx = self.event_tx.clone();
                    let cmd_tx = self.cmd_tx.clone();
                    tokio::spawn(async move {
                        let response = match handle_inbound_request(
                            &store, &identity, &event_tx, peer, request,
                        )
                        .await
                        {
                            Ok(res) => res,
                            Err(e) => {
                                warn!(error = %e, "inbound request failed");
                                InertiaResponse::Error(e.to_string())
                            }
                        };
                        let _ = cmd_tx.send(Command::SendResponse { channel, response });
                    });
                }
                Message::Response { response, request_id, .. } => {
                    if let Some(reply) = self.pending.remove(&request_id) {
                        let _ = reply.send(response);
                    } else {
                        let store = self.store.clone();
                        let event_tx = self.event_tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) =
                                handle_outbound_response(&store, &event_tx, peer, response).await
                            {
                                warn!(error = %e, "outbound response handling failed");
                            }
                        });
                    }
                }
            },
            SwarmEvent::Behaviour(InertiaBehaviourEvent::RequestResponse(
                ReqResEvent::OutboundFailure { request_id, error, .. },
            )) => {
                warn!(error = %error, "outbound request failed");
                if let Some(reply) = self.pending.remove(&request_id) {
                    let _ = reply.send(InertiaResponse::Error(error.to_string()));
                }
            }
            SwarmEvent::Behaviour(InertiaBehaviourEvent::Dcutr(dcutr::Event {
                remote_peer_id,
                result,
            })) => match result {
                Ok(connection_id) => {
                    info!(%remote_peer_id, ?connection_id, "direct connection upgrade succeeded");
                    self.state_tx.send_modify(|state| {
                        state.direct.insert(remote_peer_id);
                    });
                }
                Err(error) => {
                    debug!(%remote_peer_id, ?error, "direct connection upgrade failed");
                }
            },
            SwarmEvent::Behaviour(InertiaBehaviourEvent::Identify(
                identify::Event::Received { peer_id, info, .. },
            )) => {
                let addrs = filter_friend_multiaddrs(
                    &info
                        .listen_addrs
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>(),
                );
                if !addrs.is_empty() {
                    persist_peer_multiaddrs(&self.store, &peer_id, &addrs).await;
                }
            }
            _ => debug!("swarm event"),
        }
    }

    /// A circuit listener died. Drop its reservation and retry after a short
    /// backoff so inbound reachability self-heals without hammering the relay.
    fn on_circuit_listener_gone(&mut self, listener_id: ListenerId, reason: &str) {
        let Some(relay_peer_id) = self.circuit_listeners.remove(&listener_id) else {
            return;
        };
        let still_listening = self
            .circuit_listeners
            .values()
            .any(|peer| *peer == relay_peer_id);
        if still_listening {
            return;
        }
        self.state_tx.send_modify(|state| {
            state.reservations.remove(&relay_peer_id);
        });
        warn!(%relay_peer_id, reason, "relay circuit listener gone - retrying in 3s");
        let Some(relay_addr) = self.known_relays.get(&relay_peer_id) else {
            return;
        };
        let relay = relay_addr.to_string();
        let cmd_tx = self.cmd_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let _ = cmd_tx.send(Command::EnsureRelayCircuits {
                relays: vec![relay],
            });
        });
    }
}
