use chrono::Utc;
use rusqlite::params;

use crate::error::CoreResult;

use super::{PurgeReport, Store};

impl Store {
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
        Ok(PurgeReport {
            outbox,
            inbox,
            local_posts,
            invites,
            sent_messages,
        })
    }
}
