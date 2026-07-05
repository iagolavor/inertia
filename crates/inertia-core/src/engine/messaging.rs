use crate::content::{ContentType, DeliveryStatus};
use crate::error::CoreResult;
use crate::p2p::build_message_envelope;
use crate::storage::OutboxEntry;

use super::Engine;

impl Engine {
    pub async fn send_message(&self, recipient_id: &str, body: &str) -> CoreResult<String> {
        let identity = self.identity.read().await;
        let recipient = self
            .store
            .with(|store| store.get_contact(recipient_id))
            .await?;

        let envelope = build_message_envelope(&identity, &recipient, body, recipient_id)?;
        drop(identity);

        let envelope_json = serde_json::to_string(&envelope)?;
        let content_id = envelope.id.clone();

        self.store
            .with_mut(|store| {
                store.insert_outbox(
                    &OutboxEntry {
                        content_id: content_id.clone(),
                        recipient_id: recipient_id.to_string(),
                        status: DeliveryStatus::Pending,
                        expires_at: envelope.expires_at,
                        retry_count: 0,
                        ciphertext: envelope.ciphertext.clone(),
                        content_type: ContentType::Message,
                    },
                    &envelope_json,
                )?;
                store.insert_sent_message(
                    &content_id,
                    recipient_id,
                    body,
                    envelope.created_at,
                    envelope.expires_at,
                    DeliveryStatus::Pending,
                )
            })
            .await?;

        if let Some(peer_id_str) = recipient.peer_id.as_ref() {
            if let Ok(peer_id) = peer_id_str.parse() {
                let p2p_guard = self.p2p.lock().await;
                if let Some(p2p) = p2p_guard.as_ref() {
                    if p2p.send_envelope_to_peer(peer_id, envelope).await.is_ok() {
                        self.store
                            .with_mut(|store| {
                                store.update_outbox_status(
                                    &content_id,
                                    recipient_id,
                                    DeliveryStatus::Sent,
                                )
                            })
                            .await?;
                        self.emit_message_sent_ui(
                            &content_id,
                            recipient_id,
                            ContentType::Message,
                        )
                        .await;
                        return Ok(content_id);
                    }
                }
            }
        }

        self.store
            .with_mut(|store| {
                store.update_outbox_status(&content_id, recipient_id, DeliveryStatus::Failed)
            })
            .await?;

        Ok(content_id)
    }
}
