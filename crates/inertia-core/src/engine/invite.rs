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
use super::{Engine, InvitePreview, InviteResponse, P2pStatus};

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

/// Progress through relay bootstrap before invite create or accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InviteRelayPhase {
    NoRelaysConfigured,
    P2pNotRunning,
    RelayTcpUnreachable,
    RelayNotConnected,
    AwaitingReservation,
    AwaitingCircuitAddress,
    Ready,
}

impl InviteRelayPhase {
    fn readiness_message(self) -> &'static str {
        match self {
            Self::NoRelaysConfigured => "Add a relay in Settings before inviting.",
            Self::P2pNotRunning => "P2P is starting - try again in a moment.",
            Self::RelayTcpUnreachable => {
                "Relay VPS port unreachable - check the address in Settings."
            }
            Self::RelayNotConnected => {
                "Connecting to relay - wait for Relay OK in the header."
            }
            Self::AwaitingReservation => {
                "Waiting for relay inbound slot - tap Generate and keep the app open for a few seconds."
            }
            Self::AwaitingCircuitAddress => {
                "Relay connected but not reachable yet - tap Generate and we will prepare your circuit slot."
            }
            Self::Ready => "Ready to invite - stay in the app while your friend accepts.",
        }
    }

    fn ensure_error_message(self) -> &'static str {
        match self {
            Self::NoRelaysConfigured => {
                "relay multiaddr is not configured - add one in Settings"
            }
            Self::P2pNotRunning => "P2P is not running - restart the API and try again",
            Self::RelayTcpUnreachable | Self::RelayNotConnected => {
                "could not connect to the relay network - check Settings and try again"
            }
            Self::AwaitingReservation | Self::AwaitingCircuitAddress => {
                "relay circuit slot not ready - stay on this screen with Relay OK, then try again"
            }
            Self::Ready => unreachable!("ready phase has no ensure error"),
        }
    }

    fn to_readiness(self) -> InviteReadiness {
        InviteReadiness {
            ready: self == Self::Ready,
            relay_configured: self != Self::NoRelaysConfigured,
            relay_connected: matches!(
                self,
                Self::AwaitingReservation | Self::AwaitingCircuitAddress | Self::Ready
            ),
            reachable: self == Self::Ready,
            message: self.readiness_message().into(),
        }
    }
}

/// Connected + reserved state for the invite priority relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PriorityRelaySnapshot {
    connected: bool,
    reserved: bool,
}

impl Engine {
    /// Whether this device can issue a friend invite right now.
    pub async fn invite_readiness(&self) -> InviteReadiness {
        self.invite_relay_phase().await.to_readiness()
    }

    async fn invite_relay_phase(&self) -> InviteRelayPhase {
        let relays = self.effective_relays().await;
        if relays.is_empty() {
            return InviteRelayPhase::NoRelaysConfigured;
        }

        let status = self.p2p_status().await;
        if !status.running {
            return InviteRelayPhase::P2pNotRunning;
        }
        if status.relay_tcp_reachable == Some(false) && !status.relay_connected {
            return InviteRelayPhase::RelayTcpUnreachable;
        }
        if !status.relay_connected {
            return InviteRelayPhase::RelayNotConnected;
        }

        let priority_relay = self
            .pick_invite_relay(&relays, &status)
            .unwrap_or_else(|| relays[0].clone());
        let snapshot = match self.priority_relay_snapshot(&priority_relay).await {
            Ok(snapshot) => snapshot,
            Err(_) => return InviteRelayPhase::P2pNotRunning,
        };
        if !snapshot.reserved {
            return InviteRelayPhase::AwaitingReservation;
        }

        let reachable = self
            .connection_share_multiaddr()
            .await
            .ok()
            .flatten()
            .is_some();
        if !reachable {
            return InviteRelayPhase::AwaitingCircuitAddress;
        }

        InviteRelayPhase::Ready
    }

    fn pick_invite_relay(&self, relays: &[String], status: &P2pStatus) -> Option<String> {
        select_invite_relay(relays, &status.connected_peer_ids)
            .or_else(|| relays.first().cloned())
    }

