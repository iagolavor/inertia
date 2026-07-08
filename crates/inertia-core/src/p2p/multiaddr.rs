use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};

use crate::error::{CoreError, CoreResult};

pub fn peer_id_from_multiaddr(addr: &Multiaddr) -> CoreResult<PeerId> {
    addr.iter()
        .find_map(|protocol| match protocol {
            Protocol::P2p(peer_id) => Some(peer_id),
            _ => None,
        })
        .ok_or_else(|| CoreError::P2p("multiaddr missing /p2p peer id".into()))
}

/// Relay peer id from `/p2p/<relay>/p2p-circuit/...`.
pub fn relay_peer_id_from_circuit_multiaddr(addr: &Multiaddr) -> Option<PeerId> {
    let mut last_p2p = None;
    for protocol in addr.iter() {
        match protocol {
            Protocol::P2p(peer_id) => last_p2p = Some(peer_id),
            Protocol::P2pCircuit => return last_p2p,
            _ => {}
        }
    }
    None
}

/// True when the relay assigned a full inbound circuit address (includes our peer id).
pub fn is_confirmed_relay_circuit_listen_addr(addr: &Multiaddr, local_peer_id: &PeerId) -> bool {
    if !addr.to_string().contains("/p2p-circuit/") {
        return false;
    }
    addr.iter()
        .any(|protocol| matches!(protocol, Protocol::P2p(peer_id) if peer_id == *local_peer_id))
}

pub fn relay_circuit_listen_addr(relay: &Multiaddr) -> Multiaddr {
    if relay.iter().any(|p| matches!(p, Protocol::P2pCircuit)) {
        relay.clone()
    } else {
        relay.clone().with(Protocol::P2pCircuit)
    }
}

/// Outbound dial to a peer via a configured relay (for roaming when only peer id is known).
pub fn relay_circuit_dial_addr(relay: &str, peer_id: &str) -> Option<String> {
    let relay = relay.trim();
    let peer_id = peer_id.trim();
    if relay.is_empty() || peer_id.is_empty() {
        return None;
    }
    let base = relay.trim_end_matches('/');
    Some(format!("{base}/p2p-circuit/p2p/{peer_id}"))
}

pub fn is_routable_multiaddr(addr: &Multiaddr) -> bool {
    let raw = addr.to_string();
    !raw.contains("/ip4/0.0.0.0/") && !raw.contains("/ip6/::/")
}

/// True for RFC1918, loopback, and link-local `/ip4/` multiaddrs.
pub fn is_lan_multiaddr(addr: &Multiaddr) -> bool {
    addr.iter().any(|protocol| match protocol {
        Protocol::Ip4(octets) => {
            let v4 = std::net::Ipv4Addr::from(octets);
            v4.is_loopback() || v4.is_private() || v4.is_link_local()
        }
        _ => false,
    })
}

pub fn is_lan_multiaddr_str(addr: &str) -> bool {
    addr.parse::<Multiaddr>()
        .ok()
        .is_some_and(|multiaddr| is_lan_multiaddr(&multiaddr))
}

/// Friend paths we store and redial: relay circuits only (no LAN, no direct TCP).
pub fn is_relay_circuit_multiaddr_str(addr: &str) -> bool {
    addr.contains("/p2p-circuit/") && !is_lan_multiaddr_str(addr)
}

/// Keep only relay-circuit multiaddrs suitable for friend connectivity.
pub fn filter_friend_multiaddrs(addrs: &[String]) -> Vec<String> {
    let mut out: Vec<String> = addrs
        .iter()
        .filter(|addr| is_relay_circuit_multiaddr_str(addr))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_friend_multiaddrs_keeps_circuits_drops_lan_and_direct_tcp() {
        let addrs = vec![
            "/ip4/192.168.1.5/tcp/4784".into(),
            "/ip4/8.8.8.8/tcp/4784".into(),
            "/ip4/1.2.3.4/tcp/9000/p2p/12D3KooWExampleRelay/p2p-circuit/p2p/12D3KooWFriend"
                .into(),
        ];
        let filtered = filter_friend_multiaddrs(&addrs);
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].contains("/p2p-circuit/"));
    }
}

pub fn ensure_peer_id_suffix(addr: &Multiaddr, peer_id: &str) -> String {
    let raw = addr.to_string();
    if raw.contains("/p2p/") {
        raw
    } else {
        format!("{raw}/p2p/{peer_id}")
    }
}
