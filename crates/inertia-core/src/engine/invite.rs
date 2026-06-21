use tracing::{info, warn};

use crate::error::{CoreError, CoreResult};
use crate::invite::FriendInvite;
use crate::storage::Contact;

use super::p2p::validate_relay_multiaddr;
use super::{Engine, InvitePreview, InviteResponse};

impl Engine {
    pub async fn create_invite(&self, web_origin: Option<&str>) -> CoreResult<InviteResponse> {
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
        let web_origin = self.resolve_invite_web_origin(web_origin).await;
        let link = invite.to_link(web_origin.as_deref())?;
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
        if let Err(e) = self.redial_known_peers().await {
            warn!(error = %e, "redial after applying invite relay failed");
        }

        let p2p_guard = self.p2p.lock().await;
        let p2p = p2p_guard.as_ref().ok_or_else(|| {
            CoreError::P2p("p2p not started".into())
        })?;

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

        for addr_str in &invite.multiaddrs {
            if let Ok(addr) = addr_str.parse() {
                let _ = p2p.dial(addr).await;
            }
        }

        let peer_id = peer_id_str
            .parse::<libp2p::PeerId>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;

        let identity = self.identity.read().await;
        let redemption = crate::p2p::InviteRedemption {
            invite_nonce: invite.nonce.clone(),
            display_name: identity.display_name.clone(),
            signing_pubkey: identity.signing_pubkey.clone(),
            encryption_pubkey: identity.encryption_pubkey.clone(),
            peer_id: p2p.peer_id_string(),
        };
        drop(identity);

        p2p.redeem_invite(peer_id, redemption).await?;

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
