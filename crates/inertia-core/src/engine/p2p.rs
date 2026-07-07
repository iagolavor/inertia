use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;
use tracing::{info, warn};

use crate::error::{CoreError, CoreResult};
use crate::p2p::{filter_friend_multiaddrs, relay_circuit_dial_addr, P2pNode};

use super::{activity, p2p_status, relay_dial, Engine, P2pStatus};

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

        let relay_multiaddrs = self.effective_relays().await;
        let node = P2pNode::start(
            self.store.clone(),
            Arc::clone(&self.identity),
            listen_addr,
            relay_multiaddrs,
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
        self.emit_p2p_status_changed().await;
        result
    }

    async fn redial_known_peers_inner(&self) -> CoreResult<()> {
        let relays = self.effective_relays().await;
        let _relay_ready = relay_dial::bootstrap_relays_for_friend_dial(&self.p2p, &relays).await;

        let contacts = self.list_contacts().await?;
        for contact in contacts {
            let addrs = relay_dial::contact_dial_addrs(&contact, &relays);
            if addrs.is_empty() {
                continue;
            }
            for addr in addrs.into_iter().take(relay_dial::MAX_DIALS_PER_CONTACT) {
                if let Err(e) = self.dial_peer(&addr).await {
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

    /// Redial configured relays when libp2p is up but a relay session dropped.
    pub async fn ensure_relay_connected(&self) -> CoreResult<()> {
        let relays = self.effective_relays().await;
        if relays.is_empty() {
            return Ok(());
        }

        let guard = self.p2p.lock().await;
        let Some(p2p) = guard.as_ref() else {
            return Ok(());
        };
        let connected = p2p.connected_peer_ids().await;
        drop(guard);

        for relay in relays {
            let trimmed = relay.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Some(relay_peer_id) = peer_id_from_multiaddr_str(trimmed) else {
                continue;
            };
            if connected.iter().any(|peer| peer == &relay_peer_id) {
                continue;
            }
            self.dial_peer(trimmed).await?;
            info!(%relay_peer_id, "redialing relay (session not connected)");
        }
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
        relays: Vec<String>,
        relay_tcp_reachable: Option<bool>,
    ) -> P2pStatus {
        let relay_peer_ids: Vec<String> = relays
            .iter()
            .filter_map(|relay| peer_id_from_multiaddr_str(relay))
            .collect();
        let pending_outbox_count = activity::count_pending_outbox(&self.store).await;
        let activity_snap = self.activity.lock().await.snapshot();

        let guard = self.p2p.lock().await;
        let status_core = if let Some(p2p) = guard.as_ref() {
            let connected_peer_ids = p2p.connected_peer_ids().await;
            let relays_connected_count = relay_peer_ids
                .iter()
                .filter(|id| connected_peer_ids.iter().any(|p| p == *id))
                .count();
            let relay_connected = relays_connected_count > 0;
            let friends_online_count = connected_peer_ids
                .iter()
                .filter(|id| !relay_peer_ids.iter().any(|relay_id| relay_id == *id))
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
                relays_connected_count,
                friends_online_count,
            )
        } else {
            (false, None, Vec::new(), Vec::new(), false, 0, 0)
        };
        drop(guard);

        let (
            running,
            peer_id,
            listen_addresses,
            connected_peer_ids,
            relay_connected,
            relays_connected_count,
            friends_online_count,
        ) = status_core;

        let layers = p2p_status::build_layers(
            running,
            !relays.is_empty(),
            relay_tcp_reachable,
            relay_connected,
            friends_online_count,
            activity_snap.dial_in_progress,
            pending_outbox_count,
        );
        let labels = p2p_status::build_labels(&layers);
        let tone = p2p_status::visual_tone_str(p2p_status::visual_tone(&layers)).to_string();

        {
            let mut hints = self.relay_status_hints.lock().await;
            activity::refresh_relay_hints_from_store(
                &self.store,
                &mut hints,
                relay_tcp_reachable,
            )
            .await;
        }

        P2pStatus {
            running,
            peer_id,
            listen_addresses,
            connected_peer_ids,
            relay_configured: !relays.is_empty(),
            relay_multiaddrs: relays,
            relays_connected_count,
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

    /// Connected libp2p peers excluding configured relays.
    pub(crate) async fn connected_friend_peer_ids(&self) -> std::collections::HashSet<String> {
        use std::collections::HashSet;

        let relay_peer_ids: HashSet<String> = self
            .effective_relays()
            .await
            .iter()
            .filter_map(|relay| peer_id_from_multiaddr_str(relay))
            .collect();

        let guard = self.p2p.lock().await;
        let Some(p2p) = guard.as_ref() else {
            return HashSet::new();
        };
        let connected = p2p.connected_peer_ids().await;
        drop(guard);

        connected
            .into_iter()
            .filter(|id| !relay_peer_ids.contains(id))
            .collect()
    }

    pub async fn p2p_status(&self) -> P2pStatus {
        let relays = self.effective_relays().await;
        let relay_tcp_reachable = if let Some(addr) = relays.first() {
            Some(self.relay_tcp_reachable_cached(addr).await)
        } else {
            None
        };
        self.p2p_status_snapshot(relays, relay_tcp_reachable).await
    }

    /// Cached or session-derived relay reachability, or None when a TCP probe is needed.
    pub async fn relay_tcp_reachable_precheck(&self, relay_addr: &str) -> Option<bool> {
        use std::time::{Duration, Instant};

        const TTL: Duration = Duration::from_secs(60);

        if let Some(relay_peer_id) = peer_id_from_multiaddr_str(relay_addr) {
            let guard = self.p2p.lock().await;
            if let Some(p2p) = guard.as_ref() {
                let connected = p2p.connected_peer_ids().await;
                if connected.iter().any(|id| id == &relay_peer_id) {
                    return Some(true);
                }
            }
        }

        let now = Instant::now();
        let cache = self.relay_probe_cache.lock().await;
        if let Some((reachable, at)) = *cache {
            if now.duration_since(at) < TTL {
                return Some(reachable);
            }
        }
        None
    }

    pub async fn store_relay_tcp_probe(&self, reachable: bool) {
        use std::time::Instant;
        *self.relay_probe_cache.lock().await = Some((reachable, Instant::now()));
    }

    async fn relay_tcp_reachable_cached(&self, relay_addr: &str) -> bool {
        if let Some(reachable) = self.relay_tcp_reachable_precheck(relay_addr).await {
            return reachable;
        }

        let reachable = super::probe_relay_tcp(relay_addr).await;
        self.store_relay_tcp_probe(reachable).await;
        reachable
    }

    /// Circuit dial targets for relays with a live session when the swarm omits circuit listeners.
    async fn connected_relay_circuit_addresses(&self, peer_id: &str) -> Vec<String> {
        let relays = self.effective_relays().await;
        if relays.is_empty() {
            return Vec::new();
        }

        let connected = {
            let guard = self.p2p.lock().await;
            match guard.as_ref() {
                Some(p2p) => p2p.connected_peer_ids().await,
                None => return Vec::new(),
            }
        };

        let mut addrs = Vec::new();
        for relay in &relays {
            let Some(relay_peer) = peer_id_from_multiaddr_str(relay) else {
                continue;
            };
            if !connected.iter().any(|peer| peer == &relay_peer) {
                continue;
            }
            if let Some(circuit) = relay_circuit_dial_addr(relay, peer_id) {
                addrs.push(circuit);
            }
        }
        relay_dial::sort_contact_dial_addrs(&addrs)
    }

    /// True when any configured relay has a live libp2p session.
    pub async fn any_relay_connected(&self) -> bool {
        use std::collections::HashSet;

        let relays = self.effective_relays().await;
        if relays.is_empty() {
            return false;
        }
        let relay_ids: HashSet<String> = relays
            .iter()
            .filter_map(|relay| peer_id_from_multiaddr_str(relay))
            .collect();
        if relay_ids.is_empty() {
            return false;
        }

        let guard = self.p2p.lock().await;
        let Some(p2p) = guard.as_ref() else {
            return false;
        };
        let connected = p2p.connected_peer_ids().await;
        drop(guard);

        relay_ids
            .iter()
            .any(|relay_id| connected.iter().any(|peer| peer == relay_id))
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

        if let Some(pid) = peer_id {
            return self.connected_relay_circuit_addresses(pid).await;
        }

        Vec::new()
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
    let addrs: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|addr| {
            if addr.contains("/p2p/") {
                addr.to_string()
            } else {
                format!("{addr}/p2p/{peer_id}")
            }
        })
        .collect();
    filter_friend_multiaddrs(&addrs)
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