    async fn priority_relay_snapshot(
        &self,
        priority_relay: &str,
    ) -> CoreResult<PriorityRelaySnapshot> {
        let priority_peer_id = peer_id_from_multiaddr_str(priority_relay);
        let guard = self.p2p.lock().await;
        let node = guard
            .as_ref()
            .ok_or_else(|| CoreError::Invite(InviteRelayPhase::P2pNotRunning.ensure_error_message().into()))?;
        let connected_peers = node.connected_peer_ids().await;
        let connected = priority_peer_id
            .as_ref()
            .is_some_and(|id| connected_peers.iter().any(|peer| peer == id));
        let reserved = if let Some(id) = priority_peer_id.as_ref() {
            node.has_relay_reservation(std::slice::from_ref(id)).await
        } else {
            false
        };
        Ok(PriorityRelaySnapshot {
            connected,
            reserved,
        })
    }

    async fn resolve_invite_relay(&self) -> CoreResult<(Vec<String>, String)> {
        let relays = self.effective_relays().await;
        if relays.is_empty() {
            return Err(CoreError::Invite(
                InviteRelayPhase::NoRelaysConfigured.ensure_error_message().into(),
            ));
        }
        let status = self.p2p_status().await;
        let relay_multiaddr = self
            .pick_invite_relay(&relays, &status)
            .ok_or_else(|| {
                CoreError::Invite(InviteRelayPhase::NoRelaysConfigured.ensure_error_message().into())
            })?;
        validate_relay_multiaddr(&relay_multiaddr)?;
        Ok((relays, relay_multiaddr))
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
                InviteRelayPhase::NoRelaysConfigured.ensure_error_message().into(),
            ));
        }

        let mut snapshot = self.priority_relay_snapshot(priority_relay).await?;

        if !snapshot.connected || (require_reservation && !snapshot.reserved) {
            relay_dial::bootstrap_invite_relay(
                &self.p2p,
                priority_relay,
                relays,
                require_reservation,
            )
            .await;
            snapshot = self.priority_relay_snapshot(priority_relay).await?;
        } else {
            self.apply_relay_list_to_p2p().await?;
        }

        if !snapshot.connected && !self.any_relay_connected().await {
            return Err(CoreError::Invite(
                InviteRelayPhase::RelayNotConnected.ensure_error_message().into(),
            ));
        }

        if require_reservation && !snapshot.reserved {
            return Err(CoreError::Invite(
                InviteRelayPhase::AwaitingReservation.ensure_error_message().into(),
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
        let (relays, relay_multiaddr) = self.resolve_invite_relay().await?;
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
    ) -> bool {
        relay_dial::bootstrap_invite_relay(&p2p, priority_relay, relays, wait_for_reservation)
            .await
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

        let (relays, relay_multiaddr) = self.resolve_invite_relay().await?;
        self.ensure_invite_relay_ready(&relays, &relay_multiaddr, false)
            .await?;

        // Invite create must not mint a link unless we're inbound-dialable.
        // The API tries to bootstrap a reservation before calling this, but we
        // still fail fast if it did not actually stick.
        let snapshot = self.priority_relay_snapshot(&relay_multiaddr).await?;
        if !snapshot.reserved {
            return Err(CoreError::Invite(
                InviteRelayPhase::AwaitingReservation.ensure_error_message().into(),
            ));
        }

        let peer_id = self.peer_id().await;
        let multiaddrs = self.invite_circuit_addresses().await;
        if multiaddrs.is_empty() {
            return Err(CoreError::Invite(
                "no relay circuit address - check relay connection and try again".into(),
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
                "inviter is not reachable - they must be online with P2P running".into(),
            )
        })?;

        let relays = self.effective_relays().await;
        self.ensure_invite_relay_ready(&relays, &invite.relay_multiaddr, false)
            .await?;

        let dial_addrs = Self::inviter_dial_addrs_for_invite(&invite, &relays);
        if dial_addrs.is_empty() {
            return Err(CoreError::Invite(
                "inviter has no relay circuit addresses - ask for a fresh invite".into(),
            ));
        }

        let dial_limit = dial_addrs.len().min(INVITE_ACCEPT_DIAL_ATTEMPTS);
        info!(
            inviter = %peer_id_str,
            attempts = dial_limit,
            "accept invite - dialing inviter via relay circuits"
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
            "could not reach the inviter - they must stay online with Relay OK while you accept"
                .into(),
        )
    } else {
        err
    }
}

/// Origins that only work on the dev machine - use `inertia://` links for cross-device sharing.
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
