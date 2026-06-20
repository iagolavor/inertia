use chrono::Utc;
use rusqlite::params;

use crate::content::DeliveryStatus;
use crate::error::CoreResult;

use super::sql::{content_type_str, parse_content_type, parse_status, status_str};
use super::{OutboxEntry, Store};

impl Store {
    pub fn insert_outbox(&self, entry: &OutboxEntry, envelope_json: &str) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO outbox
             (content_id, recipient_id, status, expires_at, retry_count, ciphertext, content_type, envelope_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.content_id,
                entry.recipient_id,
                status_str(entry.status),
                entry.expires_at.to_rfc3339(),
                entry.retry_count,
                entry.ciphertext,
                content_type_str(entry.content_type),
                envelope_json,
            ],
        )?;
        Ok(())
    }

    pub fn list_outbox(&self) -> CoreResult<Vec<OutboxEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, recipient_id, status, expires_at, retry_count, ciphertext, content_type
             FROM outbox ORDER BY expires_at",
        )?;
        let rows = stmt.query_map([], |row| {
            let status: String = row.get("status")?;
            let content_type: String = row.get("content_type")?;
            Ok(OutboxEntry {
                content_id: row.get("content_id")?,
                recipient_id: row.get("recipient_id")?,
                status: parse_status(&status),
                expires_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                retry_count: row.get("retry_count")?,
                ciphertext: row.get("ciphertext")?,
                content_type: parse_content_type(&content_type),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(crate::error::CoreError::from)
    }

    pub fn get_outbox_envelope(&self, content_id: &str, recipient_id: &str) -> CoreResult<String> {
        let mut stmt = self.conn.prepare(
            "SELECT envelope_json FROM outbox WHERE content_id = ?1 AND recipient_id = ?2",
        )?;
        let mut rows = stmt.query(params![content_id, recipient_id])?;
        if let Some(row) = rows.next()? {
            Ok(row.get("envelope_json")?)
        } else {
            Err(crate::error::CoreError::ContentNotFound(format!(
                "{content_id}:{recipient_id}"
            )))
        }
    }

    pub fn update_outbox_status(
        &self,
        content_id: &str,
        recipient_id: &str,
        status: DeliveryStatus,
    ) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE outbox SET status = ?1 WHERE content_id = ?2 AND recipient_id = ?3",
            params![status_str(status), content_id, recipient_id],
        )?;
        self.conn.execute(
            "UPDATE sent_messages SET status = ?1 WHERE content_id = ?2 AND recipient_id = ?3",
            params![status_str(status), content_id, recipient_id],
        )?;
        Ok(())
    }

    pub fn increment_outbox_retry(&self, content_id: &str, recipient_id: &str) -> CoreResult<()> {
        self.conn.execute(
            "UPDATE outbox SET retry_count = retry_count + 1, status = 'pending'
             WHERE content_id = ?1 AND recipient_id = ?2",
            params![content_id, recipient_id],
        )?;
        Ok(())
    }

    pub fn record_ack(&self, content_id: &str, recipient_id: &str) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO delivery_acks (content_id, recipient_id, acked_at)
             VALUES (?1, ?2, ?3)",
            params![content_id, recipient_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }
}
