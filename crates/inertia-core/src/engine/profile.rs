use tracing::info;

use crate::content::{ContentType, DeliveryStatus};
use crate::error::{CoreError, CoreResult};
use crate::p2p::build_profile_comment_envelope;
use crate::storage::{
    ArchiveEntry, ArchiveFolder, ArchiveFolderSummary, ArchiveUpload, ArchiveUploadStatus,
    OutboxEntry, ProfileComment, ProfileItem, ProfileManifest, ProfilePhoto, CHUNK_SIZE,
};

use super::{Engine, PublishPhotoResult};

impl Engine {
    pub async fn list_profile_photos(&self) -> CoreResult<Vec<ProfilePhoto>> {
        self.store.with(|store| store.list_profile_items()).await
    }

    pub async fn list_profile_items(&self) -> CoreResult<Vec<ProfileItem>> {
        self.store.with(|store| store.list_profile_items()).await
    }

    pub async fn add_profile_photo(
        &self,
        data: &[u8],
        caption: Option<&str>,
    ) -> CoreResult<ProfilePhoto> {
        let blob_hash = self.store_blob(data).await?;
        self.insert_profile_item_record(blob_hash, caption, None)
            .await
    }

    pub async fn publish_profile_photo(
        &self,
        data: &[u8],
        caption: Option<&str>,
    ) -> CoreResult<PublishPhotoResult> {
        let blob_hash = self.store_blob(data).await?;
        let photo = self
            .insert_profile_item_record(blob_hash.clone(), caption, None)
            .await?;
        let body = caption.unwrap_or("");
        // Ephemeral feed announcement (7d); profile item stays permanent.
        let content_id = self.send_post(body, Some(&blob_hash)).await?;
        self.store
            .with_mut(|store| store.update_profile_item_content_id(&photo.id, &content_id))
            .await?;
        let mut photo = photo;
        photo.content_id = Some(content_id.clone());
        Ok(PublishPhotoResult { photo, content_id })
    }

    pub async fn delete_profile_photo(&self, item_id: &str) -> CoreResult<()> {
        let removed = self
            .store
            .with_mut(|store| store.delete_profile_item(item_id))
            .await?;
        if !removed {
            return Err(CoreError::ContentNotFound(format!(
                "profile photo {item_id}"
            )));
        }
        Ok(())
    }

    async fn insert_profile_item_record(
        &self,
        blob_hash: String,
        caption: Option<&str>,
        content_id: Option<String>,
    ) -> CoreResult<ProfilePhoto> {
        let items = self
            .store
            .with(|store| store.list_profile_items())
            .await?;

        let photo = ProfilePhoto {
            id: uuid::Uuid::new_v4().to_string(),
            blob_hash,
            caption: caption.map(|s| s.to_string()),
            content_id,
            sort_order: items.len() as i32,
            created_at: chrono::Utc::now(),
        };

        self.store
            .with_mut(|store| store.insert_profile_item(&photo))
            .await?;

        Ok(photo)
    }

    /// Build the local author's profile manifest for P2P serving.
    pub async fn build_own_profile_manifest(&self) -> CoreResult<ProfileManifest> {
        let identity = self.identity.read().await;
        if !identity.is_initialized() {
            return Err(CoreError::IdentityNotInitialized);
        }
        let display_name = identity.display_name.clone();
        let bio = identity.bio.clone();
        let avatar_blob_hash = identity.avatar_blob_hash.clone();
        let signing_pubkey = identity.signing_pubkey.clone();
        drop(identity);

        let items = self.list_profile_items().await?;
        let archive_folders = self
            .store
            .with(|store| store.list_archive_folder_summaries())
            .await?;

        Ok(ProfileManifest {
            display_name,
            bio,
            avatar_blob_hash,
            signing_pubkey,
            items,
            archive_folders,
        })
    }

    pub async fn list_profile_comments(
        &self,
        profile_item_id: &str,
    ) -> CoreResult<Vec<ProfileComment>> {
        self.store
            .with(|store| store.list_profile_comments(profile_item_id))
            .await
    }

