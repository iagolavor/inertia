use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use libp2p::{Multiaddr, PeerId};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{info, warn};

use crate::content::{ContentEnvelope, ContentType, DeliveryStatus};
use crate::error::{CoreError, CoreResult};
use crate::expiry::ExpiryScheduler;
use crate::identity::Identity;
use crate::invite::FriendInvite;
use crate::p2p::{build_comment_envelope, build_message_envelope, build_post_envelope, P2pEvent, P2pNode};
use crate::storage::{
    AppSettings, ArchivedFeedItem, ConnectionState, Contact, FeedBackup, FeedItem,
    FeedRestoreReport, LocalPost, OutboxEntry, PostComment, ProfilePhoto,
};
use crate::store_handle::StoreHandle;

/// Default libp2p TCP listen port when `INERTIA_P2P_LISTEN_PORT` is unset.
pub const DEFAULT_P2P_LISTEN_PORT: u16 = 4784;

/// High-level facade over storage, identity, expiry, and P2P networking.
pub struct Engine {
    pub store: StoreHandle,
    pub identity: Arc<RwLock<Identity>>,
    p2p: Arc<Mutex<Option<P2pNode>>>,
    _expiry_handle: Option<tokio::task::JoinHandle<()>>,
    _p2p_event_task: tokio::task::JoinHandle<()>,
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
        let p2p = Arc::new(Mutex::new(None));
        let p2p_for_events = Arc::clone(&p2p);
        let store_for_events = store.clone();
        let p2p_event_task = tokio::spawn(async move {
            run_p2p_event_loop(p2p_events, store_for_events, p2p_for_events).await;
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
        self.ensure_p2p_started().await?;
        Ok(identity)
    }

    pub async fn update_profile(
        &self,
        display_name: impl Into<String>,
        bio: impl Into<String>,
    ) -> CoreResult<Identity> {
        let display_name = display_name.into();
        let bio = bio.into();
        if display_name.trim().is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }

        {
            let current = self.identity.read().await;
            if !current.is_initialized() {
                return Err(CoreError::IdentityNotInitialized);
            }
        }

        self.store
            .with_mut(|store| store.update_identity_profile(&display_name, &bio))
            .await?;

        let mut identity = self.identity.write().await;
        identity.display_name = display_name.trim().to_string();
        identity.bio = bio.trim().to_string();
        Ok(identity.clone())
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
        let multiaddrs = self.p2p_invite_addresses(peer_id.as_deref()).await;
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

        let p2p_guard = self.p2p.lock().await;
        let p2p = p2p_guard
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
            multiaddrs: Vec::new(),
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

    /// Idempotent — returns the current peer id if P2P is already running.
    pub async fn ensure_p2p_started(&self) -> CoreResult<String> {
        self.start_p2p(0).await
    }

    pub async fn start_p2p(&self, listen_port: u16) -> CoreResult<String> {
        let listen_port = if listen_port == 0 {
            p2p_listen_port_from_env()
        } else {
            listen_port
        };

        let mut guard = self.p2p.lock().await;
        if let Some(node) = guard.as_ref() {
            return Ok(node.peer_id_string());
        }

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
        *guard = Some(node);
        drop(guard);

        if let Err(e) = self.redial_known_peers().await {
            warn!(error = %e, "redial known peers failed");
        }

        info!(%peer_id, port = listen_port, "p2p node started");
        Ok(peer_id)
    }

    /// Dial configured relay (if any) and stored contact addresses after P2P starts.
    pub async fn redial_known_peers(&self) -> CoreResult<()> {
        if let Some(relay) = std::env::var("INERTIA_RELAY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            match self.dial_peer(&relay).await {
                Ok(()) => info!("dialed configured relay"),
                Err(e) => warn!(error = %e, "failed to dial relay"),
            }
        }

        let contacts = self.list_contacts().await?;
        for contact in contacts {
            if contact.multiaddrs.is_empty() {
                continue;
            }
            for addr in &contact.multiaddrs {
                if let Err(e) = self.dial_peer(addr).await {
                    warn!(
                        friend = %contact.display_name,
                        address = %addr,
                        error = %e,
                        "failed to redial contact"
                    );
                }
            }
        }
        Ok(())
    }

    pub async fn peer_id(&self) -> Option<String> {
        self.p2p.lock().await.as_ref().map(|n| n.peer_id_string())
    }

    pub async fn p2p_listen_addresses(&self) -> CoreResult<Vec<String>> {
        let guard = self.p2p.lock().await;
        let p2p = guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        Ok(p2p
            .listen_addresses()
            .await
            .into_iter()
            .map(|a| a.to_string())
            .collect())
    }

