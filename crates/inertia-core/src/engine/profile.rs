use crate::error::CoreResult;
use crate::storage::ProfilePhoto;

use super::{Engine, PublishPhotoResult};

impl Engine {
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
        self.store.with(|store| store.read_blob_resolved(hash)).await
    }

    pub async fn store_blob(&self, data: &[u8]) -> CoreResult<String> {
        self.store.with_mut(|store| store.store_blob(data)).await
    }
}
