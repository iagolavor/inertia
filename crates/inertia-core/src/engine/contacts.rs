use crate::error::CoreResult;
use crate::storage::{ConnectionState, Contact};

use super::{ConversationMessage, Engine};

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
        self.store.with(|store| store.list_contacts()).await
    }

    pub async fn list_conversation_messages(
        &self,
        contact_id: &str,
    ) -> CoreResult<Vec<ConversationMessage>> {
        self.store
            .with(|store| store.get_contact(contact_id))
            .await?;

        let received = self
            .store
            .with(|store| store.list_inbox_messages_from_sender(contact_id))
            .await?;
        let sent = self
            .store
            .with(|store| store.list_sent_messages_for_recipient(contact_id))
            .await?;

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
