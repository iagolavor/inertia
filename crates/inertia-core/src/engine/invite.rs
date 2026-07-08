use tracing::info;

use libp2p::PeerId;
use std::time::Duration;

use crate::error::{CoreError, CoreResult};
use crate::invite::FriendInvite;
use crate::p2p::{filter_friend_multiaddrs, relay_circuit_dial_addr};
use crate::storage::Contact;

use super::p2p::peer_id_from_multiaddr_str;
use super::p2p::validate_relay_multiaddr;
use super::relay_dial::{self, sort_contact_dial_addrs};
use super::relay_list::select_invite_relay;
use super::{Engine, InvitePreview, InviteResponse};

/// Max wait for inviter libp2p session during invite accept (relay circuit redials).
pub const INVITE_INVITER_WAIT: Duration = Duration::from_secs(120);

/// Max circuit dials per redial round when accepting an invite.
pub const INVITE_ACCEPT_DIAL_ATTEMPTS: usize = 8;

#[derive(Debug, Clone, serde::Serialize)]
pub struct InviteReadiness {
    pub ready: bool,
    pub relay_configured: bool,
    pub relay_connected: bool,
    pub reachable: bool,
    pub message: String,
}

impl Engine {
    /// Whether this device can issue a friend invite right now.
    pub async fn invite_readiness(&self) -> InviteReadiness {
        let relays = self.effective_relays().await;
        if relays.is_empty() {
            return InviteReadiness {
                ready: false,
                relay_configured: false,
                relay_connected: false,
                reachable: false,
                message: "Add a relay in Settings before inviting.".into(),
            };
        }

        let status = self.p2p_status().await;
        if !status.running {
            return InviteReadiness {
                ready: false,
                relay_configured: true,
                relay_connected: false,
                reachable: false,
                message: "P2P is starting — try again in a moment.".into(),
            };
        }
        if status.relay_tcp_reachable == Some(false) && !status.relay_connected {
            return InviteReadiness {
                ready: false,
                relay_configured: true,
                relay_connected: false,
                reachable: false,
                message: "Relay VPS port unreachable — check the address in Settings.".into(),
            };
        }
        if !status.relay_connected {
            return InviteReadiness {
                ready: false,
                relay_configured: true,
                relay_connected: false,
                reachable: false,
                message: "Connecting to relay — wait for Relay OK in the header.".into(),
            };
        }

        let relays = self.effective_relays().await;
        let priority_relay = select_invite_relay(&relays, &status.connected_peer_ids)
            .or_else(|| relays.first().cloned());
        let reserved = if let Some(relay) = priority_relay.as_deref() {
            let priority_peer = peer_id_from_multiaddr_str(relay);
            if let Some(id) = priority_peer {
                let guard = self.p2p.lock().await;
                match guard.as_ref() {
                    Some(node) => node.has_relay_reservation(&[id]).await,
                    None => false,
                }
            } else {
                false
            }
        } else {
            false
        };

        if !reserved {
            return InviteReadiness {
                ready: false,
                relay_configured: true,
                relay_connected: true,
                reachable: false,
                message: "Waiting for relay inbound slot — tap Generate and keep the app open for a few seconds."
                    .into(),
            };
        }

        let share = self
            .connection_share_multiaddr()
            .await
            .ok()
            .flatten();
        let reachable = share.is_some();
        if !reachable {
            return InviteReadiness {
                ready: false,
                relay_configured: true,
                relay_connected: true,
                reachable: false,
                message: "Relay connected but not reachable yet — tap Generate and we will prepare your circuit slot."
                    .into(),
            };
        }

        InviteReadiness {
            ready: true,
            relay_configured: true,
            relay_connected: true,
            reachable: true,
            message: "Ready to invite — stay in the app while your friend accepts.".into(),
        }
    }

    fn inviter_dial_addrs_for_invite(invite: &FriendInvite, relays: &[String]) -> Vec<String> {
        let Some(pid) = invite.peer_id.as_deref() else {
            return filter_friend_multiaddrs(&invite.multiaddrs);
        };
        let mut addrs = Vec::new();
        if let Some(circuit) = relay_circuit_dial_addr(&invite.relay_multiaddr, pid) {
            addrs.push(circuit);
        }
        for relay in relays {
            if let Some(circuit) = relay_circuit_dial_addr(relay, pid) {
                addrs.push(circuit);
            }
        }
        addrs.extend(filter_friend_multiaddrs(&invite.multiaddrs));
        sort_contact_dial_addrs(&addrs)
    }

