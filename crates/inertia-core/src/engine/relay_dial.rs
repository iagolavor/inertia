use std::sync::Arc;
use std::time::Duration;

use libp2p::Multiaddr;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::error::{CoreError, CoreResult};
use crate::p2p::{filter_friend_multiaddrs, relay_circuit_dial_addr, P2pNode};
use crate::storage::Contact;

/// Cap outbound dials per friend during a redial pass.
pub const MAX_DIALS_PER_CONTACT: usize = 3;

/// Max wait for relay libp2p session before friend dials.
pub const RELAY_SESSION_WAIT: Duration = Duration::from_secs(15);

/// Pause after relay session when reservation event is slow.
pub const RELAY_RESERVATION_PAUSE: Duration = Duration::from_secs(3);

/// Max wait for relay circuit reservation before friend dials.
pub const RELAY_RESERVATION_WAIT: Duration = Duration::from_secs(12);

/// Build friend dial targets: relay circuits only (requires configured relays + peer id).
pub fn contact_dial_addrs(contact: &Contact, relays: &[String]) -> Vec<String> {
    if relays.is_empty() {
        return Vec::new();
    }
    let mut addrs = Vec::new();
    if let Some(peer_id) = contact.peer_id.as_deref() {
        for relay in relays {
            if let Some(circuit) = relay_circuit_dial_addr(relay, peer_id) {
                addrs.push(circuit);
            }
        }
    }
    addrs.extend(filter_friend_multiaddrs(&contact.multiaddrs));
    sort_contact_dial_addrs(&addrs)
}

/// Sort contact dial addresses: relay circuits first, then other circuit paths.
pub fn sort_contact_dial_addrs(addrs: &[String]) -> Vec<String> {
    let mut sorted: Vec<String> = addrs.to_vec();
    sorted.sort_by_key(|addr| dial_addr_rank(addr));
    sorted.dedup();
    sorted
}

fn dial_addr_rank(addr: &str) -> u8 {
    if addr.contains("/p2p-circuit/") {
        0
    } else {
        1
    }
}

async fn dial_multiaddr(p2p: &Arc<Mutex<Option<P2pNode>>>, multiaddr: &str) -> CoreResult<()> {
    let guard = p2p.lock().await;
    let node = guard
        .as_ref()
        .ok_or_else(|| CoreError::P2p("p2p not started".into()))?;
    let addr = multiaddr
        .parse::<Multiaddr>()
        .map_err(|e| CoreError::P2p(e.to_string()))?;
    node.dial(addr).await
}

/// Dial configured relays, wait for session + circuit reservation before friend dials.
pub(crate) async fn bootstrap_relays_for_friend_dial(
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    relays: &[String],
) -> bool {
    if relays.is_empty() {
        return true;
    }

    {
        let guard = p2p.lock().await;
        if let Some(node) = guard.as_ref() {
            node.ensure_relay_circuits(relays).await;
        }
    }

    for relay in relays {
        match dial_multiaddr(p2p, relay).await {
            Ok(()) => info!(%relay, "dialed relay before friend redial"),
            Err(e) => warn!(error = %e, %relay, "relay dial failed before friend redial"),
        }
    }

    let relay_peer_ids: Vec<String> = relays
        .iter()
        .filter_map(|relay| super::p2p::peer_id_from_multiaddr_str(relay))
        .collect();

    let relay_ready =
        wait_for_relay_connected(p2p, &relay_peer_ids, RELAY_SESSION_WAIT).await;
    if relay_ready {
        {
            let guard = p2p.lock().await;
            if let Some(node) = guard.as_ref() {
                node.ensure_relay_circuits(relays).await;
            }
        }
        let reserved = {
            let guard = p2p.lock().await;
            match guard.as_ref() {
                Some(node) => {
                    node.wait_for_relay_reservation(&relay_peer_ids, RELAY_RESERVATION_WAIT)
                        .await
                }
                None => false,
            }
        };
        if !reserved {
            warn!("relay reservation not confirmed — pausing before friend dials");
            tokio::time::sleep(RELAY_RESERVATION_PAUSE).await;
        }
    } else {
        warn!("relay session not ready before friend dials");
    }
    relay_ready
}

pub(crate) async fn wait_for_relay_connected(
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    relay_peer_ids: &[String],
    timeout: Duration,
) -> bool {
    if relay_peer_ids.is_empty() {
        return true;
    }
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let connected = {
            let guard = p2p.lock().await;
            match guard.as_ref() {
                Some(node) => node.connected_peer_ids().await,
                None => return false,
            }
        };
        if relay_peer_ids
            .iter()
            .any(|relay_id| connected.iter().any(|peer| peer == relay_id))
        {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_contact_dial_addrs_prefers_circuit_paths() {
        let addrs = vec![
            "/ip4/8.8.8.8/tcp/4784".into(),
            "/ip4/1.2.3.4/tcp/9000/p2p/12D3KooWExampleRelay/p2p-circuit/p2p/12D3KooWExampleFriend"
                .into(),
        ];
        let sorted = sort_contact_dial_addrs(&addrs);
        assert!(sorted[0].contains("/p2p-circuit/"));
    }

    #[test]
    fn contact_dial_addrs_builds_circuits_from_relays() {
        let contact = Contact {
            id: "c1".into(),
            phone_hash: None,
            display_name: "Friend".into(),
            peer_id: Some("12D3KooWFriend".into()),
            signing_pubkey: "pk".into(),
            encryption_pubkey: "ek".into(),
            last_seen: None,
            connection_state: crate::storage::ConnectionState::Offline,
            multiaddrs: vec!["/ip4/192.168.0.5/tcp/4784".into()],
        };
        let relays = vec![
            "/ip4/1.2.3.4/tcp/9000/p2p/12D3KooWExampleRelay".into(),
        ];
        let addrs = contact_dial_addrs(&contact, &relays);
        assert_eq!(addrs.len(), 1);
        assert!(addrs[0].contains("/p2p-circuit/"));
        assert!(!addrs[0].contains("192.168"));
    }
}