    /// Fetch a friend's live profile manifest over P2P (friend must be online).
    pub async fn fetch_friend_profile(&self, contact_id: &str) -> CoreResult<ProfileManifest> {
        let contact = self.get_contact(contact_id).await?;
        let peer_id_str = contact.peer_id.as_ref().ok_or_else(|| {
            CoreError::P2p(format!("friend {contact_id} has no peer id (offline)"))
        })?;
        let peer_id: libp2p::PeerId = peer_id_str
            .parse()
            .map_err(|e| CoreError::P2p(format!("invalid peer id: {e}")))?;

        let p2p_guard = self.p2p.lock().await;
        let node = p2p_guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let connected = node.connected_peer_ids().await;
        if !connected.iter().any(|id| id == peer_id_str) {
            return Err(CoreError::P2p(format!(
                "friend {contact_id} is offline"
            )));
        }
        let manifest = node.request_profile_manifest_from_peer(peer_id).await?;
        drop(p2p_guard);

        // Auto-fetch profile photo blobs (and avatar) for a smooth visit.
        let mut hashes: Vec<String> = manifest
            .items
            .iter()
            .map(|item| item.blob_hash.clone())
            .collect();
        if let Some(avatar) = &manifest.avatar_blob_hash {
            hashes.push(avatar.clone());
        }
        for hash in hashes {
            if let Err(e) = self.fetch_blob_from_contact(contact_id, &hash).await {
                tracing::warn!(%hash, %contact_id, error = %e, "profile blob fetch failed");
            }
        }

        Ok(manifest)
    }

    /// Pull a single blob from a friend when missing locally (profile thumbs, avatar).
    pub async fn fetch_blob_from_contact(
        &self,
        contact_id: &str,
        hash: &str,
    ) -> CoreResult<()> {
        if self
            .store
            .with(|s| Ok(s.blob_exists(hash)))
            .await?
        {
            return Ok(());
        }

        let contact = self.get_contact(contact_id).await?;
        let peer_id_str = contact.peer_id.as_ref().ok_or_else(|| {
            CoreError::P2p(format!("friend {contact_id} has no peer id (offline)"))
        })?;
        let peer_id: libp2p::PeerId = peer_id_str
            .parse()
            .map_err(|e| CoreError::P2p(format!("invalid peer id: {e}")))?;

        let connected = {
            let p2p_guard = self.p2p.lock().await;
            let node = p2p_guard
                .as_ref()
                .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
            node.connected_peer_ids().await
        };
        if !connected.iter().any(|id| id == peer_id_str) {
            use std::time::Duration;
            self.wait_for_peer_connected_redial(
                peer_id_str,
                &contact.multiaddrs,
                Duration::from_secs(20),
                "profile blob",
                2,
            )
            .await?;
        }

        let p2p_guard = self.p2p.lock().await;
        let node = p2p_guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        node.request_blob_from_peer(peer_id, hash).await
    }

    /// List comments for a profile item. Own items: local DB. Friend items: live P2P pull.
    pub async fn fetch_profile_item_comments(
        &self,
        contact_id: Option<&str>,
        profile_item_id: &str,
    ) -> CoreResult<Vec<ProfileComment>> {
        // Own profile item?
        if self
            .store
            .with(|s| s.get_profile_item(profile_item_id))
            .await?
            .is_some()
        {
            return self.list_profile_comments(profile_item_id).await;
        }

        let contact_id = contact_id.ok_or_else(|| {
            CoreError::ContentNotFound(format!("profile item {profile_item_id}"))
        })?;
        let contact = self.get_contact(contact_id).await?;
        let peer_id_str = contact.peer_id.as_ref().ok_or_else(|| {
            CoreError::P2p(format!("friend {contact_id} has no peer id (offline)"))
        })?;
        let peer_id: libp2p::PeerId = peer_id_str
            .parse()
            .map_err(|e| CoreError::P2p(format!("invalid peer id: {e}")))?;

        let p2p_guard = self.p2p.lock().await;
        let node = p2p_guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        node.request_profile_comments_from_peer(peer_id, profile_item_id)
            .await
    }

    /// Comment on a durable profile item. Delivered to the profile owner only.
    pub async fn add_profile_comment(
        &self,
        profile_item_id: &str,
        body: &str,
        owner_contact_id: Option<&str>,
    ) -> CoreResult<ProfileComment> {
        let body = body.trim();
        if body.is_empty() {
            return Err(CoreError::Invite("comment cannot be empty".into()));
        }

        let identity = self.identity.read().await;
        if identity.display_name.is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }

        let comment = ProfileComment {
            id: uuid::Uuid::new_v4().to_string(),
            profile_item_id: profile_item_id.to_string(),
            author_id: identity.signing_pubkey.clone(),
            author_name: identity.display_name.clone(),
            body: body.to_string(),
            created_at: chrono::Utc::now(),
        };