    pub async fn p2p_status(&self) -> P2pStatus {
        let guard = self.p2p.lock().await;
        if let Some(p2p) = guard.as_ref() {
            P2pStatus {
                running: true,
                peer_id: Some(p2p.peer_id_string()),
                listen_addresses: p2p
                    .listen_addresses()
                    .await
                    .into_iter()
                    .map(|a| a.to_string())
                    .collect(),
                connected_peer_ids: p2p.connected_peer_ids().await,
            }
        } else {
            P2pStatus {
                running: false,
                peer_id: None,
                listen_addresses: Vec::new(),
                connected_peer_ids: Vec::new(),
            }
        }
    }

    /// Addresses embedded in invites — uses `INERTIA_P2P_ANNOUNCE` when set.
    pub async fn p2p_invite_addresses(&self, peer_id: Option<&str>) -> Vec<String> {
        if let Some(pid) = peer_id {
            let announced = announced_p2p_multiaddrs(pid);
            if !announced.is_empty() {
                return announced;
            }
        }
        self.p2p_listen_addresses().await.unwrap_or_default()
    }

    pub async fn dial_peer(&self, multiaddr: &str) -> CoreResult<()> {
        let guard = self.p2p.lock().await;
        let p2p = guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let addr = multiaddr
            .parse::<Multiaddr>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;
        p2p.dial(addr).await
    }

    pub async fn send_friend_request(&self, contact_id: &str, multiaddr: &str) -> CoreResult<()> {
        let guard = self.p2p.lock().await;
        let p2p = guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let addr = multiaddr
            .parse::<Multiaddr>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;
        p2p.send_friend_request(contact_id, addr).await
    }

    pub async fn send_post(
        &self,
        body: &str,
        media_ref: Option<&str>,
    ) -> CoreResult<String> {
        let identity = self.identity.read().await;
        if identity.display_name.is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }
        let display_name = identity.display_name.clone();
        let signing_pubkey = identity.signing_pubkey.clone();

        let contacts = self.store.with(|store| store.list_contacts()).await?;
        let content_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::seconds(crate::identity::POST_TTL_SECS);

        let local_post = LocalPost {
            content_id: content_id.clone(),
            body: body.to_string(),
            media_ref: media_ref.map(|s| s.to_string()),
            created_at: now,
            expires_at,
        };

        self.store
            .with_mut(|store| store.insert_local_post(&local_post))
            .await?;

        let archive_item = ArchivedFeedItem {
            content_id: content_id.clone(),
            author_id: signing_pubkey.clone(),
            author_name: display_name.clone(),
            body: body.to_string(),
            media_ref: media_ref.map(|s| s.to_string()),
            created_at: now,
            is_own: true,
        };
        self.store
            .with_mut(|store| store.try_archive_feed_item(&archive_item))
            .await?;

        for contact in &contacts {
            let contact_envelope =
                build_post_envelope(&identity, contact, body, media_ref)?;
            let envelope_id = contact_envelope.id.clone();
            let contact_envelope_json = serde_json::to_string(&contact_envelope)?;

            self.store
                .with_mut(|store| {
                    store.insert_outbox(
                        &OutboxEntry {
                            content_id: envelope_id,
                            recipient_id: contact.id.clone(),
                            status: DeliveryStatus::Pending,
                            expires_at: contact_envelope.expires_at,
                            retry_count: 0,
                            ciphertext: contact_envelope.ciphertext.clone(),
                            content_type: ContentType::Post,
                        },
                        &contact_envelope_json,
                    )
                })
                .await?;

            if let Some(peer_id_str) = contact.peer_id.as_ref() {
                if let Ok(peer_id) = peer_id_str.parse() {
                    let p2p_guard = self.p2p.lock().await;
                    if let Some(p2p) = p2p_guard.as_ref() {
                        let _ = p2p
                            .send_envelope_to_peer(peer_id, contact_envelope)
                            .await;
                    }
                }
            }
        }

