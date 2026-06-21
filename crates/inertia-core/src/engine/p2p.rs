use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;
use tracing::{info, warn};

use crate::error::{CoreError, CoreResult};
use crate::p2p::P2pNode;

use super::{activity, p2p_status, Engine, P2pStatus};

impl Engine {
    /// Idempotent — returns the current peer id if P2P is already running.
    pub async fn ensure_p2p_started(&self) -> CoreResult<String> {
        self.start_p2p(0).await
    }

    pub async fn start_p2p(&self, listen_port: u16) -> CoreResult<String> {
        let listen_port = if listen_port == 0 {
            self.resolve_listen_port().await
        } else {
            listen_port
        };

        let mut guard = self.p2p.lock().await;
        if let Some(node) = guard.as_ref() {
            return Ok(node.peer_id_string());
        }

        let listen_addr = format!("/ip4/0.0.0.0/tcp/{listen_port}")
            .parse::<Multiaddr>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;

        let relay_multiaddr = self.effective_relay().await;
        let node = P2pNode::start(
            self.store.clone(),
            Arc::clone(&self.identity),
            listen_addr,
            relay_multiaddr,
            self.event_tx.clone(),
        )
        .await?;

        let peer_id = node.peer_id_string();
        *guard = Some(node);
        drop(guard);

        if let Err(e) = self.redial_known_peers().await {
            warn!(error = %e, "redial known peers failed");
        }

        info!(%peer_id, port = listen_port, "p2p node started");
        Ok(peer_id)
    }

    /// Dial configured relay (if any) and stored contact addresses after P2P starts.
    pub async fn redial_known_peers(&self) -> CoreResult<()> {
        self.activity.lock().await.set_dial_in_progress(true);
        let result = self.redial_known_peers_inner().await;
        self.activity.lock().await.set_dial_in_progress(false);
        result
    }

    async fn redial_known_peers_inner(&self) -> CoreResult<()> {
        if let Some(relay) = self.effective_relay().await {
            match self.dial_peer(&relay).await {
                Ok(()) => {
                    self.activity.lock().await.push("dial", "Dialed relay");
                    info!("dialed configured relay");
                }
                Err(e) => {
                    self.activity
                        .lock()
                        .await
                        .push("dial_failed", format!("Relay dial failed: {e}"));
                    warn!(error = %e, "failed to dial relay");
                }
            }
        }

        let contacts = self.list_contacts().await?;
        for contact in contacts {
            if contact.multiaddrs.is_empty() {
                continue;
            }
            for addr in &contact.multiaddrs {
                if let Err(e) = self.dial_peer(addr).await {
                    warn!(
                        friend = %contact.display_name,
                        address = %addr,
                        error = %e,
                        "failed to redial contact"
                    );
                }
            }
        }
        Ok(())
    }

    /// Redial the configured relay when libp2p is up but the relay session dropped.
    pub async fn ensure_relay_connected(&self) -> CoreResult<()> {
        let relay = match self.effective_relay().await {
            Some(addr) if !addr.trim().is_empty() => addr,
            _ => return Ok(()),
        };
        let relay_peer_id = peer_id_from_multiaddr_str(&relay);
        let guard = self.p2p.lock().await;
        let Some(p2p) = guard.as_ref() else {
            return Ok(());
        };
        let connected = p2p.connected_peer_ids().await;
        drop(guard);

        if relay_peer_id
            .as_ref()
            .is_some_and(|id| connected.iter().any(|peer| peer == id))
        {
            return Ok(());
        }

        self.dial_peer(&relay).await?;
        info!("redialing relay (session not connected)");
        Ok(())
    }

    pub async fn peer_id(&self) -> Option<String> {
        self.p2p.lock().await.as_ref().map(|n| n.peer_id_string())
    }

    pub async fn p2p_listen_addresses(&self) -> CoreResult<Vec<String>> {
        let guard = self.p2p.lock().await;
        let p2p = guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        Ok(p2p
            .listen_addresses()
            .await
            .into_iter()
            .map(|a| a.to_string())
            .collect())
    }

