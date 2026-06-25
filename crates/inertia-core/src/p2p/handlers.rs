use std::sync::Arc;

use libp2p::PeerId;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use crate::content::{ContentEnvelope, ContentType, DeliveryStatus, MessagePayload, PostPayload};
use crate::crypto::decrypt_from_sender;
use crate::error::CoreResult;
use crate::identity::Identity;
use crate::storage::{ConnectionState, Contact, InboxEntry, MAX_BLOB_BYTES};
use crate::store_handle::StoreHandle;

use super::events::P2pEvent;
use super::protocol::{
    BlobChunkData, BlobData, InertiaRequest, InertiaResponse, SendEnvelope,
};

pub async fn persist_peer_multiaddrs(store: &StoreHandle, peer_id: &PeerId, addrs: &[String]) {
    let peer_id = peer_id.to_string();
    let _ = store
        .with_mut(|s| s.merge_contact_multiaddrs_by_peer_id(&peer_id, addrs))
        .await;
}

pub async fn update_contact_state(store: &StoreHandle, peer_id: &PeerId, state: ConnectionState) {
    let peer_id = peer_id.to_string();
    let _ = store
        .with_mut(|store| {
            if let Ok(contacts) = store.list_contacts() {
                for mut c in contacts {
                    if c.peer_id.as_deref() == Some(&peer_id) {
                        c.connection_state = state;
                        c.last_seen = Some(chrono::Utc::now());
                        store.upsert_contact(&c)?;
                    }
                }
            }
            Ok(())
        })
        .await;
}

pub async fn touch_contact_last_seen_by_peer(store: &StoreHandle, peer_id: &PeerId) {
    let peer_id = peer_id.to_string();
    let _ = store
        .with_mut(|store| {
            if let Ok(contacts) = store.list_contacts() {
                for mut c in contacts {
                    if c.peer_id.as_deref() == Some(&peer_id) {
                        c.last_seen = Some(chrono::Utc::now());
                        store.upsert_contact(&c)?;
                    }
                }
            }
            Ok(())
        })
        .await;
}

pub async fn handle_inbound_request(
    store: &StoreHandle,
    identity: &Arc<RwLock<Identity>>,
    event_tx: &mpsc::UnboundedSender<P2pEvent>,
    peer: PeerId,
    request: InertiaRequest,
) -> CoreResult<InertiaResponse> {
    match request {
        InertiaRequest::FriendRequest(req) => {
            let _ = event_tx.send(P2pEvent::FriendRequestReceived(req));
            Ok(InertiaResponse::Ok)
        }
        InertiaRequest::FriendAccept(accept) => {
            let contact = Contact {
                id: accept.contact_id.clone(),
                phone_hash: accept.phone_hash,
                display_name: accept.display_name,
                peer_id: Some(accept.peer_id),
                signing_pubkey: accept.signing_pubkey,
                encryption_pubkey: accept.encryption_pubkey,
                last_seen: Some(chrono::Utc::now()),
                connection_state: ConnectionState::Online,
                multiaddrs: Vec::new(),
            };
            store.with_mut(|s| s.upsert_contact(&contact)).await?;
            Ok(InertiaResponse::Ok)
        }
        InertiaRequest::InviteRedemption(redemption) => {
            store
                .with_mut(|s| s.consume_issued_invite(&redemption.invite_nonce))
                .await?;
            let contact = Contact {
                id: redemption.signing_pubkey.clone(),
                phone_hash: None,
                display_name: redemption.display_name,
                peer_id: Some(redemption.peer_id),
                signing_pubkey: redemption.signing_pubkey,
                encryption_pubkey: redemption.encryption_pubkey,
                last_seen: Some(chrono::Utc::now()),
                connection_state: ConnectionState::Online,
                multiaddrs: Vec::new(),
            };
            store.with_mut(|s| s.upsert_contact(&contact)).await?;
            info!(friend = %contact.display_name, "invite redeemed");
            let _ = identity;
            Ok(InertiaResponse::Ok)
        }
        InertiaRequest::SendEnvelope(SendEnvelope { envelope }) => {
            let missing_hash =
                process_incoming_envelope(store, identity, event_tx, &envelope).await?;
            touch_contact_last_seen_by_peer(store, &peer).await;
            if let Some(hash) = missing_hash {
                if !store.with(|s| Ok(s.blob_exists(&hash))).await? {
                    let _ = event_tx.send(P2pEvent::BlobNeeded { hash, peer_id: peer });
                }
            }
            Ok(InertiaResponse::DeliveryAck(
                super::protocol::DeliveryAck {
                    content_id: envelope.id.clone(),
                },
            ))
        }
        InertiaRequest::BlobRequest(req) => {
            match store.with(|s| s.read_blob(&req.hash)).await {
                Ok(data) if data.len() <= MAX_BLOB_BYTES => Ok(InertiaResponse::BlobData(BlobData {
                    hash: req.hash,
                    data,
                })),
                _ => Ok(InertiaResponse::BlobNotFound),
            }
        }
        InertiaRequest::BlobPush(blob) => {
            store
                .with_mut(|s| s.store_blob_verified(&blob.hash, &blob.data))
                .await?;
            info!(hash = %blob.hash, bytes = blob.data.len(), %peer, "stored pushed blob");
            Ok(InertiaResponse::Ok)
        }
        InertiaRequest::BlobChunkRequest(req) => {
            match store
                .with(|s| s.read_chunk(&req.root_hash, req.chunk_index))
                .await
            {
                Ok(data) if data.len() <= crate::storage::CHUNK_SIZE => {
                    Ok(InertiaResponse::BlobChunkData(BlobChunkData {
                        root_hash: req.root_hash,
                        chunk_index: req.chunk_index,
                        data,
                    }))
                }
                _ => Ok(InertiaResponse::BlobChunkNotFound),
            }
        }
        InertiaRequest::BlobHave(_) => Ok(InertiaResponse::Ok),
    }
}

