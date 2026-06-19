use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use libp2p::request_response::{self, Event, Message, OutboundRequestId};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{
    dcutr, identify, multiaddr::Protocol, noise, relay, tcp, yamux, Multiaddr, PeerId, Swarm,
    SwarmBuilder,
};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::content::{ContentEnvelope, ContentType, DeliveryStatus, MessagePayload, PostPayload};
use crate::crypto::{decrypt_from_sender, encrypt_for_recipient};
use crate::error::{CoreError, CoreResult};
use crate::identity::Identity;
use crate::storage::{ConnectionState, Contact, InboxEntry};
use crate::store_handle::StoreHandle;

use super::codec::{protocol_stream, request_response_config, JsonCodec};
use super::keypair::load_or_create_keypair;
use super::protocol::{
    FriendRequest, InertiaRequest, InertiaResponse, InviteRedemption, SendEnvelope,
};

#[derive(NetworkBehaviour)]
pub struct InertiaBehaviour {
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub request_response: request_response::Behaviour<JsonCodec>,
    pub identify: identify::Behaviour,
}

pub struct P2pNode {
    peer_id: PeerId,
    swarm: Arc<Mutex<Swarm<InertiaBehaviour>>>,
    store: StoreHandle,
    identity: Arc<RwLock<Identity>>,
    event_tx: mpsc::UnboundedSender<P2pEvent>,
    pending_responses: Arc<Mutex<HashMap<OutboundRequestId, mpsc::Sender<InertiaResponse>>>>,
}

#[derive(Debug, Clone)]
pub enum P2pEvent {
    FriendRequestReceived(FriendRequest),
    MessageReceived { sender_id: String, body: String },
    DeliveryAcked { content_id: String },
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
}

