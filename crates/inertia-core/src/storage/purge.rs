use chrono::Utc;
use rusqlite::params;

use crate::error::CoreResult;

use super::{PurgeReport, Store};

impl Store {
    /// Purge ephemeral feed/message rows only.
    /// Never deletes `profile_items`, `profile_comments`, `archive_folders`, or `archive_entries`.
    pub fn purge_expired(&self) -> CoreResult<PurgeReport> {
        let now = Utc::now().to_rfc3339();
        let outbox = self.conn.execute(
            "DELETE FROM outbox WHERE expires_at < ?1",
            params![now],
        )?;
        let inbox = self.conn.execute(
            "DELETE FROM inbox WHERE expires_at < ?1",
            params![now],
        )?;
        let local_posts = self.conn.execute(
            "DELETE FROM local_posts WHERE expires_at < ?1",
            params![now],
        )?;
        let invites = self.conn.execute(
            "DELETE FROM issued_invites WHERE expires_at < ?1 AND consumed_at IS NULL",
            params![now],
        )?;
        let sent_messages = self.conn.execute(
            "DELETE FROM sent_messages WHERE expires_at < ?1",
            params![now],
        )?;
        // Intentionally leave profile_items / profile_comments / archive_* intact.
        // Blob GC (Phase 6) must reference-count those tables before deleting blobs.
        Ok(PurgeReport {
            outbox,
            inbox,
            local_posts,
            invites,
            sent_messages,
        })
    }

    /// Hashes that must not be deleted by orphan blob GC (profile + archive + identity).
    pub fn durable_blob_refs(&self) -> CoreResult<std::collections::HashSet<String>> {
        let mut refs = std::collections::HashSet::new();
        for hash in self.profile_item_blob_hashes()? {
            refs.insert(hash);
        }
        for root in self.archive_entry_root_hashes()? {
            refs.insert(root.clone());
            if let Ok(Some(manifest)) = self.get_manifest(&root) {
                if !manifest.thumb_hash.is_empty() {
                    refs.insert(manifest.thumb_hash);
                }
            }
        }
        if let Ok(Some(identity)) = self.load_identity() {
            if let Some(avatar) = identity.avatar_blob_hash {
                refs.insert(avatar);
            }
        }
        Ok(refs)
    }
}
