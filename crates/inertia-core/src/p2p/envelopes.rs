use crate::content::ContentEnvelope;
use crate::crypto::encrypt_for_recipient;
use crate::error::CoreResult;
use crate::identity::Identity;
use crate::storage::Contact;

pub fn build_post_envelope(
    identity: &Identity,
    recipient: &Contact,
    content_id: &str,
    body: &str,
    media_ref: Option<&str>,
) -> CoreResult<ContentEnvelope> {
    let payload = crate::content::PostPayload {
        body: body.to_string(),
        media_ref: media_ref.map(|s| s.to_string()),
    };
    let plaintext = serde_json::to_vec(&payload)?;
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