impl P2pNode {
    pub async fn start(
        store: StoreHandle,
        identity: Arc<RwLock<Identity>>,
        listen_addr: Multiaddr,
        relay_multiaddr: Option<String>,
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
            .with_behaviour(move |key, relay_client| {
                Ok(InertiaBehaviour {
                    relay_client,
                    dcutr: dcutr::Behaviour::new(key.public().to_peer_id()),
                    request_response: request_response::Behaviour::new(
                        [(protocol_stream(), request_response::ProtocolSupport::Full)],
                        request_response_config(),
                    ),
                    identify: identify::Behaviour::new(identify::Config::new(
                        "/inertia/1.0.0".into(),
                        key.public(),
                    )),
                })
            })
            .map_err(|e| CoreError::P2p(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        swarm
            .listen_on(listen_addr)
            .map_err(|e| CoreError::P2p(e.to_string()))?;

        if let Some(relay) = relay_multiaddr.as_deref().filter(|s| !s.trim().is_empty()) {
            match relay.trim().parse::<Multiaddr>() {
                Ok(relay_addr) => {
                    let circuit_addr = relay_circuit_listen_addr(&relay_addr);
                    match swarm.listen_on(circuit_addr.clone()) {
                        Ok(listener_id) => {
                            info!(%circuit_addr, ?listener_id, "listening via relay circuit");
                        }
                        Err(e) => warn!(error = %e, "failed to listen via relay circuit"),
                    }
                }
                Err(e) => warn!(relay = %relay, error = %e, "invalid relay multiaddr"),
            }
        }

        let node = Self {
            peer_id,
            swarm: Arc::new(Mutex::new(swarm)),
            store,
            identity,
            event_tx,
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
        };

        node.spawn_event_loop();
        Ok(node)
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
            .map_err(|_| CoreError::Invite("inviter did not respond in time — are they online?".into()))?
            .ok_or_else(|| CoreError::P2p("invite redemption channel closed".into()))?;

        self.pending_responses.lock().await.remove(&request_id);

        match response {
            InertiaResponse::Ok => Ok(()),
            InertiaResponse::Error(msg) => Err(CoreError::Invite(msg)),
            _ => Err(CoreError::Invite("unexpected response to invite redemption".into())),
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

    fn spawn_event_loop(&self) {
        let swarm = Arc::clone(&self.swarm);
        let store = self.store.clone();
        let identity = Arc::clone(&self.identity);
        let event_tx = self.event_tx.clone();
        let pending_responses = Arc::clone(&self.pending_responses);

        tokio::spawn(async move {
            loop {
                let event = {
                    let mut swarm = swarm.lock().await;
                    swarm.next().await
                };

                let Some(event) = event else {
                    break;
                };

                match event {
                    libp2p::swarm::SwarmEvent::ConnectionEstablished {
                        peer_id,
                        endpoint,
                        ..
                    } => {
                        info!(%peer_id, "peer connected");
                        let remote = endpoint.get_remote_address().to_string();
                        persist_peer_multiaddrs(&store, &peer_id, &[remote]).await;
                        let _ = event_tx.send(P2pEvent::PeerConnected(peer_id));
                        update_contact_state(&store, &peer_id, ConnectionState::Online).await;
                    }
                    libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        info!(%peer_id, "peer disconnected");
                        let _ = event_tx.send(P2pEvent::PeerDisconnected(peer_id));
                        update_contact_state(&store, &peer_id, ConnectionState::Offline).await;
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(
                        InertiaBehaviourEvent::RequestResponse(Event::Message {
                            peer,
                            message,
                        }),
                    ) => {
                        if let Message::Request { request, channel, .. } = message {
                            let response =
                                match handle_inbound_request(&store, &identity, &event_tx, request)
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
                            let _ = peer;
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
                        info!(%relay_peer_id, "relay reservation accepted");
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(
                        InertiaBehaviourEvent::Dcutr(dcutr::Event { remote_peer_id, result }),
                    ) => match result {
                        Ok(connection_id) => {
                            info!(%remote_peer_id, ?connection_id, "direct connection upgrade succeeded");
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
                        let addrs: Vec<String> =
                            info.listen_addrs.iter().map(|a| a.to_string()).collect();
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

async fn persist_peer_multiaddrs(store: &StoreHandle, peer_id: &PeerId, addrs: &[String]) {
    let peer_id = peer_id.to_string();
    let _ = store
        .with_mut(|s| s.merge_contact_multiaddrs_by_peer_id(&peer_id, addrs))
        .await;
}

async fn update_contact_state(store: &StoreHandle, peer_id: &PeerId, state: ConnectionState) {
    let peer_id = peer_id.to_string();
    let _ = store
        .with_mut(|store| {
            if let Ok(contacts) = store.list_contacts() {
                for mut c in contacts {
                    if c.peer_id.as_deref() == Some(&peer_id) {
                        c.connection_state = state;
                        c.last_seen = Some(chrono::Utc::now());
                        store.upsert_contact(&c)?;
                    }
                }
            }
            Ok(())
        })
        .await;
}

async fn handle_inbound_request(
    store: &StoreHandle,
    identity: &Arc<RwLock<Identity>>,
    event_tx: &mpsc::UnboundedSender<P2pEvent>,
    request: InertiaRequest,
) -> CoreResult<InertiaResponse> {
    match request {
        InertiaRequest::FriendRequest(req) => {
            let _ = event_tx.send(P2pEvent::FriendRequestReceived(req));
            Ok(InertiaResponse::Ok)
        }
        InertiaRequest::FriendAccept(accept) => {
            let contact = Contact {
                id: accept.contact_id.clone(),
                phone_hash: accept.phone_hash,
                display_name: accept.display_name,
                peer_id: Some(accept.peer_id),
                signing_pubkey: accept.signing_pubkey,
                encryption_pubkey: accept.encryption_pubkey,
                last_seen: Some(chrono::Utc::now()),
                connection_state: ConnectionState::Online,
                multiaddrs: Vec::new(),
            };
            store.with_mut(|s| s.upsert_contact(&contact)).await?;
            Ok(InertiaResponse::Ok)
        }
        InertiaRequest::InviteRedemption(redemption) => {
            store
                .with_mut(|s| s.consume_issued_invite(&redemption.invite_nonce))
                .await?;
            let contact = Contact {
                id: redemption.signing_pubkey.clone(),
                phone_hash: None,
                display_name: redemption.display_name,
                peer_id: Some(redemption.peer_id),
                signing_pubkey: redemption.signing_pubkey,
                encryption_pubkey: redemption.encryption_pubkey,
                last_seen: Some(chrono::Utc::now()),
                connection_state: ConnectionState::Online,
                multiaddrs: Vec::new(),
            };
            store.with_mut(|s| s.upsert_contact(&contact)).await?;
            info!(friend = %contact.display_name, "invite redeemed");
            let _ = identity;
            Ok(InertiaResponse::Ok)
        }
        InertiaRequest::SendEnvelope(SendEnvelope { envelope }) => {
            process_incoming_envelope(store, identity, event_tx, &envelope).await?;
            Ok(InertiaResponse::DeliveryAck(
                super::protocol::DeliveryAck {
                    content_id: envelope.id.clone(),
                },
            ))
        }
    }
}

async fn handle_outbound_response(
    store: &StoreHandle,
    event_tx: &mpsc::UnboundedSender<P2pEvent>,
    response: InertiaResponse,
) -> CoreResult<()> {
    match response {
        InertiaResponse::DeliveryAck(ack) => {
            let _ = event_tx.send(P2pEvent::DeliveryAcked {
                content_id: ack.content_id.clone(),
            });
            let content_id = ack.content_id.clone();
            store
                .with_mut(|s| {
                    let entries = s.list_outbox()?;
                    for entry in entries {
                        if entry.content_id == content_id {
                            s.update_outbox_status(
                                &content_id,
                                &entry.recipient_id,
                                DeliveryStatus::Delivered,
                            )?;
                            s.record_ack(&content_id, &entry.recipient_id)?;
                            break;
                        }
                    }
                    Ok(())
                })
                .await?;
        }
        InertiaResponse::FriendAccept(accept) => {
            let contact = Contact {
                id: accept.contact_id,
                phone_hash: accept.phone_hash,
                display_name: accept.display_name,
                peer_id: Some(accept.peer_id),
                signing_pubkey: accept.signing_pubkey,
                encryption_pubkey: accept.encryption_pubkey,
                last_seen: Some(chrono::Utc::now()),
                connection_state: ConnectionState::Online,
                multiaddrs: Vec::new(),
            };
            store.with_mut(|s| s.upsert_contact(&contact)).await?;
        }
        InertiaResponse::Ok => {}
        InertiaResponse::Error(msg) => {
            warn!(error = %msg, "peer returned error");
        }
    }
    Ok(())
}

async fn process_incoming_envelope(
    store: &StoreHandle,
    identity: &Arc<RwLock<Identity>>,
    event_tx: &mpsc::UnboundedSender<P2pEvent>,
    envelope: &ContentEnvelope,
) -> CoreResult<()> {
    if !crate::identity::Identity::verify_signature(
        &envelope.author_signing_pubkey,
        &envelope.signing_bytes(),
        &envelope.signature,
    )? {
        return Err(CoreError::Crypto("invalid envelope signature".into()));
    }

    let id = identity.read().await;
    let plaintext = decrypt_from_sender(
        id.encryption_secret()?,
        &envelope.author_encryption_pubkey,
        &envelope.ciphertext,
    )?;
    drop(id);

    let (body, media_ref) = match envelope.content_type {
        ContentType::Message => {
            let payload: MessagePayload = serde_json::from_slice(&plaintext)?;
            (payload.body, None)
        }
        ContentType::Post => {
            let payload: crate::content::PostPayload = serde_json::from_slice(&plaintext)?;
            (payload.body, payload.media_ref)
        }
        ContentType::Comment => {
            let payload: crate::content::CommentPayload = serde_json::from_slice(&plaintext)?;
            let sender_id = envelope.author_signing_pubkey.clone();
            let author_name = store
                .with(|s| s.list_contacts())
                .await
                .ok()
                .and_then(|contacts| {
                    contacts
                        .into_iter()
                        .find(|c| c.id == sender_id || c.signing_pubkey == sender_id)
                        .map(|c| c.display_name)
                })
                .unwrap_or_else(|| "Friend".to_string());

            let comment = crate::storage::PostComment {
                id: envelope.id.clone(),
                post_id: payload.post_id,
                author_id: sender_id,
                author_name,
                body: payload.body,
                created_at: envelope.created_at,
            };
            store
                .with_mut(|s| s.insert_post_comment(&comment))
                .await?;
            return Ok(());
        }
    };

    let sender_id = envelope.author_signing_pubkey.clone();
    store
        .with_mut(|s| {
            s.insert_inbox(&InboxEntry {
                content_id: envelope.id.clone(),
                sender_id: sender_id.clone(),
                received_at: envelope.created_at,
                expires_at: envelope.expires_at,
                read_at: None,
                body: body.clone(),
                media_ref: media_ref.clone(),
                content_type: envelope.content_type,
            })
        })
        .await?;

    if envelope.content_type == ContentType::Post {
        let author_name = store
            .with(|s| s.list_contacts())
            .await
            .ok()
            .and_then(|contacts| {
                contacts
                    .into_iter()
                    .find(|c| c.id == sender_id || c.signing_pubkey == sender_id)
                    .map(|c| c.display_name)
            })
            .unwrap_or_else(|| "Friend".to_string());

        let archive_item = crate::storage::ArchivedFeedItem {
            content_id: envelope.id.clone(),
            author_id: sender_id.clone(),
            author_name,
            body: body.clone(),
            media_ref: media_ref.clone(),
            created_at: envelope.created_at,
            is_own: false,
        };
        let _ = store
            .with_mut(|s| s.try_archive_feed_item(&archive_item))
            .await;
    }

    let _ = event_tx.send(P2pEvent::MessageReceived { sender_id, body });
    Ok(())
}

pub fn build_post_envelope(
    identity: &Identity,
    recipient: &Contact,
    body: &str,
    media_ref: Option<&str>,
) -> CoreResult<ContentEnvelope> {
    let payload = PostPayload {
        body: body.to_string(),
        media_ref: media_ref.map(|s| s.to_string()),
    };
    let plaintext = serde_json::to_vec(&payload)?;
    let ciphertext = encrypt_for_recipient(
        identity.encryption_secret()?,
        &recipient.encryption_pubkey,
        &plaintext,
    )?;

    let mut envelope = ContentEnvelope::new_post(
        identity.signing_pubkey.clone(),
        identity.encryption_pubkey.clone(),
        ciphertext,
        vec![],
    );
    envelope.signature = identity.sign(&envelope.signing_bytes())?;
    Ok(envelope)
}

pub fn build_message_envelope(
    identity: &Identity,
    recipient: &Contact,
    body: &str,
    thread_id: &str,
) -> CoreResult<ContentEnvelope> {
    let payload = MessagePayload {
        body: body.to_string(),
        thread_id: thread_id.to_string(),
    };
    let plaintext = serde_json::to_vec(&payload)?;
    let ciphertext = encrypt_for_recipient(
        identity.encryption_secret()?,
        &recipient.encryption_pubkey,
        &plaintext,
    )?;

    let mut envelope = ContentEnvelope::new_message(
        identity.signing_pubkey.clone(),
        identity.encryption_pubkey.clone(),
        ciphertext,
        vec![],
    );
    envelope.signature = identity.sign(&envelope.signing_bytes())?;
    Ok(envelope)
}

pub fn build_comment_envelope(
    identity: &Identity,
    recipient: &Contact,
    post_id: &str,
    body: &str,
) -> CoreResult<ContentEnvelope> {
    let payload = crate::content::CommentPayload {
        post_id: post_id.to_string(),
        body: body.to_string(),
    };
    let plaintext = serde_json::to_vec(&payload)?;
    let ciphertext = encrypt_for_recipient(
        identity.encryption_secret()?,
        &recipient.encryption_pubkey,
        &plaintext,
    )?;

    let mut envelope = ContentEnvelope::new_comment(
        identity.signing_pubkey.clone(),
        identity.encryption_pubkey.clone(),
        ciphertext,
        vec![],
    );
    envelope.signature = identity.sign(&envelope.signing_bytes())?;
    Ok(envelope)
}

fn peer_id_from_multiaddr(addr: &Multiaddr) -> CoreResult<PeerId> {
    addr.iter()
        .find_map(|protocol| match protocol {
            Protocol::P2p(peer_id) => Some(peer_id),
            _ => None,
        })
        .ok_or_else(|| CoreError::P2p("multiaddr missing /p2p peer id".into()))
}

fn relay_circuit_listen_addr(relay: &Multiaddr) -> Multiaddr {
    if relay.iter().any(|p| matches!(p, Protocol::P2pCircuit)) {
        relay.clone()
    } else {
        relay.clone().with(Protocol::P2pCircuit)
    }
}

fn is_routable_multiaddr(addr: &Multiaddr) -> bool {
    let raw = addr.to_string();
    !raw.contains("/ip4/0.0.0.0/") && !raw.contains("/ip6/::/")
}

fn ensure_peer_id_suffix(addr: &Multiaddr, peer_id: &str) -> String {
    let raw = addr.to_string();
    if raw.contains("/p2p/") {
        raw
    } else {
        format!("{raw}/p2p/{peer_id}")
    }
}