    async fn ensure_invite_relay_ready(
        &self,
        relays: &[String],
        priority_relay: &str,
        require_reservation: bool,
    ) -> CoreResult<()> {
        if relays.is_empty() {
            return Err(CoreError::Invite(
                "relay multiaddr is not configured — add one in Settings".into(),
            ));
        }

        let priority_peer_id = peer_id_from_multiaddr_str(priority_relay);
        let (priority_connected, priority_reserved) = {
            let guard = self.p2p.lock().await;
            let Some(node) = guard.as_ref() else {
                return Err(CoreError::Invite(
                    "P2P is not running — restart the API and try again".into(),
                ));
            };
            let connected = node.connected_peer_ids().await;
            let connected_ok = priority_peer_id
                .as_ref()
                .is_some_and(|id| connected.iter().any(|peer| peer == id));
            let reserved_ok = if let Some(id) = priority_peer_id.as_ref() {
                node.has_relay_reservation(std::slice::from_ref(id)).await
            } else {
                false
            };
            (connected_ok, reserved_ok)
        };

        let needs_bootstrap =
            !priority_connected || (require_reservation && !priority_reserved);
        if needs_bootstrap {
            relay_dial::bootstrap_invite_relay(
                &self.p2p,
                priority_relay,
                relays,
                require_reservation,
            )
            .await;
        } else {
            self.apply_relay_list_to_p2p().await?;
        }

        let (priority_connected, priority_reserved) = {
            let guard = self.p2p.lock().await;
            let Some(node) = guard.as_ref() else {
                return Err(CoreError::Invite(
                    "P2P is not running — restart the API and try again".into(),
                ));
            };
            let connected = node.connected_peer_ids().await;
            let connected_ok = priority_peer_id
                .as_ref()
                .is_some_and(|id| connected.iter().any(|peer| peer == id));
            let reserved_ok = if let Some(id) = priority_peer_id.as_ref() {
                node.has_relay_reservation(std::slice::from_ref(id)).await
            } else {
                false
            };
            (connected_ok, reserved_ok)
        };

        if !priority_connected && !self.any_relay_connected().await {
            return Err(CoreError::Invite(
                "could not connect to the relay network — check Settings and try again".into(),
            ));
        }

        if require_reservation && !priority_reserved {
            return Err(CoreError::Invite(
                "relay circuit slot not ready — stay on this screen with Relay OK, then try again"
                    .into(),
            ));
        }

        self.store_relay_tcp_probe(true).await;
        Ok(())
    }

    /// Short lock for invite create: start P2P and pick the invite relay.
    pub async fn plan_invite_create_bootstrap(
        &self,
    ) -> CoreResult<(
        std::sync::Arc<tokio::sync::Mutex<Option<crate::p2p::P2pNode>>>,
        Vec<String>,
        String,
    )> {
        self.ensure_p2p_started().await?;
        let relays = self.effective_relays().await;
        if relays.is_empty() {
            return Err(CoreError::Invite(
                "relay multiaddr is not configured".into(),
            ));
        }
        let status = self.p2p_status().await;
        let relay_multiaddr = select_invite_relay(&relays, &status.connected_peer_ids)
            .or_else(|| relays.first().cloned())
            .ok_or_else(|| CoreError::Invite("relay multiaddr is not configured".into()))?;
        Ok((
            std::sync::Arc::clone(&self.p2p),
            relays,
            relay_multiaddr,
        ))
    }

    /// Short lock for invite accept: start P2P and snapshot relay list.
    pub async fn plan_invite_accept_bootstrap(
        &self,
    ) -> CoreResult<(
        std::sync::Arc<tokio::sync::Mutex<Option<crate::p2p::P2pNode>>>,
        Vec<String>,
    )> {
        self.ensure_p2p_started().await?;
        let relays = self.effective_relays().await;
        Ok((std::sync::Arc::clone(&self.p2p), relays))
    }

    pub async fn bootstrap_invite_relay_only(
        p2p: std::sync::Arc<tokio::sync::Mutex<Option<crate::p2p::P2pNode>>>,
        priority_relay: &str,
        relays: &[String],
        wait_for_reservation: bool,
    ) {
        relay_dial::bootstrap_invite_relay(&p2p, priority_relay, relays, wait_for_reservation)
            .await;
    }

    async fn invite_circuit_addresses(&self) -> Vec<String> {
        self.p2p_invite_addresses(self.peer_id().await.as_deref())
            .await
    }

    pub async fn create_invite(&self, _web_origin: Option<&str>) -> CoreResult<InviteResponse> {
        let identity = self.identity.read().await;
        if identity.display_name.is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }
        drop(identity);

        let relays = self.effective_relays().await;
        let status = self.p2p_status().await;
        let relay_multiaddr = select_invite_relay(&relays, &status.connected_peer_ids)
            .or_else(|| relays.first().cloned())
            .ok_or_else(|| CoreError::Invite("relay multiaddr is not configured".into()))?;
        validate_relay_multiaddr(&relay_multiaddr)?;

        self.ensure_invite_relay_ready(&relays, &relay_multiaddr, false)
            .await?;

        let peer_id = self.peer_id().await;
        let multiaddrs = self.invite_circuit_addresses().await;
        if multiaddrs.is_empty() {
            return Err(CoreError::Invite(
                "no relay circuit address — check relay connection and try again".into(),
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

        let peer_id_str = invite.peer_id.as_ref().ok_or_else(|| {
            CoreError::Invite(
                "inviter is not reachable — they must be online with P2P running".into(),
            )
        })?;

        let relays = self.effective_relays().await;
        self.ensure_invite_relay_ready(&relays, &invite.relay_multiaddr, false)
            .await?;

        let dial_addrs = Self::inviter_dial_addrs_for_invite(&invite, &relays);
        if dial_addrs.is_empty() {
            return Err(CoreError::Invite(
                "inviter has no relay circuit addresses — ask for a fresh invite".into(),
            ));
        }

        let dial_limit = dial_addrs.len().min(INVITE_ACCEPT_DIAL_ATTEMPTS);
        info!(
            inviter = %peer_id_str,
            attempts = dial_limit,
            "accept invite — dialing inviter via relay circuits"
        );
        for addr in dial_addrs.iter().take(dial_limit) {
            let _ = self.dial_peer(addr).await;
        }

        self.wait_for_peer_connected_redial(
            peer_id_str,
            &dial_addrs,
            INVITE_INVITER_WAIT,
            "inviter",
            dial_limit,
        )
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
            "could not reach the inviter — they must stay online with Relay OK while you accept"
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
    fn allows_on_device_api_origin() {
        assert!(!is_local_dev_origin("http://127.0.0.1:4783"));
    }
}
