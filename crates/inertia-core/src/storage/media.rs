use std::path::PathBuf;

use sha2::{Digest, Sha256};

use crate::content::{MediaKind, MediaManifest};
use crate::error::{CoreError, CoreResult};
use crate::identity::encode_hex;

use super::{FeedItem, Store};

pub const CHUNK_SIZE: usize = 512 * 1024;
pub const MAX_VIDEO_BYTES: usize = 50 * 1024 * 1024;
pub const MAX_THUMB_BYTES: usize = 256 * 1024;
/// Legacy single-shot base64 archive upload cap (chunked ingest has no product cap).
pub const MAX_ARCHIVE_FILE_BYTES: usize = 50 * 1024 * 1024;
/// Soft guidance for UI zip-in-browser warnings (not enforced server-side on chunked path).
pub const ARCHIVE_ZIP_SOFT_WARN_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestBody {
    pub kind: MediaKind,
    pub mime: String,
    pub total_bytes: u64,
    pub chunk_size: u32,
    pub chunk_hashes: Vec<String>,
    pub thumb_hash: String,
    pub duration_ms: u32,
}

impl Store {
    pub fn chunk_dir(&self, root_hash: &str) -> PathBuf {
        self.blob_dir.join("chunks").join(root_hash)
    }

    pub fn chunk_path(&self, root_hash: &str, index: u32) -> PathBuf {
        self.chunk_dir(root_hash).join(format!("{index:05}"))
    }