        // Commenting on own item: store locally only.
        if self
            .store
            .with(|s| s.get_profile_item(profile_item_id))
            .await?
            .is_some()
        {
            drop(identity);
            self.store
                .with_mut(|store| store.insert_profile_comment(&comment))
                .await?;
            info!(%profile_item_id, "own profile comment saved");
            return Ok(comment);
        }

        let owner_id = owner_contact_id.ok_or_else(|| {
            CoreError::Invite("owner_contact_id required for friend profile comments".into())
        })?;
        let owner = self.get_contact(owner_id).await?;
        let envelope = build_profile_comment_envelope(
            &identity,
            &owner,
            profile_item_id,
            body,
        )?;
        drop(identity);

        let envelope_id = envelope.id.clone();
        let envelope_json = serde_json::to_string(&envelope)?;
        self.store
            .with_mut(|store| {
                store.insert_outbox(
                    &OutboxEntry {
                        content_id: envelope_id,
                        recipient_id: owner.id.clone(),
                        status: DeliveryStatus::Pending,
                        expires_at: envelope.expires_at,
                        retry_count: 0,
                        ciphertext: envelope.ciphertext.clone(),
                        content_type: ContentType::ProfileComment,
                    },
                    &envelope_json,
                )
            })
            .await?;

        if let Some(peer_id_str) = owner.peer_id.as_ref() {
            if let Ok(peer_id) = peer_id_str.parse() {
                let p2p_guard = self.p2p.lock().await;
                if let Some(p2p) = p2p_guard.as_ref() {
                    let _ = p2p.send_envelope_to_peer(peer_id, envelope).await;
                }
            }
        }

