use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use libp2p::Multiaddr;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

use crate::content::{ContentType, DeliveryStatus};
use crate::error::{CoreError, CoreResult};
use crate::expiry::ExpiryScheduler;
use crate::identity::Identity;
use crate::invite::FriendInvite;
use crate::p2p::{build_message_envelope, P2pEvent, P2pNode};
use crate::storage::{ConnectionState, Contact, OutboxEntry};
use crate::store_handle::StoreHandle;

/// High-level facade over storage, identity, expiry, and P2P networking.
pub struct Engine {
    pub store: StoreHandle,
    pub identity: Arc<RwLock<Identity>>,
    p2p: Option<P2pNode>,
    _expiry_handle: Option<tokio::task::JoinHandle<()>>,
    _p2p_events: mpsc::UnboundedReceiver<P2pEvent>,
    event_tx: mpsc::UnboundedSender<P2pEvent>,
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

        Ok(Self {
            store,
            identity,
            p2p: None,
            _expiry_handle: expiry_handle,
            _p2p_events: p2p_events,
            event_tx,
        })
    }

    pub async fn initialize_identity(
        &self,
        display_name: impl Into<String>,
    ) -> CoreResult<Identity> {
        {
            let current = self.identity.read().await;
            if current.is_initialized() {
                return Err(CoreError::ProfileAlreadyExists);
            }
        }

        if self.store.with(|s| s.has_profile()).await? {
            return Err(CoreError::ProfileAlreadyExists);
        }

        let identity = Identity::generate(display_name);
        self.store
            .with_mut(|store| store.create_identity(&identity))
            .await?;
        *self.identity.write().await = identity.clone();
        info!(display_name = %identity.display_name, "identity initialized");
        Ok(identity)
    }

    pub async fn identity_snapshot(&self) -> Identity {
        self.identity.read().await.clone()
    }

    pub async fn create_invite(&self, web_origin: Option<&str>) -> CoreResult<InviteResponse> {
        let identity = self.identity.read().await;
        if identity.display_name.is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }
        drop(identity);

        let peer_id = self.peer_id().await;
        let multiaddrs = self.p2p_listen_addresses().await.unwrap_or_default();
        let identity = self.identity.read().await;
        let invite = FriendInvite::new(&identity, peer_id, multiaddrs)?;
        drop(identity);

        self.store
            .with_mut(|store| {
                store.register_issued_invite(&invite.nonce, invite.expires_at)
            })
            .await?;

        let payload = invite.to_payload()?;
        let link = invite.to_link(web_origin)?;
        let safety_code = invite.safety_code();
        Ok(InviteResponse {
            link,
            payload,
            safety_code,
            expires_at: invite.expires_at,
            display_name: invite.display_name,
        })
    }

    pub async fn preview_invite(&self, input: &str) -> CoreResult<InvitePreview> {
        let invite = FriendInvite::parse(input)?;
        let safety_code = invite.safety_code();
        Ok(InvitePreview {
            display_name: invite.display_name,
            signing_pubkey: invite.signing_pubkey,
            safety_code,
            expires_at: invite.expires_at,
            peer_id: invite.peer_id,
            multiaddrs: invite.multiaddrs,
        })
    }

    pub async fn accept_invite(&self, input: &str) -> CoreResult<Contact> {
        let invite = FriendInvite::parse(input)?;

        if self
            .store
            .with(|store| store.is_invite_redeemed_locally(&invite.nonce))
            .await?
        {
            return Err(CoreError::Invite(
                "you already accepted this invite on this device".into(),
            ));
        }

        let p2p = self
            .p2p
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;

        let peer_id_str = invite.peer_id.as_ref().ok_or_else(|| {
            CoreError::Invite(
                "inviter is not reachable — they must be online with P2P running".into(),
            )
        })?;

        if invite.multiaddrs.is_empty() {
            return Err(CoreError::Invite(
                "inviter has no connection addresses — they must be online".into(),
            ));
        }

        for addr_str in &invite.multiaddrs {
            if let Ok(addr) = addr_str.parse() {
                let _ = p2p.dial(addr).await;
            }
        }

        let peer_id = peer_id_str
            .parse::<libp2p::PeerId>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;

        let identity = self.identity.read().await;
        let redemption = crate::p2p::InviteRedemption {
            invite_nonce: invite.nonce.clone(),
            display_name: identity.display_name.clone(),
            signing_pubkey: identity.signing_pubkey.clone(),
            encryption_pubkey: identity.encryption_pubkey.clone(),
            peer_id: p2p.peer_id_string(),
        };
        drop(identity);

        p2p.redeem_invite(peer_id, redemption).await?;

        let contact = invite.to_contact();
        let nonce = invite.nonce.clone();
        let issuer = invite.signing_pubkey.clone();
        self.store
            .with_mut(|store| {
                store.upsert_contact(&contact)?;
                store.mark_invite_redeemed_locally(&nonce, &issuer)
            })
            .await?;

        info!(friend = %contact.display_name, "invite accepted");
        Ok(contact)
    }

    #[allow(dead_code)]
    pub async fn add_pending_contact(
        &self,
        contact_id: &str,
        display_name: &str,
        signing_pubkey: &str,
        encryption_pubkey: &str,
    ) -> CoreResult<Contact> {
        let contact = Contact {
            id: contact_id.to_string(),
            phone_hash: None,
            display_name: display_name.to_string(),
            peer_id: None,
            signing_pubkey: signing_pubkey.to_string(),
            encryption_pubkey: encryption_pubkey.to_string(),
            last_seen: None,
            connection_state: ConnectionState::Offline,
        };
        self.store
            .with_mut(|store| store.upsert_contact(&contact))
            .await?;
        Ok(contact)
    }

    pub async fn list_contacts(&self) -> CoreResult<Vec<Contact>> {
        self.store.with(|store| store.list_contacts()).await
    }

    pub async fn list_outbox(&self) -> CoreResult<Vec<OutboxEntry>> {
        self.store.with(|store| store.list_outbox()).await
    }

    pub async fn list_inbox(&self) -> CoreResult<Vec<crate::storage::InboxEntry>> {
        self.store.with(|store| store.list_inbox()).await
    }

    pub async fn start_p2p(&mut self, listen_port: u16) -> CoreResult<String> {
        let listen_addr = format!("/ip4/0.0.0.0/tcp/{listen_port}")
            .parse::<Multiaddr>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;

        let node = P2pNode::start(
            self.store.clone(),
            Arc::clone(&self.identity),
            listen_addr,
            self.event_tx.clone(),
        )
        .await?;

        let peer_id = node.peer_id_string();
        self.p2p = Some(node);
        info!(%peer_id, "p2p node started");
        Ok(peer_id)
    }

    pub async fn peer_id(&self) -> Option<String> {
        self.p2p.as_ref().map(|n| n.peer_id_string())
    }

    pub async fn p2p_listen_addresses(&self) -> CoreResult<Vec<String>> {
        let p2p = self
            .p2p
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        Ok(p2p
            .listen_addresses()
            .await
            .into_iter()
            .map(|a| a.to_string())
            .collect())
    }

    pub async fn dial_peer(&self, multiaddr: &str) -> CoreResult<()> {
        let p2p = self
            .p2p
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let addr = multiaddr
            .parse::<Multiaddr>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;
        p2p.dial(addr).await
    }

    pub async fn send_friend_request(&self, contact_id: &str, multiaddr: &str) -> CoreResult<()> {
        let p2p = self
            .p2p
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let addr = multiaddr
            .parse::<Multiaddr>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;
        p2p.send_friend_request(contact_id, addr).await
    }

    pub async fn send_message(&self, recipient_id: &str, body: &str) -> CoreResult<String> {
        let identity = self.identity.read().await;
        let recipient = self
            .store
            .with(|store| store.get_contact(recipient_id))
            .await?;

        let envelope = build_message_envelope(&identity, &recipient, body, recipient_id)?;
        drop(identity);

        let envelope_json = serde_json::to_string(&envelope)?;
        let content_id = envelope.id.clone();

        self.store
            .with_mut(|store| {
                store.insert_outbox(
                    &OutboxEntry {
                        content_id: content_id.clone(),
                        recipient_id: recipient_id.to_string(),
                        status: DeliveryStatus::Pending,
                        expires_at: envelope.expires_at,
                        retry_count: 0,
                        ciphertext: envelope.ciphertext.clone(),
                        content_type: ContentType::Message,
                    },
                    &envelope_json,
                )
            })
            .await?;

        if let (Some(p2p), Some(peer_id_str)) = (&self.p2p, recipient.peer_id.as_ref()) {
            if let Ok(peer_id) = peer_id_str.parse() {
                if p2p.send_envelope_to_peer(peer_id, envelope).await.is_ok() {
                    return Ok(content_id);
                }
            }
        }

        self.store
            .with_mut(|store| {
                store.update_outbox_status(&content_id, recipient_id, DeliveryStatus::Failed)
            })
            .await?;

        Ok(content_id)
    }

    pub async fn retry_outbox(&self, content_id: &str, recipient_id: &str) -> CoreResult<()> {
        let envelope_json = self
            .store
            .with(|store| store.get_outbox_envelope(content_id, recipient_id))
            .await?;
        let recipient = self
            .store
            .with(|store| store.get_contact(recipient_id))
            .await?;

        let envelope: crate::content::ContentEnvelope = serde_json::from_str(&envelope_json)?;

        let p2p = self
            .p2p
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let peer_id = recipient
            .peer_id
            .as_ref()
            .ok_or_else(|| CoreError::P2p("recipient has no peer id".into()))?
            .parse::<libp2p::PeerId>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;

        self.store
            .with_mut(|store| store.increment_outbox_retry(content_id, recipient_id))
            .await?;

        match p2p.send_envelope_to_peer(peer_id, envelope).await {
            Ok(()) => Ok(()),
            Err(e) => {
                self.store
                    .with_mut(|store| {
                        store.update_outbox_status(content_id, recipient_id, DeliveryStatus::Failed)
                    })
                    .await?;
                Err(e)
            }
        }
    }

    pub async fn run_expiry_sweep(&self) -> CoreResult<crate::storage::PurgeReport> {
        self.store.with(|store| store.purge_expired()).await
    }
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
}
