use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::identity::Identity;
use crate::storage::{ConnectionState, Contact};

/// Invite links expire after 15 minutes.
pub const INVITE_TTL_SECS: i64 = 15 * 60;

const INVITE_VERSION: u8 = 1;
const INVITE_SCHEME: &str = "inertia://invite/";
const WEB_INVITE_PREFIX: &str = "/invite#";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendInvite {
    pub version: u8,
    pub display_name: String,
    pub signing_pubkey: String,
    pub encryption_pubkey: String,
    pub peer_id: Option<String>,
    pub multiaddrs: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub nonce: String,
    pub signature: String,
}

impl FriendInvite {
    pub fn new(
        identity: &Identity,
        peer_id: Option<String>,
        multiaddrs: Vec<String>,
    ) -> CoreResult<Self> {
        let now = Utc::now();
        let mut invite = Self {
            version: INVITE_VERSION,
            display_name: identity.display_name.clone(),
            signing_pubkey: identity.signing_pubkey.clone(),
            encryption_pubkey: identity.encryption_pubkey.clone(),
            peer_id,
            multiaddrs,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(INVITE_TTL_SECS),
            nonce: Uuid::new_v4().to_string(),
            signature: String::new(),
        };
        let sig = identity.sign(&invite.signing_bytes()?)?;
        invite.signature = hex::encode(sig);
        Ok(invite)
    }

    pub fn signing_bytes(&self) -> CoreResult<Vec<u8>> {
        Ok(serde_json::to_vec(&InviteSigningPayload {
            version: self.version,
            display_name: &self.display_name,
            signing_pubkey: &self.signing_pubkey,
            encryption_pubkey: &self.encryption_pubkey,
            peer_id: self.peer_id.as_deref(),
            multiaddrs: &self.multiaddrs,
            created_at: self.created_at,
            expires_at: self.expires_at,
            nonce: &self.nonce,
        })?)
    }

    pub fn verify(&self) -> CoreResult<()> {
        if self.version != INVITE_VERSION {
            return Err(CoreError::Crypto("unsupported invite version".into()));
        }
        if Utc::now() > self.expires_at {
            return Err(CoreError::Invite("invite has expired".into()));
        }
        let sig = hex::decode(&self.signature)
            .map_err(|e| CoreError::Crypto(e.to_string()))?;
        let valid = Identity::verify_signature(
            &self.signing_pubkey,
            &self.signing_bytes()?,
            &sig,
        )?;
        if !valid {
            return Err(CoreError::Crypto("invalid invite signature".into()));
        }
        Ok(())
    }

    pub fn to_payload(&self) -> CoreResult<String> {
        let json = serde_json::to_vec(self)?;
        Ok(URL_SAFE_NO_PAD.encode(json))
    }

    pub fn to_link(&self, web_origin: Option<&str>) -> CoreResult<String> {
        let payload = self.to_payload()?;
        if let Some(origin) = web_origin {
            Ok(format!("{origin}{WEB_INVITE_PREFIX}{payload}"))
        } else {
            Ok(format!("{INVITE_SCHEME}{payload}"))
        }
    }

    pub fn parse(input: &str) -> CoreResult<Self> {
        let payload = extract_payload(input)?;
        let json = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|e| CoreError::Crypto(e.to_string()))?;
        let invite: Self = serde_json::from_slice(&json)?;
        invite.verify()?;
        Ok(invite)
    }

    pub fn to_contact(&self) -> Contact {
        Contact {
            id: self.signing_pubkey.clone(),
            phone_hash: None,
            display_name: self.display_name.clone(),
            peer_id: self.peer_id.clone(),
            signing_pubkey: self.signing_pubkey.clone(),
            encryption_pubkey: self.encryption_pubkey.clone(),
            last_seen: None,
            connection_state: ConnectionState::Offline,
        }
    }

    /// Short code derived from signing key for out-of-band verification.
    pub fn safety_code(&self) -> String {
        self.signing_pubkey.chars().take(8).collect()
    }
}

#[derive(Serialize)]
struct InviteSigningPayload<'a> {
    version: u8,
    display_name: &'a str,
    signing_pubkey: &'a str,
    encryption_pubkey: &'a str,
    peer_id: Option<&'a str>,
    multiaddrs: &'a [String],
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    nonce: &'a str,
}

fn extract_payload(input: &str) -> CoreResult<String> {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix(INVITE_SCHEME) {
        return Ok(rest.to_string());
    }
    if let Some(idx) = trimmed.find(WEB_INVITE_PREFIX) {
        return Ok(trimmed[idx + WEB_INVITE_PREFIX.len()..].to_string());
    }
    if let Some(hash_idx) = trimmed.find('#') {
        let fragment = &trimmed[hash_idx + 1..];
        if !fragment.is_empty() {
            return Ok(fragment.to_string());
        }
    }
    if let Some(query) = trimmed.split('?').nth(1) {
        for part in query.split('&') {
            if let Some(val) = part.strip_prefix("d=") {
                return Ok(val.to_string());
            }
        }
    }
    Ok(trimmed.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_invite() {
        let id = Identity::generate("Alice");
        let invite = FriendInvite::new(
            &id,
            Some("12D3KooW".into()),
            vec!["/ip4/127.0.0.1/tcp/4001".into()],
        )
        .unwrap();
        let link = invite.to_link(Some("http://localhost:5173")).unwrap();
        let parsed = FriendInvite::parse(&link).unwrap();
        assert_eq!(parsed.display_name, "Alice");
        assert_eq!(parsed.safety_code(), invite.safety_code());
    }
}