    pub async fn p2p_status_snapshot(
        &self,
        relay: Option<String>,
        relay_tcp_reachable: Option<bool>,
    ) -> P2pStatus {
        let relay_peer_id = relay.as_deref().and_then(peer_id_from_multiaddr_str);
        let pending_outbox_count = activity::count_pending_outbox(&self.store).await;
        let activity_snap = self.activity.lock().await.snapshot();

        let guard = self.p2p.lock().await;
        let status_core = if let Some(p2p) = guard.as_ref() {
            let connected_peer_ids = p2p.connected_peer_ids().await;
            let relay_connected = relay_peer_id
                .as_ref()
                .is_some_and(|id| connected_peer_ids.iter().any(|p| p == id));
            let friends_online_count = connected_peer_ids
                .iter()
                .filter(|id| relay_peer_id.as_ref() != Some(id))
                .count();
            (
                true,
                Some(p2p.peer_id_string()),
                p2p.listen_addresses()
                    .await
                    .into_iter()
                    .map(|a| a.to_string())
                    .collect(),
                connected_peer_ids,
                relay_connected,
                friends_online_count,
            )
        } else {
            (false, None, Vec::new(), Vec::new(), false, 0)
        };
        drop(guard);

        let (
            running,
            peer_id,
            listen_addresses,
            connected_peer_ids,
            relay_connected,
            friends_online_count,
        ) = status_core;

        let layers = p2p_status::build_layers(
            running,
            relay.is_some(),
            relay_tcp_reachable,
            relay_connected,
            friends_online_count,
            activity_snap.dial_in_progress,
            pending_outbox_count,
        );
        let labels = p2p_status::build_labels(&layers);
        let tone = p2p_status::visual_tone_str(p2p_status::visual_tone(&layers)).to_string();

        P2pStatus {
            running,
            peer_id,
            listen_addresses,
            connected_peer_ids,
            relay_configured: relay.is_some(),
            relay_peer_id,
            relay_connected,
            relay_tcp_reachable,
            pending_outbox_count,
            dial_in_progress: activity_snap.dial_in_progress,
            last_activity_at: activity_snap.last_activity_at,
            recent_activity: activity_snap.events,
            layers,
            labels,
            tone,
        }
    }

    pub async fn p2p_activity(&self) -> super::P2pActivitySnapshot {
        self.activity.lock().await.snapshot()
    }

    pub async fn p2p_status(&self) -> P2pStatus {
        let relay = self.relay_multiaddr().await;
        let relay_tcp_reachable = if let Some(ref addr) = relay {
            Some(relay_tcp_reachable(addr).await)
        } else {
            None
        };
        self.p2p_status_snapshot(relay, relay_tcp_reachable).await
    }

    /// Addresses embedded in invites — uses settings or `INERTIA_P2P_ANNOUNCE` when set.
    pub async fn p2p_invite_addresses(&self, peer_id: Option<&str>) -> Vec<String> {
        if let Some(pid) = peer_id {
            let announce = self
                .store
                .with(|s| s.get_settings())
                .await
                .ok()
                .and_then(|s| s.p2p_announce);
            let announced = announced_p2p_multiaddrs(pid, announce.as_deref());
            if !announced.is_empty() {
                return announced;
            }
        }

        if let Ok(addrs) = self.p2p_routable_addresses().await {
            if !addrs.is_empty() {
                return addrs;
            }
        }

        self.p2p_listen_addresses().await.unwrap_or_default()
    }

    pub async fn p2p_routable_addresses(&self) -> CoreResult<Vec<String>> {
        let guard = self.p2p.lock().await;
        let p2p = guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        Ok(p2p.routable_listen_addresses().await)
    }

    pub async fn connection_share_multiaddr(&self) -> CoreResult<Option<String>> {
        let addrs = self.p2p_invite_addresses(self.peer_id().await.as_deref()).await;
        Ok(addrs.into_iter().next())
    }

    pub async fn dial_peer(&self, multiaddr: &str) -> CoreResult<()> {
        let guard = self.p2p.lock().await;
        let p2p = guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let addr = multiaddr
            .parse::<Multiaddr>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;
        p2p.dial(addr).await
    }