        drop(identity);
        info!(%content_id, recipients = contacts.len(), "post saved; outbox pending");
        Ok(content_id)
    }

    pub async fn list_feed(&self) -> CoreResult<Vec<FeedItem>> {
        let settings = self.store.with(|store| store.get_settings()).await?;
        let ephemeral = self.collect_ephemeral_feed_items().await?;

        let mut items = if settings.feed_history_enabled {
            let archived = self
                .store
                .with(|store| store.list_feed_archive())
                .await?
                .into_iter()
                .map(|item| item.to_feed_item())
                .collect::<Vec<_>>();
            let archived_ids: HashSet<String> =
                archived.iter().map(|item| item.content_id.clone()).collect();
            let mut merged = archived;
            for item in ephemeral {
                if !archived_ids.contains(&item.content_id) {
                    merged.push(item);
                }
            }
            merged
        } else {
            ephemeral
        };

        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        for item in &mut items {
            item.comment_count = self
                .store
                .with(|store| store.count_post_comments(&item.content_id))
                .await
                .unwrap_or(0);
        }

        Ok(items)
    }

    pub async fn get_feed_item(&self, content_id: &str) -> CoreResult<Option<FeedItem>> {
        let items = self.list_feed().await?;
        Ok(items.into_iter().find(|item| item.content_id == content_id))
    }

    pub async fn list_post_comments(&self, post_id: &str) -> CoreResult<Vec<PostComment>> {
        self.store
            .with(|store| store.list_post_comments(post_id))
            .await
    }

    pub async fn add_post_comment(&self, post_id: &str, body: &str) -> CoreResult<PostComment> {
        let body = body.trim();
        if body.is_empty() {
            return Err(CoreError::Invite("comment cannot be empty".into()));
        }

        let identity = self.identity.read().await;
        if identity.display_name.is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }

        let comment = PostComment {
            id: uuid::Uuid::new_v4().to_string(),
            post_id: post_id.to_string(),
            author_id: identity.signing_pubkey.clone(),
            author_name: identity.display_name.clone(),
            body: body.to_string(),
            created_at: chrono::Utc::now(),
        };
        drop(identity);

        self.store
            .with_mut(|store| store.insert_post_comment(&comment))
            .await?;

        let contacts = self.store.with(|store| store.list_contacts()).await?;
        let identity = self.identity.read().await;

        for contact in &contacts {
            let contact_envelope =
                build_comment_envelope(&identity, contact, post_id, body)?;
            let envelope_id = contact_envelope.id.clone();
            let contact_envelope_json = serde_json::to_string(&contact_envelope)?;

            self.store
                .with_mut(|store| {
                    store.insert_outbox(
                        &OutboxEntry {
                            content_id: envelope_id,
                            recipient_id: contact.id.clone(),
                            status: DeliveryStatus::Pending,
                            expires_at: contact_envelope.expires_at,
                            retry_count: 0,
                            ciphertext: contact_envelope.ciphertext.clone(),
                            content_type: ContentType::Comment,
                        },
                        &contact_envelope_json,
                    )
                })
                .await?;

            if let Some(peer_id_str) = contact.peer_id.as_ref() {
                if let Ok(peer_id) = peer_id_str.parse() {
                    let p2p_guard = self.p2p.lock().await;
                    if let Some(p2p) = p2p_guard.as_ref() {
                        let _ = p2p
                            .send_envelope_to_peer(peer_id, contact_envelope)
                            .await;
                    }
                }
            }
        }

        drop(identity);
        info!(post_id, "comment saved and queued for peers");
        Ok(comment)
    }

    async fn collect_ephemeral_feed_items(&self) -> CoreResult<Vec<FeedItem>> {
        let identity = self.identity.read().await;
        let display_name = identity.display_name.clone();
        let signing_pubkey = identity.signing_pubkey.clone();
        drop(identity);

        let contacts = self.store.with(|store| store.list_contacts()).await?;
        let contact_names: std::collections::HashMap<String, String> = contacts
            .iter()
            .flat_map(|c| {
                [
                    (c.id.clone(), c.display_name.clone()),
                    (c.signing_pubkey.clone(), c.display_name.clone()),
                ]
            })
            .collect();

        let local_posts = self
            .store
            .with(|store| store.list_local_posts())
            .await?;
        let inbox_posts = self
            .store
            .with(|store| store.list_inbox_posts())
            .await?;

        let mut items: Vec<FeedItem> = local_posts
            .into_iter()
            .map(|p| FeedItem {
                content_id: p.content_id,
                author_id: signing_pubkey.clone(),
                author_name: display_name.clone(),
                body: p.body,
                media_ref: p.media_ref,
                created_at: p.created_at,
                expires_at: p.expires_at,
                is_own: true,
                is_archived: false,
                comment_count: 0,
            })
            .collect();

        for entry in inbox_posts {
            items.push(FeedItem {
                content_id: entry.content_id,
                author_id: entry.sender_id.clone(),
                author_name: contact_names
                    .get(&entry.sender_id)
                    .cloned()
                    .unwrap_or_else(|| "Friend".to_string()),
                body: entry.body,
                media_ref: entry.media_ref,
                created_at: entry.received_at,
                expires_at: entry.expires_at,
                is_own: false,
                is_archived: false,
                comment_count: 0,
            });
        }

        Ok(items)
    }

    pub async fn get_settings(&self) -> CoreResult<AppSettings> {
        self.store.with(|store| store.get_settings()).await
    }

    pub async fn set_feed_history_enabled(&self, enabled: bool) -> CoreResult<AppSettings> {
        self.store
            .with_mut(|store| store.set_feed_history_enabled(enabled))
            .await?;

        if enabled {
            let ephemeral = self.collect_ephemeral_feed_items().await?;
            for item in ephemeral {
                let archived = ArchivedFeedItem::from(&item);
                self.store
                    .with_mut(|store| store.upsert_feed_archive(&archived))
                    .await?;
            }
        }

        self.get_settings().await
    }

    pub async fn export_feed_backup(&self) -> CoreResult<FeedBackup> {
        self.store.with(|store| store.export_feed_backup()).await
    }

    pub async fn import_feed_backup(&self, backup: FeedBackup) -> CoreResult<FeedRestoreReport> {
        let report = self
            .store
            .with_mut(|store| store.import_feed_backup(&backup))
            .await?;
        if !self.get_settings().await?.feed_history_enabled {
            self.set_feed_history_enabled(true).await?;
        }
        Ok(report)
    }

    pub async fn list_profile_photos(&self) -> CoreResult<Vec<ProfilePhoto>> {
        self.store.with(|store| store.list_profile_photos()).await
    }

    pub async fn add_profile_photo(
        &self,
        data: &[u8],
        caption: Option<&str>,
    ) -> CoreResult<ProfilePhoto> {
        let blob_hash = self.store_blob(data).await?;
        self.insert_profile_photo_record(blob_hash, caption, None).await
    }

    pub async fn publish_profile_photo(
        &self,
        data: &[u8],
        caption: Option<&str>,
    ) -> CoreResult<PublishPhotoResult> {
        let blob_hash = self.store_blob(data).await?;
        let photo = self
            .insert_profile_photo_record(blob_hash.clone(), caption, None)
            .await?;
        let body = caption.unwrap_or("");
        let content_id = self.send_post(body, Some(&blob_hash)).await?;
        self.store
            .with_mut(|store| store.update_profile_photo_content_id(&photo.id, &content_id))
            .await?;
        let mut photo = photo;
        photo.content_id = Some(content_id.clone());
        Ok(PublishPhotoResult { photo, content_id })
    }

    async fn insert_profile_photo_record(
        &self,
        blob_hash: String,
        caption: Option<&str>,
        content_id: Option<String>,
    ) -> CoreResult<ProfilePhoto> {
        let photos = self
            .store
            .with(|store| store.list_profile_photos())
            .await?;

        let photo = ProfilePhoto {
            id: uuid::Uuid::new_v4().to_string(),
            blob_hash,
            caption: caption.map(|s| s.to_string()),
            content_id,
            sort_order: photos.len() as i32,
            created_at: chrono::Utc::now(),
        };

        self.store
            .with_mut(|store| store.insert_profile_photo(&photo))
            .await?;

        Ok(photo)
    }

    pub async fn read_blob(&self, hash: &str) -> CoreResult<Vec<u8>> {
        self.store.with(|store| store.read_blob(hash)).await
    }

    pub async fn store_blob(&self, data: &[u8]) -> CoreResult<String> {
        self.store.with_mut(|store| store.store_blob(data)).await
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

        if let Some(peer_id_str) = recipient.peer_id.as_ref() {
            if let Ok(peer_id) = peer_id_str.parse() {
                let p2p_guard = self.p2p.lock().await;
                if let Some(p2p) = p2p_guard.as_ref() {
                    if p2p.send_envelope_to_peer(peer_id, envelope).await.is_ok() {
                        return Ok(content_id);
                    }
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
        deliver_outbox_entry(
            &self.store,
            &self.p2p,
            content_id,
            recipient_id,
            true,
        )
        .await
    }

    pub async fn run_expiry_sweep(&self) -> CoreResult<crate::storage::PurgeReport> {
        self.store.with(|store| store.purge_expired()).await
    }
}

pub fn default_p2p_listen_port() -> u16 {
    p2p_listen_port_from_env()
}

fn p2p_listen_port_from_env() -> u16 {
    std::env::var("INERTIA_P2P_LISTEN_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&port| port > 0)
        .unwrap_or(DEFAULT_P2P_LISTEN_PORT)
}

async fn run_p2p_event_loop(
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
        }
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

async fn deliver_outbox_entry(
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

/// Comma-separated multiaddrs from `INERTIA_P2P_ANNOUNCE`, with `/p2p/<peer_id>` appended when missing.
fn announced_p2p_multiaddrs(peer_id: &str) -> Vec<String> {
    let Some(raw) = std::env::var("INERTIA_P2P_ANNOUNCE").ok() else {
        return Vec::new();
    };
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|addr| {
            if addr.contains("/p2p/") {
                addr.to_string()
            } else {
                format!("{addr}/p2p/{peer_id}")
            }
        })
        .collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct P2pStatus {
    pub running: bool,
    pub peer_id: Option<String>,
    pub listen_addresses: Vec<String>,
    pub connected_peer_ids: Vec<String>,
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
}
