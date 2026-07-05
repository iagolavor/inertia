use chrono::Utc;

use crate::error::CoreResult;
use crate::storage::{ConnectionState, Contact};

use super::{ConversationMessage, Engine};

const REACHABLE_WINDOW: chrono::Duration = chrono::Duration::hours(24);

fn recently_reachable(contact: &Contact) -> bool {
    contact.last_seen.is_some_and(|seen| {
        Utc::now().signed_duration_since(seen) <= REACHABLE_WINDOW
    })
}

/// Live libp2p session + recent `last_seen` for API responses (reachable tier not persisted).
pub(crate) fn presence_for_contact(
    contact: &Contact,
    connected_friend_peer_ids: &std::collections::HashSet<String>,
) -> ConnectionState {
    if contact
        .peer_id
        .as_ref()
        .is_some_and(|id| connected_friend_peer_ids.contains(id))
    {
        return ConnectionState::Online;
    }
    if contact.peer_id.is_none() && contact.multiaddrs.is_empty() {
        return ConnectionState::Unreachable;
    }
    if recently_reachable(contact) {
        return ConnectionState::Reachable;
    }
    ConnectionState::Unreachable
}

impl Engine {
    #[allow(dead_code)]
    pub async fn add_pending_contact(
        &self,
        contact_id: &str,
        display_name: &str,
        signing_pubkey: &str,
        encryption_pubkey: &str,
    ) -> CoreResult<Contact> {
        let contact = Contact {
            id: contact_id.to_string(),
            phone_hash: None,
            display_name: display_name.to_string(),
            peer_id: None,
            signing_pubkey: signing_pubkey.to_string(),
            encryption_pubkey: encryption_pubkey.to_string(),
            last_seen: None,
            connection_state: ConnectionState::Offline,
            multiaddrs: Vec::new(),
        };
        self.store
            .with_mut(|store| store.upsert_contact(&contact))
            .await?;
        Ok(contact)
    }

    pub async fn list_contacts(&self) -> CoreResult<Vec<Contact>> {
        let mut contacts = self.store.with(|store| store.list_contacts()).await?;
        let connected = self.connected_friend_peer_ids().await;
        for contact in &mut contacts {
            contact.connection_state = presence_for_contact(contact, &connected);
        }
        Ok(contacts)
    }

    pub async fn get_contact(&self, contact_id: &str) -> CoreResult<Contact> {
        let mut contact = self
            .store
            .with(|store| store.get_contact(contact_id))
            .await?;
        let connected = self.connected_friend_peer_ids().await;
        contact.connection_state = presence_for_contact(&contact, &connected);
        Ok(contact)
    }

    pub async fn list_conversation_messages(
        &self,
        contact_id: &str,
    ) -> CoreResult<Vec<ConversationMessage>> {
        let contact = self
            .store
            .with(|store| store.get_contact(contact_id))
            .await?;

        let mut received = self
            .store
            .with(|store| store.list_inbox_messages_from_sender(&contact.id))
            .await?;
        if received.is_empty() && contact.signing_pubkey != contact.id {
            received = self
                .store
                .with(|store| store.list_inbox_messages_from_sender(&contact.signing_pubkey))
                .await?;
        }
        let mut sent = self
            .store
            .with(|store| store.list_sent_messages_for_recipient(&contact.id))
            .await?;
        if sent.is_empty() && contact.signing_pubkey != contact.id {
            sent = self
                .store
                .with(|store| store.list_sent_messages_for_recipient(&contact.signing_pubkey))
                .await?;
        }

        let mut messages: Vec<ConversationMessage> = received
            .into_iter()
            .map(|entry| ConversationMessage {
                content_id: entry.content_id,
                body: entry.body,
                at: entry.received_at,
                expires_at: entry.expires_at,
                is_own: false,
                delivery_status: None,
            })
            .chain(sent.into_iter().map(|entry| ConversationMessage {
                content_id: entry.content_id,
                body: entry.body,
                at: entry.sent_at,
                expires_at: entry.expires_at,
                is_own: true,
                delivery_status: Some(entry.status),
            }))
            .collect();

        messages.sort_by_key(|m| m.at);
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::presence_for_contact;
    use crate::storage::{ConnectionState, Contact};
    use chrono::Utc;
    use std::collections::HashSet;

    fn sample_contact(
        peer_id: Option<&str>,
        multiaddrs: Vec<String>,
        last_seen: Option<chrono::DateTime<Utc>>,
    ) -> Contact {
        Contact {
            id: "c1".into(),
            phone_hash: None,
            display_name: "Alex".into(),
            peer_id: peer_id.map(str::to_string),
            signing_pubkey: "pk".into(),
            encryption_pubkey: "ek".into(),
            last_seen,
            connection_state: ConnectionState::Offline,
            multiaddrs,
        }
    }

    #[test]
    fn live_peer_session_is_online() {
        let contact = sample_contact(Some("12D3KooW"), vec![], None);
        let mut connected = HashSet::new();
        connected.insert("12D3KooW".into());
        assert_eq!(
            presence_for_contact(&contact, &connected),
            ConnectionState::Online
        );
    }

    #[test]
    fn recent_last_seen_without_live_session_is_reachable() {
        let contact = sample_contact(
            Some("12D3KooW"),
            vec!["/ip4/1.2.3.4/tcp/9000".into()],
            Some(Utc::now() - chrono::Duration::hours(1)),
        );
        assert_eq!(
            presence_for_contact(&contact, &HashSet::new()),
            ConnectionState::Reachable
        );
    }

    #[test]
    fn stale_last_seen_without_live_session_is_unreachable() {
        let contact = sample_contact(
            Some("12D3KooW"),
            vec!["/ip4/1.2.3.4/tcp/9000".into()],
            Some(Utc::now() - chrono::Duration::hours(25)),
        );
        assert_eq!(
            presence_for_contact(&contact, &HashSet::new()),
            ConnectionState::Unreachable
        );
    }

    #[test]
    fn no_peer_hints_is_unreachable() {
        let contact = sample_contact(None, vec![], None);
        assert_eq!(
            presence_for_contact(&contact, &HashSet::new()),
            ConnectionState::Unreachable
        );
    }
}
