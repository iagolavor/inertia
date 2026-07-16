use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::error::{CoreError, CoreResult};

use super::sql::{
    connection_state_str, decode_multiaddrs, encode_multiaddrs, merge_multiaddr_lists,
    parse_connection_state,
};
use super::{Contact, Store};

impl Store {
    pub fn upsert_contact(&self, contact: &Contact) -> CoreResult<()> {
        let mut contact = contact.clone();
        if let Ok(existing) = self.get_contact(&contact.id) {
            contact.multiaddrs =
                merge_multiaddr_lists(&existing.multiaddrs, &contact.multiaddrs);
        }
        self.conn.execute(
            "INSERT OR REPLACE INTO contacts
             (id, phone_hash, display_name, peer_id, signing_pubkey, encryption_pubkey, last_seen, connection_state, multiaddrs)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                contact.id,
                contact.phone_hash,
                contact.display_name,
                contact.peer_id,
                contact.signing_pubkey,
                contact.encryption_pubkey,
                contact.last_seen.map(|t| t.to_rfc3339()),
                connection_state_str(contact.connection_state),
                encode_multiaddrs(&contact.multiaddrs),
            ],
        )?;
        Ok(())
    }

    pub fn merge_contact_multiaddrs_by_peer_id(
        &self,
        peer_id: &str,
        new_addrs: &[String],
    ) -> CoreResult<()> {
        for mut contact in self.list_contacts()? {
            if contact.peer_id.as_deref() == Some(peer_id) {
                contact.multiaddrs = merge_multiaddr_lists(&contact.multiaddrs, new_addrs);
                self.upsert_contact(&contact)?;
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn list_contacts(&self) -> CoreResult<Vec<Contact>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, phone_hash, display_name, peer_id, signing_pubkey, encryption_pubkey, last_seen, connection_state, multiaddrs
             FROM contacts ORDER BY display_name",
        )?;
        let rows = stmt.query_map([], |row| {
            let state_str: String = row.get("connection_state")?;
            let connection_state = parse_connection_state(&state_str);
            let multiaddrs_raw: String = row.get("multiaddrs").unwrap_or_else(|_| "[]".into());
            Ok(Contact {
                id: row.get("id")?,
                phone_hash: row.get("phone_hash")?,
                display_name: row.get("display_name")?,
                peer_id: row.get("peer_id")?,
                signing_pubkey: row.get("signing_pubkey")?,
                encryption_pubkey: row.get("encryption_pubkey")?,
                last_seen: row
                    .get::<_, Option<String>>("last_seen")?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                connection_state,
                multiaddrs: decode_multiaddrs(&multiaddrs_raw),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CoreError::from)
    }

    pub fn get_contact(&self, id: &str) -> CoreResult<Contact> {
        let contacts = self.list_contacts()?;
        contacts
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| CoreError::ContactNotFound(id.to_string()))
    }

    /// Remove a contact and peer-tied DM delivery state (local-only; peer is not notified).
    pub fn delete_contact(&self, id: &str) -> CoreResult<()> {
        let contact = self.get_contact(id)?;
        let mut peer_keys = vec![contact.id.clone()];
        if contact.signing_pubkey != contact.id {
            peer_keys.push(contact.signing_pubkey.clone());
        }

        for key in &peer_keys {
            self.conn.execute(
                "DELETE FROM outbox WHERE recipient_id = ?1",
                params![key],
            )?;
            self.conn.execute(
                "DELETE FROM delivery_acks WHERE recipient_id = ?1",
                params![key],
            )?;
            self.conn.execute(
                "DELETE FROM sent_messages WHERE recipient_id = ?1",
                params![key],
            )?;
            self.conn.execute(
                "DELETE FROM inbox WHERE sender_id = ?1 AND content_type = 'message'",
                params![key],
            )?;
        }

        let removed = self
            .conn
            .execute("DELETE FROM contacts WHERE id = ?1", params![id])?;
        if removed == 0 {
            return Err(CoreError::ContactNotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn get_contact_by_phone_hash(&self, phone_hash: &str) -> CoreResult<Option<Contact>> {
        Ok(self
            .list_contacts()?
            .into_iter()
            .find(|c| c.phone_hash.as_deref() == Some(phone_hash)))
    }
}
