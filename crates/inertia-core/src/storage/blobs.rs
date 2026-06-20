use sha2::{Digest, Sha256};

use crate::error::{CoreError, CoreResult};
use crate::identity::encode_hex;

use super::sql::FEED_HISTORY_KEY;
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
    pub fn missing_media_refs_for_author(&self, author_signing_key: &str) -> CoreResult<Vec<String>> {
        let mut missing = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let inbox = self.list_inbox()?;
        for entry in inbox {
            if entry.sender_id != author_signing_key {
                continue;
            }
            if let Some(ref hash) = entry.media_ref {
                if seen.insert(hash.clone()) && !self.blob_exists(hash) {
                    missing.push(hash.clone());
                }
            }
        }

        if self.get_bool_setting(FEED_HISTORY_KEY)?.unwrap_or(false) {
            for item in self.list_feed_archive()? {
                if item.author_id != author_signing_key {
                    continue;
                }
                if let Some(ref hash) = item.media_ref {
                    if seen.insert(hash.clone()) && !self.blob_exists(hash) {
                        missing.push(hash.clone());
                    }
                }
            }
        }

        Ok(missing)
    }

    pub fn read_blob(&self, hash: &str) -> CoreResult<Vec<u8>> {
        let path = self.blob_path(hash);
        if !path.exists() {
            return Err(CoreError::ContentNotFound(hash.to_string()));
        }
        Ok(std::fs::read(&path)?)
    }
}
