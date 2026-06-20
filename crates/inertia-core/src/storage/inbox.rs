use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::content::DeliveryStatus;
use crate::error::CoreResult;

use super::sql::{content_type_str, parse_content_type, parse_status, status_str};
use super::{InboxEntry, SentMessage, Store};

impl Store {
    pub fn insert_inbox(&self, entry: &InboxEntry) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO inbox
             (content_id, sender_id, received_at, expires_at, read_at, body, media_ref, content_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.content_id,
                entry.sender_id,
                entry.received_at.to_rfc3339(),
                entry.expires_at.to_rfc3339(),
                entry.read_at.map(|t| t.to_rfc3339()),
                entry.body,
                entry.media_ref,
                content_type_str(entry.content_type),
            ],
        )?;
        Ok(())
    }

    pub fn insert_sent_message(
        &self,
        content_id: &str,
        recipient_id: &str,
        body: &str,
        sent_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        status: DeliveryStatus,
    ) -> CoreResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sent_messages
             (content_id, recipient_id, body, sent_at, expires_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                content_id,
                recipient_id,
                body,
                sent_at.to_rfc3339(),
                expires_at.to_rfc3339(),
                status_str(status),
            ],
        )?;
        Ok(())
    }

    pub fn list_sent_messages_for_recipient(
        &self,
        recipient_id: &str,
    ) -> CoreResult<Vec<SentMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, recipient_id, body, sent_at, expires_at, status
             FROM sent_messages WHERE recipient_id = ?1 ORDER BY sent_at ASC",
        )?;
        let rows = stmt.query_map(params![recipient_id], |row| {
            let status: String = row.get("status")?;
            Ok(SentMessage {
                content_id: row.get("content_id")?,
                recipient_id: row.get("recipient_id")?,
                body: row.get("body")?,
                sent_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("sent_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                status: parse_status(&status),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(crate::error::CoreError::from)
    }

    pub fn list_inbox_messages_from_sender(&self, sender_id: &str) -> CoreResult<Vec<InboxEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, sender_id, received_at, expires_at, read_at, body, media_ref, content_type
             FROM inbox
             WHERE sender_id = ?1 AND content_type = 'message'
             ORDER BY received_at ASC",
        )?;
        let rows = stmt.query_map(params![sender_id], |row| {
            let content_type: String = row.get("content_type")?;
            Ok(InboxEntry {
                content_id: row.get("content_id")?,
                sender_id: row.get("sender_id")?,
                received_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("received_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                read_at: row
                    .get::<_, Option<String>>("read_at")?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                content_type: parse_content_type(&content_type),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(crate::error::CoreError::from)
    }

    pub fn list_inbox(&self) -> CoreResult<Vec<InboxEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_id, sender_id, received_at, expires_at, read_at, body, media_ref, content_type
             FROM inbox ORDER BY received_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let content_type: String = row.get("content_type")?;
            Ok(InboxEntry {
                content_id: row.get("content_id")?,
                sender_id: row.get("sender_id")?,
                received_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("received_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("expires_at")?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                read_at: row
                    .get::<_, Option<String>>("read_at")?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                body: row.get("body")?,
                media_ref: row.get("media_ref")?,
                content_type: parse_content_type(&content_type),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(crate::error::CoreError::from)
    }
}
