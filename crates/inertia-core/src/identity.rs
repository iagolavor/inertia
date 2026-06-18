use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519Public, StaticSecret};

use crate::error::{CoreError, CoreResult};

/// Default post lifetime: 7 days.
pub const POST_TTL_SECS: i64 = 7 * 24 * 60 * 60;
/// Message lifetime: 7 days.
pub const MESSAGE_TTL_SECS: i64 = 7 * 24 * 60 * 60;

#[derive(Clone, Serialize, Deserialize)]
pub struct Identity {
    pub signing_pubkey: String,
    pub encryption_pubkey: String,
    pub phone_hash: Option<String>,
    pub display_name: String,
    #[serde(default)]
    pub bio: String,
    #[serde(skip)]
    signing_key: Option<SigningKey>,
    #[serde(skip)]
    encryption_secret: Option<StaticSecret>,
}

impl Identity {
    pub fn generate(display_name: impl Into<String>) -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let encryption_secret = StaticSecret::random_from_rng(OsRng);
        let encryption_pubkey = X25519Public::from(&encryption_secret);

        Self {
            signing_pubkey: hex::encode(signing_key.verifying_key().as_bytes()),
            encryption_pubkey: hex::encode(encryption_pubkey.as_bytes()),
            phone_hash: None,
            display_name: display_name.into(),
            bio: String::new(),
            signing_key: Some(signing_key),
            encryption_secret: Some(encryption_secret),
        }
    }

    pub fn from_persisted(
        signing_pubkey: String,
        encryption_pubkey: String,
        phone_hash: Option<String>,
        display_name: String,
        bio: String,
        signing_key_hex: Option<String>,
        encryption_secret_hex: Option<String>,
    ) -> CoreResult<Self> {
        let signing_key = signing_key_hex
            .map(|hex| -> CoreResult<SigningKey> {
                let bytes = decode_hex(&hex).map_err(|e| CoreError::Crypto(e))?;
                let arr: [u8; 32] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| CoreError::Crypto("invalid signing key length".into()))?;
                Ok(SigningKey::from_bytes(&arr))
            })
            .transpose()?;

        let encryption_secret = encryption_secret_hex
            .map(|hex| -> CoreResult<StaticSecret> {
                let bytes = decode_hex(&hex).map_err(|e| CoreError::Crypto(e))?;
                let arr: [u8; 32] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| CoreError::Crypto("invalid encryption secret length".into()))?;
                Ok(StaticSecret::from(arr))
            })
            .transpose()?;

        Ok(Self {
            signing_pubkey,
            encryption_pubkey,
            phone_hash,
            display_name,
            bio,
            signing_key,
            encryption_secret,
        })
    }

    pub fn is_initialized(&self) -> bool {
        !self.display_name.is_empty()
            && self.signing_key.is_some()
            && self.encryption_secret.is_some()
    }

    pub fn bind_phone(&mut self, phone: &str) -> String {
        let hash = hash_phone(phone);
        self.phone_hash = Some(hash.clone());
        hash
    }

    pub fn signing_key(&self) -> CoreResult<&SigningKey> {
        self.signing_key
            .as_ref()
            .ok_or(CoreError::IdentityNotInitialized)
    }

    pub fn encryption_secret(&self) -> CoreResult<&StaticSecret> {
        self.encryption_secret
            .as_ref()
            .ok_or(CoreError::IdentityNotInitialized)
    }

    pub fn sign(&self, message: &[u8]) -> CoreResult<Vec<u8>> {
        Ok(self.signing_key()?.sign(message).to_bytes().to_vec())
    }

    pub fn verify_signature(
        signing_pubkey_hex: &str,
        message: &[u8],
        signature: &[u8],
    ) -> CoreResult<bool> {
        let pubkey_bytes = hex::decode(signing_pubkey_hex)
            .map_err(|e| CoreError::Crypto(e.to_string()))?;
        let verifying_key = VerifyingKey::from_bytes(
            pubkey_bytes
                .as_slice()
                .try_into()
                .map_err(|_| CoreError::Crypto("invalid signing pubkey".into()))?,
        )
        .map_err(|e| CoreError::Crypto(e.to_string()))?;

        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| CoreError::Crypto("invalid signature length".into()))?;

        Ok(verifying_key.verify(message, &ed25519_dalek::Signature::from_bytes(&sig_bytes)).is_ok())
    }
}

pub fn hash_phone(phone: &str) -> String {
    let normalized: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    pub fn decode(s: &str) -> std::result::Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("odd length hex".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}

pub(crate) use hex::{decode as decode_hex, encode as encode_hex};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_and_signs() {
        let id = Identity::generate("Alice");
        let sig = id.sign(b"hello").unwrap();
        assert!(Identity::verify_signature(&id.signing_pubkey, b"hello", &sig).unwrap());
    }

    #[test]
    fn phone_hash_is_stable() {
        assert_eq!(hash_phone("+1 (555) 123-4567"), hash_phone("15551234567"));
    }
}
