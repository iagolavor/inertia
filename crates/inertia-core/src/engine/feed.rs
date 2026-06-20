use std::collections::HashSet;

use tracing::info;

use crate::content::{ContentType, DeliveryStatus};
use crate::error::{CoreError, CoreResult};
use crate::p2p::{build_comment_envelope, build_post_envelope};
use crate::storage::{
    ArchivedFeedItem, FeedItem, LocalPost, OutboxEntry, PostComment,
};

use super::Engine;

impl Engine {
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
                build_post_envelope(&identity, contact, &content_id, body, media_ref)?;
            let contact_envelope_json = serde_json::to_string(&contact_envelope)?;

            self.store
                .with_mut(|store| {
                    store.insert_outbox(
                        &OutboxEntry {
                            content_id: content_id.clone(),
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

    pub(super) async fn collect_ephemeral_feed_items(&self) -> CoreResult<Vec<FeedItem>> {
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
}
