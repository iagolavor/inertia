use libp2p::relay;
use tracing::info;

/// Relay resource caps for a private or community VPS (not public federation).
/// Defaults align with libp2p reservation cap (128) and a 4x libp2p circuit default (64).
pub fn relay_config() -> relay::Config {
    let max_reservations = env_usize("INERTIA_RELAY_MAX_RESERVATIONS", 128);
    let max_reservations_per_peer = env_usize("INERTIA_RELAY_MAX_RESERVATIONS_PER_PEER", 4);
    let max_circuits = env_usize("INERTIA_RELAY_MAX_CIRCUITS", 64);
    let max_circuits_per_peer = env_usize("INERTIA_RELAY_MAX_CIRCUITS_PER_PEER", 4);

    let config = relay::Config {
        max_reservations,
        max_reservations_per_peer,
        max_circuits,
        max_circuits_per_peer,
        ..relay::Config::default()
    };

    info!(
        max_reservations,
        max_reservations_per_peer,
        max_circuits,
        max_circuits_per_peer,
        "relay resource limits"
    );

    config
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(default)
}
