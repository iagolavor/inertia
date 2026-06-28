use sha2::{Digest, Sha256};

use crate::error::{CoreError, CoreResult};
use crate::identity::encode_hex;

use super::Store;

/// Max blob size accepted over P2P (matches web client compression cap with headroom).
pub const MAX_BLOB_BYTES: usize = 2 * 1024 * 1024;

impl Store {
    pub fn store_blob(&self, data: &[u8]) -> CoreResult<String> {
        let hash = encode_hex(Sha256::digest(data));
        let path = self.blob_path(&hash);
        if !path.exists() {
            std::fs::write(&path, data)?;
        }
        Ok(hash)
    }

    pub fn blob_exists(&self, hash: &str) -> bool {
        self.blob_path(hash).exists()
    }

    pub fn store_blob_verified(&self, expected_hash: &str, data: &[u8]) -> CoreResult<()> {
        if data.len() > MAX_BLOB_BYTES {
            return Err(CoreError::P2p(format!(
                "blob too large ({} bytes, max {})",
                data.len(),
                MAX_BLOB_BYTES
            )));
        }
        let computed = encode_hex(Sha256::digest(data));
        if computed != expected_hash {
            return Err(CoreError::Crypto("blob hash mismatch".into()));
        }
        let path = self.blob_path(expected_hash);
        if !path.exists() {
            std::fs::write(&path, data)?;
        }
        Ok(())
    }

    /// Media hashes referenced in inbox/feed for a given author signing key, missing on disk.
    /// Prefer [`Store::missing_sync_hashes_for_author`] for P2P sync (thumb-only for video).
    pub fn missing_media_refs_for_author(&self, author_signing_key: &str) -> CoreResult<Vec<String>> {
        self.missing_sync_hashes_for_author(author_signing_key)
    }

    pub fn read_blob(&self, hash: &str) -> CoreResult<Vec<u8>> {
        let path = self.blob_path(hash);
        if !path.exists() {
            return Err(CoreError::ContentNotFound(hash.to_string()));
        }
        Ok(std::fs::read(&path)?)
    }
}