    /// Poll until a peer id appears in the connected set or timeout.
    pub(super) async fn wait_for_peer_connected(
        &self,
        peer_id: &str,
        timeout: Duration,
        role: &str,
    ) -> CoreResult<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let connected = {
                let guard = self.p2p.lock().await;
                match guard.as_ref() {
                    Some(p2p) => p2p.connected_peer_ids().await,
                    None => return Err(CoreError::P2p("p2p not started".into())),
                }
            };
            if connected.iter().any(|peer| peer == peer_id) {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                let message = match role {
                    "relay" => {
                        "could not connect to the relay network — wait until the header shows Relay OK, then try again"
                    }
                    _ => {
                        "could not reach the inviter — they must stay online with Relay OK while you accept"
                    }
                };
                return Err(CoreError::Invite(message.into()));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    pub async fn send_friend_request(&self, contact_id: &str, multiaddr: &str) -> CoreResult<()> {
        let guard = self.p2p.lock().await;
        let p2p = guard
            .as_ref()
            .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
        let addr = multiaddr
            .parse::<Multiaddr>()
            .map_err(|e| CoreError::P2p(e.to_string()))?;
        p2p.send_friend_request(contact_id, addr).await
    }
}

/// Comma-separated multiaddrs from settings or `INERTIA_P2P_ANNOUNCE`, with `/p2p/<peer_id>` appended when missing.
pub(super) fn announced_p2p_multiaddrs(peer_id: &str, announce: Option<&str>) -> Vec<String> {
    let Some(raw) = announce
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| std::env::var("INERTIA_P2P_ANNOUNCE").ok())
    else {
        return Vec::new();
    };
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|addr| {
            if addr.contains("/p2p/") {
                addr.to_string()
            } else {
                format!("{addr}/p2p/{peer_id}")
            }
        })
        .collect()
}

pub(super) fn validate_relay_multiaddr(raw: &str) -> CoreResult<()> {
    let trimmed = raw.trim();
    let addr: Multiaddr = trimmed
        .parse()
        .map_err(|e| CoreError::P2p(format!("invalid relay multiaddr: {e}")))?;
    let mut has_ip = false;
    let mut has_tcp = false;
    for protocol in addr.iter() {
        match protocol {
            Protocol::Ip4(_) | Protocol::Ip6(_) => has_ip = true,
            Protocol::Tcp(_) => has_tcp = true,
            _ => {}
        }
    }
    if !has_ip || !has_tcp {
        return Err(CoreError::P2p(
            "relay multiaddr must be a full address like /ip4/HOST/tcp/9000/p2p/PEER_ID — not just the peer id".into(),
        ));
    }
    Ok(())
}

pub(super) fn peer_id_from_multiaddr_str(multiaddr: &str) -> Option<String> {
    multiaddr
        .parse::<Multiaddr>()
        .ok()?
        .iter()
        .find_map(|protocol| match protocol {
            Protocol::P2p(peer_id) => Some(peer_id.to_string()),
            _ => None,
        })
}

fn relay_tcp_endpoint(multiaddr: &str) -> Option<(IpAddr, u16)> {
    let addr = multiaddr.parse::<Multiaddr>().ok()?;
    let mut ip = None;
    let mut port = None;
    for protocol in addr.iter() {
        match protocol {
            Protocol::Ip4(v) => ip = Some(IpAddr::V4(v)),
            Protocol::Ip6(v) => ip = Some(IpAddr::V6(v)),
            Protocol::Tcp(p) => port = Some(p),
            _ => {}
        }
    }
    Some((ip?, port?))
}

pub(super) async fn relay_tcp_reachable(multiaddr: &str) -> bool {
    let Some((ip, port)) = relay_tcp_endpoint(multiaddr) else {
        return false;
    };
    match tokio::time::timeout(
        Duration::from_millis(400),
        tokio::net::TcpStream::connect((ip, port)),
    )
    .await
    {
        Ok(Ok(_stream)) => true,
        _ => false,
    }
}