    pub fn insert_manifest(&self, manifest: &MediaManifest) -> CoreResult<()> {
        let json = serde_json::to_string(manifest)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO media_manifests (root_hash, manifest_json, expires_at)
             VALUES (?1, ?2, datetime('now', '+7 days'))",
            rusqlite::params![manifest.root_hash, json],
        )?;
        Ok(())
    }

    /// Durable manifest for shared-folder files (no 7d expiry).
    pub fn insert_manifest_durable(&self, manifest: &MediaManifest) -> CoreResult<()> {
        let json = serde_json::to_string(manifest)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO media_manifests (root_hash, manifest_json, expires_at)
             VALUES (?1, ?2, '2099-01-01T00:00:00+00:00')",
            rusqlite::params![manifest.root_hash, json],
        )?;
        Ok(())
    }

    pub fn get_manifest(&self, root_hash: &str) -> CoreResult<Option<MediaManifest>> {
        let mut stmt = self.conn.prepare(
            "SELECT manifest_json FROM media_manifests WHERE root_hash = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![root_hash])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            Ok(Some(serde_json::from_str(&json)?))
        } else {
            Ok(None)
        }
    }

    pub fn store_chunk_verified(
        &self,
        root_hash: &str,
        index: u32,
        expected_hash: &str,
        data: &[u8],
    ) -> CoreResult<()> {
        if data.len() > CHUNK_SIZE {
            return Err(CoreError::P2p(format!(
                "chunk too large ({} bytes, max {})",
                data.len(),
                CHUNK_SIZE
            )));
        }
        let computed = encode_hex(Sha256::digest(data));
        if computed != expected_hash {
            return Err(CoreError::Crypto("chunk hash mismatch".into()));
        }
        let dir = self.chunk_dir(root_hash);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(self.chunk_path(root_hash, index), data)?;
        Ok(())
    }

    pub fn read_chunk(&self, root_hash: &str, index: u32) -> CoreResult<Vec<u8>> {
        let path = self.chunk_path(root_hash, index);
        if !path.exists() {
            return Err(CoreError::ContentNotFound(format!(
                "{root_hash}#{index}"
            )));
        }
        Ok(std::fs::read(path)?)
    }

    pub fn chunk_exists(&self, root_hash: &str, index: u32) -> bool {
        self.chunk_path(root_hash, index).exists()
    }

    pub fn count_local_chunks(&self, manifest: &MediaManifest) -> u32 {
        (0..manifest.chunk_hashes.len() as u32)
            .filter(|i| self.chunk_exists(&manifest.root_hash, *i))
            .count() as u32
    }

    pub fn media_is_complete(&self, manifest: &MediaManifest) -> bool {
        if self.blob_exists(&manifest.root_hash) {
            return true;
        }
        manifest.chunk_hashes.iter().enumerate().all(|(i, _)| {
            self.chunk_exists(&manifest.root_hash, i as u32)
        })
    }

    pub fn assemble_media_if_complete(&self, manifest: &MediaManifest) -> CoreResult<bool> {
        if self.blob_exists(&manifest.root_hash) {
            return Ok(true);
        }
        if !manifest
            .chunk_hashes
            .iter()
            .enumerate()
            .all(|(i, hash)| {
                self.chunk_exists(&manifest.root_hash, i as u32)
                    && self
                        .read_chunk(&manifest.root_hash, i as u32)
                        .map(|data| encode_hex(Sha256::digest(&data)) == *hash)
                        .unwrap_or(false)
            })
        {
            return Ok(false);
        }

        let mut out = Vec::with_capacity(manifest.total_bytes as usize);
        for i in 0..manifest.chunk_hashes.len() {
            out.extend_from_slice(&self.read_chunk(&manifest.root_hash, i as u32)?);
        }
        if out.len() as u64 != manifest.total_bytes {
            return Err(CoreError::P2p("assembled size mismatch".into()));
        }
        let path = self.blob_path(&manifest.root_hash);
        std::fs::write(path, &out)?;
        Ok(true)
    }

    pub fn chunk_and_store_video(
        &self,
        video: &[u8],
        thumb: &[u8],
        duration_ms: u32,
    ) -> CoreResult<MediaManifest> {
        self.chunk_and_store_bytes(
            video,
            thumb,
            MediaKind::Video,
            "video/mp4",
            duration_ms,
            MAX_VIDEO_BYTES,
        )
    }

    /// Chunk and store a shared-folder file (no inbox fan-out; pull on demand).
    pub fn chunk_and_store_file(
        &self,
        data: &[u8],
        thumb: &[u8],
        mime: &str,
        duration_ms: u32,
    ) -> CoreResult<MediaManifest> {
        self.chunk_and_store_bytes(
            data,
            thumb,
            MediaKind::File,
            mime,
            duration_ms,
            MAX_ARCHIVE_FILE_BYTES,
        )
    }

    fn chunk_and_store_bytes(
        &self,
        data: &[u8],
        thumb: &[u8],
        kind: MediaKind,
        mime: &str,
        duration_ms: u32,
        max_bytes: usize,
    ) -> CoreResult<MediaManifest> {
        if data.len() > max_bytes {
            return Err(CoreError::P2p(format!(
                "file too large ({} bytes, max {})",
                data.len(),
                max_bytes
            )));
        }
        if thumb.len() > MAX_THUMB_BYTES {
            return Err(CoreError::P2p(format!(
                "thumb too large ({} bytes, max {})",
                thumb.len(),
                MAX_THUMB_BYTES
            )));
        }

        let thumb_hash = if thumb.is_empty() {
            String::new()
        } else {
            self.store_blob(thumb)?
        };
        let mut chunk_hashes = Vec::new();
        for chunk in data.chunks(CHUNK_SIZE) {
            chunk_hashes.push(encode_hex(Sha256::digest(chunk)));
        }

        let body = ManifestBody {
            kind,
            mime: mime.into(),
            total_bytes: data.len() as u64,
            chunk_size: CHUNK_SIZE as u32,
            chunk_hashes: chunk_hashes.clone(),
            thumb_hash: thumb_hash.clone(),
            duration_ms,
        };
        let root_hash = hash_manifest_body(&body);
        let manifest = MediaManifest {
            root_hash: root_hash.clone(),
            kind,
            mime: body.mime.clone(),
            total_bytes: body.total_bytes,
            chunk_size: body.chunk_size,
            chunk_hashes: body.chunk_hashes.clone(),
            thumb_hash: body.thumb_hash.clone(),
            duration_ms: body.duration_ms,
        };

        for (i, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
            self.store_chunk_verified(&root_hash, i as u32, &chunk_hashes[i], chunk)?;
        }
        self.insert_manifest(&manifest)?;
        self.assemble_media_if_complete(&manifest)?;
        Ok(manifest)
    }

    /// Read a blob from disk, assembling chunked video into `blobs/{root_hash}` when needed.
    pub fn read_blob_resolved(&self, hash: &str) -> CoreResult<Vec<u8>> {
        if self.blob_exists(hash) {
            return self.read_blob(hash);
        }
        if let Some(manifest) = self.get_manifest(hash)? {
            self.assemble_media_if_complete(&manifest)?;
        }
        self.read_blob(hash)
    }

    pub fn apply_media_meta(&self, item: &mut FeedItem) -> CoreResult<()> {
        let Some(ref media_ref) = item.media_ref else {
            item.media_ready = false;
            return Ok(());
        };

        if let Some(manifest) = self.get_manifest(media_ref)? {
            item.media_kind = Some(manifest.kind);
            item.thumb_ref = Some(manifest.thumb_hash.clone());
            if self.media_is_complete(&manifest) {
                let _ = self.assemble_media_if_complete(&manifest);
            }
            item.media_ready = self.blob_exists(media_ref);
        } else {
            item.media_kind = Some(MediaKind::Photo);
            item.thumb_ref = Some(media_ref.clone());
            item.media_ready = self.blob_exists(media_ref);
        }
        Ok(())
    }

    pub fn sync_hash_for_media_ref(&self, media_ref: &str) -> CoreResult<String> {
        if let Some(manifest) = self.get_manifest(media_ref)? {
            Ok(manifest.thumb_hash)
        } else {
            Ok(media_ref.to_string())
        }
    }

    pub fn missing_sync_hashes_for_author(&self, author_signing_key: &str) -> CoreResult<Vec<String>> {
        let mut missing = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let inbox = self.list_inbox()?;
        for entry in inbox {
            if entry.sender_id != author_signing_key {
                continue;
            }
            Self::push_missing_media_hash(self, entry.media_ref.as_deref(), &mut missing, &mut seen)?;
        }

        if self.get_bool_setting(super::sql::FEED_HISTORY_KEY)?.unwrap_or(false) {
            for item in self.list_feed_archive()? {
                if item.author_id != author_signing_key {
                    continue;
                }
                Self::push_missing_media_hash(
                    self,
                    item.media_ref.as_deref(),
                    &mut missing,
                    &mut seen,
                )?;
            }
        }

        Ok(missing)
    }

    fn push_missing_media_hash(
        &self,
        media_ref: Option<&str>,
        missing: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
    ) -> CoreResult<()> {
        let Some(media_ref) = media_ref else {
            return Ok(());
        };
        if let Some(manifest) = self.get_manifest(media_ref)? {
            if seen.insert(manifest.thumb_hash.clone()) && !self.blob_exists(&manifest.thumb_hash) {
                missing.push(manifest.thumb_hash);
            }
        } else if seen.insert(media_ref.to_string()) && !self.blob_exists(media_ref) {
            missing.push(media_ref.to_string());
        }
        Ok(())
    }
}

pub fn hash_manifest_body(body: &ManifestBody) -> String {
    encode_hex(Sha256::digest(serde_json::to_vec(body).unwrap_or_default()))
}
