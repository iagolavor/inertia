//! Status polling reuses relay TCP probe results within the cache TTL.

use inertia_core::Engine;
use tempfile::tempdir;

#[tokio::test]
async fn consecutive_status_snapshots_keep_cached_relay_probe() {
    let dir = tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).await.expect("open engine");

    let first = engine.p2p_status().await;
    let second = engine.p2p_status().await;

    assert_eq!(first.relay_tcp_reachable, second.relay_tcp_reachable);
    assert_eq!(first.relay_connected, second.relay_connected);
}
