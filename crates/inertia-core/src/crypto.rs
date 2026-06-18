use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::RngCore;
use x25519_dalek::{PublicKey as X25519Public, StaticSecret};

use crate::error::{CoreError, CoreResult};

pub fn encrypt_for_recipient(
    sender_secret: &StaticSecret,
    recipient_pubkey_hex: &str,
    plaintext: &[u8],
) -> CoreResult<Vec<u8>> {
    let recipient_bytes = hex_decode(recipient_pubkey_hex)?;
    let recipient_pubkey: [u8; 32] = recipient_bytes
        .try_into()
        .map_err(|_| CoreError::Crypto("invalid encryption pubkey".into()))?;
    let shared = sender_secret.diffie_hellman(&X25519Public::from(recipient_pubkey));

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let cipher = ChaCha20Poly1305::new_from_slice(shared.as_bytes())
        .map_err(|e| CoreError::Crypto(e.to_string()))?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|e| CoreError::Crypto(e.to_string()))?;

    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_from_sender(
    recipient_secret: &StaticSecret,
    sender_pubkey_hex: &str,
    ciphertext: &[u8],
) -> CoreResult<Vec<u8>> {
    if ciphertext.len() < 12 {
        return Err(CoreError::Crypto("ciphertext too short".into()));
    }
    let sender_bytes = hex_decode(sender_pubkey_hex)?;
    let sender_pubkey: [u8; 32] = sender_bytes
        .try_into()
        .map_err(|_| CoreError::Crypto("invalid sender pubkey".into()))?;
    let shared = recipient_secret.diffie_hellman(&X25519Public::from(sender_pubkey));

    let cipher = ChaCha20Poly1305::new_from_slice(shared.as_bytes())
        .map_err(|e| CoreError::Crypto(e.to_string()))?;
    cipher
        .decrypt(
            Nonce::from_slice(&ciphertext[..12]),
            &ciphertext[12..],
        )
        .map_err(|e| CoreError::Crypto(e.to_string()))
}

fn hex_decode(s: &str) -> CoreResult<Vec<u8>> {
    if s.len() % 2 != 0 {
        return Err(CoreError::Crypto("odd length hex".into()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| CoreError::Crypto(e.to_string()))
        })
        .collect()
}
