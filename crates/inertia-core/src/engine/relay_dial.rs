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

/// Max wait for relay libp2p session during invite create/accept.
pub const INVITE_RELAY_SESSION_WAIT: Duration = Duration::from_secs(20);

/// Max wait for relay circuit reservation before friend dials.
/// Event-driven: normally satisfied in well under a second.
pub const RELAY_RESERVATION_WAIT: Duration = Duration::from_secs(15);

/// Max wait for circuit reservation on the invite relay.
pub const INVITE_RELAY_RESERVATION_WAIT: Duration = Duration::from_secs(20);

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

/// Clone the node handle out of the engine mutex so waits never hold it.
async fn node_handle(p2p: &Arc<Mutex<Option<P2pNode>>>) -> Option<P2pNode> {
    p2p.lock().await.as_ref().cloned()
}

async fn dial_multiaddr(node: &P2pNode, multiaddr: &str) -> CoreResult<()> {
    let addr = multiaddr
        .parse::<Multiaddr>()
        .map_err(|e| CoreError::P2p(e.to_string()))?;
    node.dial(addr).await
}

fn dedupe_relays(priority_relay: &str, relays: &[String]) -> Vec<String> {
    let mut ordered: Vec<String> = Vec::new();
    let priority = priority_relay.trim();
    if !priority.is_empty() {
        ordered.push(priority.to_string());
    }
    for relay in relays {
        let trimmed = relay.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !ordered.iter().any(|existing| existing == trimmed) {
            ordered.push(trimmed.to_string());
        }
    }
    ordered
}

fn relay_peer_ids(relays: &[String]) -> Vec<String> {
    relays
        .iter()
        .filter_map(|relay| super::p2p::peer_id_from_multiaddr_str(relay))
        .collect()
}

/// Dial relays, open inbound circuit listeners, and wait (event-driven) for a
/// reservation on any configured relay.
async fn connect_and_reserve(
    node: &P2pNode,
    relays: &[String],
    session_wait: Duration,
    reservation_wait: Duration,
    wait_for_reservation: bool,
) -> bool {
    for relay in relays {
        match dial_multiaddr(node, relay).await {
            Ok(()) => info!(%relay, "dialed relay"),
            Err(e) => warn!(error = %e, %relay, "relay dial failed"),
        }
    }

    let peer_ids = relay_peer_ids(relays);
    if !node.wait_for_any_connected(&peer_ids, session_wait).await {
        warn!("relay session not ready on configured relays");
        return false;
    }

    node.ensure_relay_circuits(relays).await;

    if !wait_for_reservation {
        return true;
    }

    let reserved = node
        .wait_for_relay_reservation(&peer_ids, reservation_wait)
        .await;
    if reserved {
        info!("relay reservation confirmed");
    } else {
        warn!("relay reservation not confirmed on any configured relay");
    }
    reserved
}

/// Dial relays and wait for an inbound reservation before friend dials.
pub(crate) async fn bootstrap_relays_for_friend_dial(
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    relays: &[String],
) -> bool {
    if relays.is_empty() {
        return true;
    }
    let Some(node) = node_handle(p2p).await else {
        return false;
    };
    connect_and_reserve(&node, relays, RELAY_SESSION_WAIT, RELAY_RESERVATION_WAIT, true).await
}

/// Bootstrap relays for invite flows: session on any listed relay, plus a
/// reservation when the caller needs to be reachable inbound (invite create).
pub(crate) async fn bootstrap_invite_relay(
    p2p: &Arc<Mutex<Option<P2pNode>>>,
    priority_relay: &str,
    relays: &[String],
    wait_for_reservation: bool,
) -> bool {
    let ordered = dedupe_relays(priority_relay, relays);
    if ordered.is_empty() {
        return false;
    }
    let Some(node) = node_handle(p2p).await else {
        return false;
    };
    connect_and_reserve(
        &node,
        &ordered,
        INVITE_RELAY_SESSION_WAIT,
        INVITE_RELAY_RESERVATION_WAIT,
        wait_for_reservation,
    )
    .await
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

    #[test]
    fn dedupe_relays_puts_priority_first() {
        let relays = vec![
            "/ip4/1.1.1.1/tcp/9000/p2p/12D3KooWB".into(),
            "/ip4/2.2.2.2/tcp/9000/p2p/12D3KooWA".into(),
        ];
        let ordered = dedupe_relays("/ip4/2.2.2.2/tcp/9000/p2p/12D3KooWA", &relays);
        assert_eq!(ordered.len(), 2);
        assert!(ordered[0].contains("2.2.2.2"));
    }
}
