use crate::content::{ContentEnvelope, PostPayload};
use crate::crypto::encrypt_for_recipient;
use crate::error::CoreResult;
use crate::identity::Identity;
use crate::storage::Contact;

pub fn build_post_envelope(
    identity: &Identity,
    recipient: &Contact,
    content_id: &str,
    payload: &PostPayload,
) -> CoreResult<ContentEnvelope> {
    let plaintext = serde_json::to_vec(payload)?;
    let ciphertext = encrypt_for_recipient(
        identity.encryption_secret()?,
        &recipient.encryption_pubkey,
        &plaintext,
    )?;

    let mut envelope = ContentEnvelope::new_post(
        identity.signing_pubkey.clone(),
        identity.encryption_pubkey.clone(),
        ciphertext,
        vec![],
    );
    envelope.id = content_id.to_string();
    envelope.signature = identity.sign(&envelope.signing_bytes())?;
    Ok(envelope)
}

pub fn build_message_envelope(
    identity: &Identity,
    recipient: &Contact,
    body: &str,
    thread_id: &str,
) -> CoreResult<ContentEnvelope> {
    let payload = crate::content::MessagePayload {
        body: body.to_string(),
        thread_id: thread_id.to_string(),
    };
    let plaintext = serde_json::to_vec(&payload)?;
    let ciphertext = encrypt_for_recipient(
        identity.encryption_secret()?,
        &recipient.encryption_pubkey,
        &plaintext,
    )?;

    let mut envelope = ContentEnvelope::new_message(
        identity.signing_pubkey.clone(),
        identity.encryption_pubkey.clone(),
        ciphertext,
        vec![],
    );
    envelope.signature = identity.sign(&envelope.signing_bytes())?;
    Ok(envelope)
}

pub fn build_comment_envelope(
    identity: &Identity,
    recipient: &Contact,
    post_id: &str,
    body: &str,
) -> CoreResult<ContentEnvelope> {
    let payload = crate::content::CommentPayload {
        post_id: post_id.to_string(),
        body: body.to_string(),
    };
    let plaintext = serde_json::to_vec(&payload)?;
    let ciphertext = encrypt_for_recipient(
        identity.encryption_secret()?,
        &recipient.encryption_pubkey,
        &plaintext,
    )?;

    let mut envelope = ContentEnvelope::new_comment(
        identity.signing_pubkey.clone(),
        identity.encryption_pubkey.clone(),
        ciphertext,
        vec![],
    );
    envelope.signature = identity.sign(&envelope.signing_bytes())?;
    Ok(envelope)
}

pub fn build_profile_comment_envelope(
    identity: &Identity,
    recipient: &Contact,
    profile_item_id: &str,
    body: &str,
) -> CoreResult<ContentEnvelope> {
    let payload = crate::content::ProfileCommentPayload {
        profile_item_id: profile_item_id.to_string(),
        body: body.to_string(),
    };
    let plaintext = serde_json::to_vec(&payload)?;
    let ciphertext = encrypt_for_recipient(
        identity.encryption_secret()?,
        &recipient.encryption_pubkey,
        &plaintext,
    )?;

    let mut envelope = ContentEnvelope::new_profile_comment(
        identity.signing_pubkey.clone(),
        identity.encryption_pubkey.clone(),
        ciphertext,
        vec![],
    );
    envelope.signature = identity.sign(&envelope.signing_bytes())?;
    Ok(envelope)
}
