use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::error::{CoreError, CoreResult};

use super::Store;

impl Store {
    pub fn register_issued_invite(&self, nonce: &str, expires_at: DateTime<Utc>) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO issued_invites (nonce, expires_at, consumed_at) VALUES (?1, ?2, NULL)",
            params![nonce, expires_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn consume_issued_invite(&self, nonce: &str) -> CoreResult<()> {
        let mut stmt = self
            .conn
            .prepare("SELECT expires_at, consumed_at FROM issued_invites WHERE nonce = ?1")?;
        let mut rows = stmt.query(params![nonce])?;
        let row = rows
            .next()?
            .ok_or_else(|| CoreError::Invite("unknown invite".into()))?;
        let expires_at: String = row.get("expires_at")?;
        let consumed_at: Option<String> = row.get("consumed_at")?;
        if consumed_at.is_some() {
            return Err(CoreError::Invite("invite already used".into()));
        }
        let expires = DateTime::parse_from_rfc3339(&expires_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| CoreError::Invite(e.to_string()))?;
        if Utc::now() > expires {
            return Err(CoreError::Invite("invite has expired".into()));
        }
        self.conn.execute(
            "UPDATE issued_invites SET consumed_at = ?1 WHERE nonce = ?2",
            params![Utc::now().to_rfc3339(), nonce],
        )?;
        Ok(())
    }

    pub fn is_invite_redeemed_locally(&self, nonce: &str) -> CoreResult<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM redeemed_invites WHERE nonce = ?1")?;
        let mut rows = stmt.query(params![nonce])?;
        Ok(rows.next()?.is_some())
    }

    pub fn mark_invite_redeemed_locally(
        &self,
        nonce: &str,
        issuer_signing_pubkey: &str,
    ) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO redeemed_invites (nonce, issuer_signing_pubkey, redeemed_at)
             VALUES (?1, ?2, ?3)",
            params![nonce, issuer_signing_pubkey, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }
}