pub async fn handle_outbound_response(
    store: &StoreHandle,
    event_tx: &mpsc::UnboundedSender<P2pEvent>,
    peer: PeerId,
    response: InertiaResponse,
) -> CoreResult<()> {
    match response {
        InertiaResponse::DeliveryAck(ack) => {
            touch_contact_last_seen_by_peer(store, &peer).await;
            let _ = event_tx.send(P2pEvent::DeliveryAcked {
                content_id: ack.content_id.clone(),
                peer_id: peer,
            });
            let content_id = ack.content_id.clone();
            store
                .with_mut(|s| {
                    let entries = s.list_outbox()?;
                    for entry in entries {
                        if entry.content_id == content_id {
                            s.update_outbox_status(
                                &content_id,
                                &entry.recipient_id,
                                DeliveryStatus::Delivered,
                            )?;
                            s.record_ack(&content_id, &entry.recipient_id)?;
                            break;
                        }
                    }
                    Ok(())
                })
                .await?;
        }
        InertiaResponse::FriendAccept(accept) => {
            let contact = Contact {
                id: accept.contact_id,
                phone_hash: accept.phone_hash,
                display_name: accept.display_name,
                peer_id: Some(accept.peer_id),
                signing_pubkey: accept.signing_pubkey,
                encryption_pubkey: accept.encryption_pubkey,
                last_seen: Some(chrono::Utc::now()),
                connection_state: ConnectionState::Online,
                multiaddrs: Vec::new(),
            };
            store.with_mut(|s| s.upsert_contact(&contact)).await?;
        }
        InertiaResponse::Ok => {}
        InertiaResponse::Error(msg) => {
            warn!(error = %msg, "peer returned error");
        }
        InertiaResponse::BlobData(_) | InertiaResponse::BlobNotFound => {}
        InertiaResponse::BlobChunkData(_) | InertiaResponse::BlobChunkNotFound => {}
    }
    Ok(())
}

async fn process_incoming_envelope(
    store: &StoreHandle,
    identity: &Arc<RwLock<Identity>>,
    event_tx: &mpsc::UnboundedSender<P2pEvent>,
    envelope: &ContentEnvelope,
) -> CoreResult<Option<String>> {
    if !crate::identity::Identity::verify_signature(
        &envelope.author_signing_pubkey,
        &envelope.signing_bytes(),
        &envelope.signature,
    )? {
        return Err(crate::error::CoreError::Crypto(
            "invalid envelope signature".into(),
        ));
    }

    let id = identity.read().await;
    let plaintext = decrypt_from_sender(
        id.encryption_secret()?,
        &envelope.author_encryption_pubkey,
        &envelope.ciphertext,
    )?;
    drop(id);

    let (body, media_ref, sync_hash) = match envelope.content_type {
        ContentType::Message => {
            let payload: MessagePayload = serde_json::from_slice(&plaintext)?;
            (payload.body, None, None)
        }
        ContentType::Post => {
            let payload: PostPayload = serde_json::from_slice(&plaintext)?;
            if let Some(ref manifest) = payload.manifest {
                store
                    .with_mut(|s| s.insert_manifest(manifest))
                    .await?;
            }
            let media_ref = payload.media_ref;
            let sync_hash = payload.thumb_ref.or_else(|| media_ref.clone());
            (payload.body, media_ref, sync_hash)
        }
        ContentType::Comment => {
            let payload: crate::content::CommentPayload = serde_json::from_slice(&plaintext)?;
            let sender_id = envelope.author_signing_pubkey.clone();
            let author_name = store
                .with(|s| s.list_contacts())
                .await
                .ok()
                .and_then(|contacts| {
                    contacts
                        .into_iter()
                        .find(|c| c.id == sender_id || c.signing_pubkey == sender_id)
                        .map(|c| c.display_name)
                })
                .unwrap_or_else(|| "Friend".to_string());

            let comment = crate::storage::PostComment {
                id: envelope.id.clone(),
                post_id: payload.post_id,
                author_id: sender_id,
                author_name,
                body: payload.body,
                created_at: envelope.created_at,
            };
            store
                .with_mut(|s| s.insert_post_comment(&comment))
                .await?;
            return Ok(None);
        }
    };

    let sender_id = envelope.author_signing_pubkey.clone();
    store
        .with_mut(|s| {
            s.insert_inbox(&InboxEntry {
                content_id: envelope.id.clone(),
                sender_id: sender_id.clone(),
                received_at: envelope.created_at,
                expires_at: envelope.expires_at,
                read_at: None,
                body: body.clone(),
                media_ref: media_ref.clone(),
                content_type: envelope.content_type,
            })
        })
        .await?;

    if envelope.content_type == ContentType::Post {
        let author_name = store
            .with(|s| s.list_contacts())
            .await
            .ok()
            .and_then(|contacts| {
                contacts
                    .into_iter()
                    .find(|c| c.id == sender_id || c.signing_pubkey == sender_id)
                    .map(|c| c.display_name)
            })
            .unwrap_or_else(|| "Friend".to_string());

        let archive_item = crate::storage::ArchivedFeedItem {
            content_id: envelope.id.clone(),
            author_id: sender_id.clone(),
            author_name,
            body: body.clone(),
            media_ref: media_ref.clone(),
            created_at: envelope.created_at,
            is_own: false,
        };
        let _ = store
            .with_mut(|s| s.try_archive_feed_item(&archive_item))
            .await;
    }

    let _ = event_tx.send(P2pEvent::MessageReceived { sender_id, body });
    Ok(sync_hash)
}
