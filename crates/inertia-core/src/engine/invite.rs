use tracing::{info, warn};

use libp2p::PeerId;
use std::time::Duration;

use crate::error::{CoreError, CoreResult};
use crate::invite::FriendInvite;
use crate::storage::Contact;

use super::p2p::{peer_id_from_multiaddr_str, validate_relay_multiaddr};
use super::{Engine, InvitePreview, InviteResponse};

impl Engine {
    pub async fn create_invite(&self, _web_origin: Option<&str>) -> CoreResult<InviteResponse> {
        let identity = self.identity.read().await;
        if identity.display_name.is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }
        drop(identity);

        let status = self.p2p_status().await;
        if !status.relay_configured {
            return Err(CoreError::Invite(
                "relay multiaddr is not configured".into(),
            ));
        }
        if status.relay_tcp_reachable == Some(false) {
            return Err(CoreError::Invite(
                "connect to the relay in Settings before inviting (header must show Relay OK)"
                    .into(),
            ));
        }
        if !status.relay_connected {
            return Err(CoreError::Invite(
                "connect to the relay in Settings before inviting (header must show Relay OK)"
                    .into(),
            ));
        }
        let relay_multiaddr = self.effective_relay().await.ok_or_else(|| {
            CoreError::Invite("relay multiaddr is not configured".into())
        })?;
        validate_relay_multiaddr(&relay_multiaddr)?;

        let peer_id = self.peer_id().await;
        let multiaddrs = self.p2p_invite_addresses(peer_id.as_deref()).await;
        if multiaddrs.is_empty() {
            return Err(CoreError::Invite(
                "no routable P2P addresses — check relay connection and try again".into(),
            ));
        }
        let identity = self.identity.read().await;
        let invite = FriendInvite::new(
            &identity,
            peer_id,
            multiaddrs,
            relay_multiaddr,
        )?;
        drop(identity);

        self.store
            .with_mut(|store| {
                store.register_issued_invite(&invite.nonce, invite.expires_at)
            })
            .await?;

        let payload = invite.to_payload()?;
        // Share link uses Settings / INERTIA_WEB_ORIGIN only — not the browser tab URL (localhost dev).
        let settings_origin = self.resolve_invite_web_origin(None).await;
        let share_origin = settings_origin
            .as_deref()
            .filter(|origin| !is_local_dev_origin(origin));
        let link = invite.to_link(share_origin)?;
        let safety_code = invite.safety_code();
        Ok(InviteResponse {
            link,
            payload,
            safety_code,
            expires_at: invite.expires_at,
            display_name: invite.display_name,
        })
    }

    pub async fn preview_invite(&self, input: &str) -> CoreResult<InvitePreview> {
        let invite = FriendInvite::parse(input)?;
        let safety_code = invite.safety_code();
        Ok(InvitePreview {
            display_name: invite.display_name,
            signing_pubkey: invite.signing_pubkey,
            safety_code,
            expires_at: invite.expires_at,
            peer_id: invite.peer_id,
            multiaddrs: invite.multiaddrs,
            relay_multiaddr: invite.relay_multiaddr.clone(),
        })
    }

    pub async fn accept_invite(&self, input: &str) -> CoreResult<Contact> {
        let invite = FriendInvite::parse(input)?;

        if self
            .store
            .with(|store| store.is_invite_redeemed_locally(&invite.nonce))
            .await?
        {
            return Err(CoreError::Invite(
                "you already accepted this invite on this device".into(),
            ));
        }

        self.apply_relay_from_invite(&invite.relay_multiaddr).await?;
        let _ = self.ensure_relay_connected().await;
        if let Err(e) = self.redial_known_peers().await {
            warn!(error = %e, "redial after applying invite relay failed");
        }

        if let Some(relay) = self.effective_relay().await {
            if let Some(relay_peer_id) = peer_id_from_multiaddr_str(&relay) {
                self.wait_for_peer_connected(&relay_peer_id, Duration::from_secs(20), "relay")
                    .await?;
            }
        }

        let peer_id_str = invite.peer_id.as_ref().ok_or_else(|| {
            CoreError::Invite(
                "inviter is not reachable — they must be online with P2P running".into(),
            )
        })?;

        if invite.multiaddrs.is_empty() {
            return Err(CoreError::Invite(
                "inviter has no connection addresses — they must be online".into(),
            ));
        }

        {
            let p2p_guard = self.p2p.lock().await;
            let p2p = p2p_guard.as_ref().ok_or_else(|| {
                CoreError::P2p("p2p not started".into())
            })?;
            for addr_str in &invite.multiaddrs {
                if let Ok(addr) = addr_str.parse() {
                    let _ = p2p.dial(addr).await;
                }
            }
        }

        self.wait_for_peer_connected(peer_id_str, Duration::from_secs(30), "inviter")
            .await?;

        let peer_id = peer_id_str
            .parse::<PeerId>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;

        let identity = self.identity.read().await;
        let redemption = crate::p2p::InviteRedemption {
            invite_nonce: invite.nonce.clone(),
            display_name: identity.display_name.clone(),
            signing_pubkey: identity.signing_pubkey.clone(),
            encryption_pubkey: identity.encryption_pubkey.clone(),
            peer_id: {
                let p2p_guard = self.p2p.lock().await;
                let p2p = p2p_guard.as_ref().ok_or_else(|| {
                    CoreError::P2p("p2p not started".into())
                })?;
                p2p.peer_id_string()
            },
        };
        drop(identity);

        {
            let p2p_guard = self.p2p.lock().await;
            let p2p = p2p_guard.as_ref().ok_or_else(|| {
                CoreError::P2p("p2p not started".into())
            })?;
            p2p.redeem_invite(peer_id, redemption)
                .await
                .map_err(map_invite_dial_error)?;
        }

        let contact = invite.to_contact();
        let nonce = invite.nonce.clone();
        let issuer = invite.signing_pubkey.clone();
        self.store
            .with_mut(|store| {
                store.upsert_contact(&contact)?;
                store.mark_invite_redeemed_locally(&nonce, &issuer)
            })
            .await?;

        info!(friend = %contact.display_name, "invite accepted");
        Ok(contact)
    }
}

fn map_invite_dial_error(err: CoreError) -> CoreError {
    let msg = err.to_string();
    if msg.contains("failed to dial") {
        CoreError::Invite(
            "could not reach the inviter — both sides need Relay OK in the header and the inviter must stay online"
                .into(),
        )
    } else {
        err
    }
}

/// Origins that only work on the dev machine — use `inertia://` links for cross-device sharing.
pub(super) fn is_local_dev_origin(origin: &str) -> bool {
    let lower = origin.trim().to_lowercase();
    if lower.contains("localhost") {
        return true;
    }
    if lower == "http://127.0.0.1:4783"
        || lower.starts_with("http://127.0.0.1:4783/")
        || lower == "https://127.0.0.1:4783"
        || lower.starts_with("https://127.0.0.1:4783/")
    {
        return false;
    }
    lower.starts_with("http://127.0.0.1:") || lower.starts_with("https://127.0.0.1:")
}

#[cfg(test)]
mod origin_tests {
    use super::is_local_dev_origin;

    #[test]
    fn flags_localhost_dev() {
        assert!(is_local_dev_origin("http://localhost:5173"));
        assert!(is_local_dev_origin("http://127.0.0.1:4173"));
    }

    #[test]
    fn allows_phone_stage_b_origin() {
        assert!(!is_local_dev_origin("http://127.0.0.1:4783"));
    }
}