        info!(%profile_item_id, owner = %owner_id, "profile comment queued for owner");
        Ok(comment)
    }

    pub async fn fetch_friend_archive_folder(
        &self,
        contact_id: &str,
        folder_id: &str,
    ) -> CoreResult<(ArchiveFolderSummary, Vec<ArchiveEntry>)> {
        let contact = self.get_contact(contact_id).await?;
        let peer_id_str = contact.peer_id.as_ref().ok_or_else(|| {
            CoreError::P2p(format!("friend {contact_id} has no peer id (offline)"))
        })?;
        let peer_id: libp2p::PeerId = peer_id_str
            .parse()
            .map_err(|e| CoreError::P2p(format!("invalid peer id: {e}")))?;

        let p2p_guard = self.p2p.lock().await;
        let node = p2p_guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let (folder, entries, manifests) = node
            .request_archive_list_from_peer(peer_id, folder_id)
            .await?;
        drop(p2p_guard);

        // Persist manifests so chunk fetch can resolve author + hashes.
        for manifest in manifests {
            let _ = self
                .store
                .with_mut(|s| s.insert_manifest(&manifest))
                .await;
        }

        Ok((folder, entries))
    }

    pub async fn list_archive_folders(&self) -> CoreResult<Vec<ArchiveFolder>> {
        self.store.with(|store| store.list_archive_folders()).await
    }

    pub async fn list_archive_folder_summaries(&self) -> CoreResult<Vec<ArchiveFolderSummary>> {
        self.store
            .with(|store| store.list_archive_folder_summaries())
            .await
    }

    pub async fn list_archive_entries(&self, folder_id: &str) -> CoreResult<Vec<ArchiveEntry>> {
        self.store
            .with(|store| store.list_archive_entries(folder_id))
            .await
    }

    pub async fn create_archive_folder(&self, name: &str) -> CoreResult<ArchiveFolder> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::Invite("folder name cannot be empty".into()));
        }
        let folder = ArchiveFolder {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            created_at: chrono::Utc::now(),
        };
        self.store
            .with_mut(|store| store.insert_archive_folder(&folder))
            .await?;
        Ok(folder)
    }

    pub async fn delete_archive_folder(&self, folder_id: &str) -> CoreResult<()> {
        self.store
            .with_mut(|store| store.delete_archive_folder(folder_id))
            .await
    }

    pub async fn add_archive_entry(
        &self,
        folder_id: &str,
        name: &str,
        data: &[u8],
        mime: &str,
    ) -> CoreResult<ArchiveEntry> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::Invite("entry name cannot be empty".into()));
        }
        let folder = self
            .store
            .with(|store| store.get_archive_folder(folder_id))
            .await?;
        if folder.is_none() {
            return Err(CoreError::ContentNotFound(folder_id.to_string()));
        }

        // Reuse video chunking for large files (thumb is empty placeholder).
        let empty_thumb: &[u8] = &[];
        let manifest = self
            .store
            .with_mut(|store| store.chunk_and_store_file(data, empty_thumb, mime, 0))
            .await?;

        let entry = ArchiveEntry {
            id: uuid::Uuid::new_v4().to_string(),
            folder_id: folder_id.to_string(),
            name: name.to_string(),
            root_hash: manifest.root_hash.clone(),
            total_bytes: manifest.total_bytes,
            mime: mime.to_string(),
            created_at: chrono::Utc::now(),
        };
        self.store
            .with_mut(|store| {
                store.insert_manifest(&manifest)?;
                store.insert_archive_entry(&entry)
            })
            .await?;
        Ok(entry)
    }

    pub async fn delete_archive_entry(&self, entry_id: &str) -> CoreResult<()> {
        self.store
            .with_mut(|store| store.delete_archive_entry(entry_id))
            .await
    }

    pub async fn begin_archive_upload(
        &self,
        folder_id: &str,
        name: &str,
        mime: &str,
        total_bytes: u64,
    ) -> CoreResult<ArchiveUploadStatus> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::Invite("entry name cannot be empty".into()));
        }
        if total_bytes == 0 {
            return Err(CoreError::Invite("file cannot be empty".into()));
        }
        let folder = self
            .store
            .with(|store| store.get_archive_folder(folder_id))
            .await?;
        if folder.is_none() {
            return Err(CoreError::ContentNotFound(folder_id.to_string()));
        }
        let chunk_size = CHUNK_SIZE as u32;
        let chunks_total = total_bytes.div_ceil(CHUNK_SIZE as u64) as u32;
        let upload = ArchiveUpload {
            id: uuid::Uuid::new_v4().to_string(),
            folder_id: folder_id.to_string(),
            name: name.to_string(),
            mime: mime.to_string(),
            total_bytes,
            chunk_size,
            chunks_total,
            root_hash: None,
            created_at: chrono::Utc::now(),
            completed_at: None,
        };
        self.store
            .with_mut(|store| store.insert_archive_upload(&upload))
            .await?;
        self.store
            .with(|store| {
                store
                    .archive_upload_status(&upload.id)?
                    .ok_or_else(|| CoreError::ContentNotFound(upload.id.clone()))
            })
            .await
    }

    pub async fn put_archive_upload_chunk(
        &self,
        upload_id: &str,
        index: u32,
        expected_hash: &str,
        data: &[u8],
    ) -> CoreResult<ArchiveUploadStatus> {
        let upload = self
            .store
            .with(|s| s.get_archive_upload(upload_id))
            .await?
            .ok_or_else(|| CoreError::ContentNotFound(upload_id.to_string()))?;
        if upload.completed_at.is_some() {
            return Err(CoreError::Invite("upload already completed".into()));
        }
        if index >= upload.chunks_total {
            return Err(CoreError::Invite(format!(
                "chunk index {index} out of range (0..{})",
                upload.chunks_total
            )));
        }
        self.store
            .with_mut(|s| s.store_upload_chunk(upload_id, index, expected_hash, data))
            .await?;
        self.store
            .with(|s| {
                s.archive_upload_status(upload_id)?
                    .ok_or_else(|| CoreError::ContentNotFound(upload_id.to_string()))
            })
            .await
    }

    pub async fn archive_upload_status(
        &self,
        upload_id: &str,
    ) -> CoreResult<ArchiveUploadStatus> {
        self.store
            .with(|s| {
                s.archive_upload_status(upload_id)?
                    .ok_or_else(|| CoreError::ContentNotFound(upload_id.to_string()))
            })
            .await
    }

    pub async fn complete_archive_upload(&self, upload_id: &str) -> CoreResult<ArchiveEntry> {
        self.store
            .with_mut(|s| s.finalize_archive_upload(upload_id))
            .await
    }

    pub async fn read_blob(&self, hash: &str) -> CoreResult<Vec<u8>> {
        self.store.with(|store| store.read_blob_resolved(hash)).await
    }

    pub async fn store_blob(&self, data: &[u8]) -> CoreResult<String> {
        self.store.with_mut(|store| store.store_blob(data)).await
    }
}
