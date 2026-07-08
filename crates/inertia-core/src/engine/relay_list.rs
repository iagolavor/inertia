use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;

use crate::error::CoreResult;

use super::p2p::validate_relay_multiaddr;

/// Split comma- or newline-separated relay multiaddrs (used by env and UI paste).
pub fn parse_relay_multiaddrs(raw: &str) -> Vec<String> {
    raw.split([',', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn relay_peer_id(multiaddr: &str) -> Option<String> {
    multiaddr
        .parse::<Multiaddr>()
        .ok()?
        .iter()
        .find_map(|protocol| match protocol {
            Protocol::P2p(peer_id) => Some(peer_id.to_string()),
            _ => None,
        })
}

pub fn validate_relay_list(relays: &[String]) -> CoreResult<()> {
    for relay in relays {
        validate_relay_multiaddr(relay)?;
    }
    Ok(())
}

/// Add `new` when its relay peer id is not already in `list`.
pub fn merge_relay(list: &[String], new: &str) -> Vec<String> {
    let new = new.trim();
    if new.is_empty() {
        return list.to_vec();
    }
    let new_peer = relay_peer_id(new);
    let mut out = list.to_vec();
    if new_peer.is_some_and(|id| {
        out.iter()
            .any(|existing| relay_peer_id(existing).as_deref() == Some(id.as_str()))
    }) {
        return out;
    }
    out.push(new.to_string());
    out
}

/// Primary (first) if connected, else first connected relay in list.
pub fn select_invite_relay(relays: &[String], connected_peer_ids: &[String]) -> Option<String> {
    if relays.is_empty() {
        return None;
    }
    let is_connected = |relay: &str| {
        relay_peer_id(relay).is_some_and(|id| connected_peer_ids.iter().any(|p| p == &id))
    };
    if let Some(primary) = relays.first() {
        if is_connected(primary) {
            return Some(primary.clone());
        }
    }
    relays
        .iter()
        .find(|relay| is_connected(relay))
        .cloned()
}

/// Compare relay lists ignoring order and empty entries.
pub fn relay_lists_equivalent(a: &[String], b: &[String]) -> bool {
    fn normalize(relays: &[String]) -> Vec<String> {
        let mut out: Vec<String> = relays
            .iter()
            .map(|relay| relay.trim().to_string())
            .filter(|relay| !relay.is_empty())
            .collect();
        out.sort();
        out.dedup();
        out
    }
    normalize(a) == normalize(b)
}

pub fn relays_from_env() -> Vec<String> {
    std::env::var("INERTIA_RELAY")
        .ok()
        .map(|raw| parse_relay_multiaddrs(&raw))
        .filter(|list| !list.is_empty())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELAY_A: &str = "/ip4/203.0.113.1/tcp/9000/p2p/12D3KooWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const RELAY_B: &str = "/ip4/203.0.113.2/tcp/9000/p2p/12D3KooWBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";

    #[test]
    fn merge_relay_dedupes_by_peer_id() {
        let alt_ip = "/ip4/198.51.100.9/tcp/9000/p2p/12D3KooWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let merged = merge_relay(&[RELAY_A.to_string()], alt_ip);
        assert_eq!(merged.len(), 1);
        let merged = merge_relay(&[RELAY_A.to_string()], RELAY_B);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn select_invite_relay_prefers_primary() {
        let relays = vec![RELAY_A.to_string(), RELAY_B.to_string()];
        let peer_b = relay_peer_id(RELAY_B).unwrap();
        let chosen = select_invite_relay(&relays, &[peer_b.clone()]);
        assert_eq!(chosen.as_deref(), Some(RELAY_B));
        let peer_a = relay_peer_id(RELAY_A).unwrap();
        let chosen = select_invite_relay(&relays, &[peer_a, peer_b]);
        assert_eq!(chosen.as_deref(), Some(RELAY_A));
    }

    #[test]
    fn parse_relay_multiaddrs_splits_lines_and_commas() {
        let raw = format!("{RELAY_A},\n{RELAY_B}");
        let parsed = parse_relay_multiaddrs(&raw);
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn single_relay_list_uses_primary_for_invite() {
        let relays = vec![RELAY_A.to_string()];
        let peer_a = relay_peer_id(RELAY_A).unwrap();
        let chosen = select_invite_relay(&relays, &[peer_a]);
        assert_eq!(chosen.as_deref(), Some(RELAY_A));
    }

    #[test]
    fn merge_relay_keeps_primary_first_when_adding_second() {
        let merged = merge_relay(&[RELAY_A.to_string()], RELAY_B);
        assert_eq!(merged, vec![RELAY_A.to_string(), RELAY_B.to_string()]);
    }

    #[test]
    fn merge_relay_on_empty_list_sets_primary() {
        let merged = merge_relay(&[], RELAY_A);
        assert_eq!(merged, vec![RELAY_A.to_string()]);
    }

    #[test]
    fn relay_lists_equivalent_ignores_order() {
        let a = vec![
            RELAY_B.to_string(),
            RELAY_A.to_string(),
        ];
        let b = vec![RELAY_A.to_string(), RELAY_B.to_string()];
        assert!(relay_lists_equivalent(&a, &b));
    }

    #[test]
    fn validate_relay_list_accepts_single_primary() {
        validate_relay_list(&[RELAY_A.to_string()]).expect("single relay valid");
    }
}
