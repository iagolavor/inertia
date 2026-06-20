use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;
use tracing::{info, warn};

use crate::error::{CoreError, CoreResult};
use crate::p2p::P2pNode;

use super::{Engine, P2pStatus};

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
        if let Some(relay) = self.effective_relay().await {
            match self.dial_peer(&relay).await {
                Ok(()) => info!("dialed configured relay"),
                Err(e) => warn!(error = %e, "failed to dial relay"),
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

        let guard = self.p2p.lock().await;
        if let Some(p2p) = guard.as_ref() {
            let connected_peer_ids = p2p.connected_peer_ids().await;
            let relay_connected = relay_peer_id
                .as_ref()
                .is_some_and(|id| connected_peer_ids.iter().any(|p| p == id));
            P2pStatus {
                running: true,
                peer_id: Some(p2p.peer_id_string()),
                listen_addresses: p2p
                    .listen_addresses()
                    .await
                    .into_iter()
                    .map(|a| a.to_string())
                    .collect(),
                connected_peer_ids,
                relay_configured: relay.is_some(),
                relay_peer_id,
                relay_connected,
                relay_tcp_reachable,
            }
        } else {
            P2pStatus {
                running: false,
                peer_id: None,
                listen_addresses: Vec::new(),
                connected_peer_ids: Vec::new(),
                relay_configured: relay.is_some(),
                relay_peer_id,
                relay_connected: false,
                relay_tcp_reachable,
            }
        }
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
