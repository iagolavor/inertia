use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use libp2p::request_response::{self, Event, Message, OutboundRequestId};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{identify, multiaddr::Protocol, noise, tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::content::{ContentEnvelope, ContentType, DeliveryStatus, MessagePayload};
use crate::crypto::{decrypt_from_sender, encrypt_for_recipient};
use crate::error::{CoreError, CoreResult};
use crate::identity::Identity;
use crate::storage::{ConnectionState, Contact, InboxEntry};
use crate::store_handle::StoreHandle;

use super::codec::{protocol_stream, request_response_config, JsonCodec};
use super::protocol::{
    FriendRequest, InertiaRequest, InertiaResponse, InviteRedemption, SendEnvelope,
};

#[derive(NetworkBehaviour)]
pub struct InertiaBehaviour {
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
        event_tx: mpsc::UnboundedSender<P2pEvent>,
    ) -> CoreResult<Self> {
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = local_key.public().to_peer_id();

        let behaviour = InertiaBehaviour {
            request_response: request_response::Behaviour::new(
                [(protocol_stream(), request_response::ProtocolSupport::Full)],
                request_response_config(),
            ),
            identify: identify::Behaviour::new(identify::Config::new(
                "/inertia/1.0.0".into(),
                local_key.public(),
            )),
        };

        let mut swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| CoreError::P2p(e.to_string()))?
            .with_behaviour(|_| behaviour)
            .map_err(|e| CoreError::P2p(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
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
                    StreamExt::next(&mut *swarm).now_or_never().flatten()
                };

                let Some(event) = event else {
                    tokio::task::yield_now().await;
                    continue;
                };

                match event {
                    libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        info!(%peer_id, "peer connected");
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
                    _ => debug!("swarm event"),
                }
            }
        });
    }
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

    let body = match envelope.content_type {
        ContentType::Message => {
            let payload: MessagePayload = serde_json::from_slice(&plaintext)?;
            payload.body
        }
        ContentType::Post => {
            let payload: crate::content::PostPayload = serde_json::from_slice(&plaintext)?;
            payload.body
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
                content_type: envelope.content_type,
            })
        })
        .await?;

    let _ = event_tx.send(P2pEvent::MessageReceived { sender_id, body });
    Ok(())
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

fn peer_id_from_multiaddr(addr: &Multiaddr) -> CoreResult<PeerId> {
    addr.iter()
        .find_map(|protocol| match protocol {
            Protocol::P2p(peer_id) => Some(peer_id),
            _ => None,
        })
        .ok_or_else(|| CoreError::P2p("multiaddr missing /p2p peer id".into()))
}
