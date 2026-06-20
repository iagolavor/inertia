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

pub fn relay_circuit_listen_addr(relay: &Multiaddr) -> Multiaddr {
    if relay.iter().any(|p| matches!(p, Protocol::P2pCircuit)) {
        relay.clone()
    } else {
        relay.clone().with(Protocol::P2pCircuit)
    }
}

pub fn is_routable_multiaddr(addr: &Multiaddr) -> bool {
    let raw = addr.to_string();
    !raw.contains("/ip4/0.0.0.0/") && !raw.contains("/ip6/::/")
}

pub fn ensure_peer_id_suffix(addr: &Multiaddr, peer_id: &str) -> String {
    let raw = addr.to_string();
    if raw.contains("/p2p/") {
        raw
    } else {
        format!("{raw}/p2p/{peer_id}")
    }
}
