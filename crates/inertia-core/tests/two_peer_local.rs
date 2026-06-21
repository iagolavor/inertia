//! Two local P2P peers connect over loopback (Phase 5 integration test).

use std::time::Duration;

use inertia_core::Engine;
use tempfile::tempdir;
use tokio::time::sleep;

#[tokio::test]
async fn two_local_peers_connect_over_tcp() {
    let dir_a = tempdir().expect("tempdir a");
    let dir_b = tempdir().expect("tempdir b");

    let engine_a = Engine::open(dir_a.path()).await.expect("open a");
    let engine_b = Engine::open(dir_b.path()).await.expect("open b");

    engine_a
        .update_settings(None, Some(19210), None, None, None)
        .await
        .expect("settings a");
    engine_b
        .update_settings(None, Some(19211), None, None, None)
        .await
        .expect("settings b");

    engine_a
        .initialize_identity("Alice")
        .await
        .expect("identity a");
    engine_b
        .initialize_identity("Bob")
        .await
        .expect("identity b");

    engine_a.ensure_p2p_started().await.expect("p2p a");
    engine_b.ensure_p2p_started().await.expect("p2p b");

    let peer_a = engine_a.peer_id().await.expect("peer a");
    let peer_b = engine_b.peer_id().await.expect("peer b");

    let addr_a = format!("/ip4/127.0.0.1/tcp/19210/p2p/{peer_a}");
    let addr_b = format!("/ip4/127.0.0.1/tcp/19211/p2p/{peer_b}");

    engine_a.dial_peer(&addr_b).await.expect("dial b from a");
    engine_b.dial_peer(&addr_a).await.expect("dial a from b");

    let mut connected = false;
    for _ in 0..100 {
        sleep(Duration::from_millis(100)).await;
        let status_a = engine_a.p2p_status().await;
        if status_a.connected_peer_ids.iter().any(|p| p == &peer_b) {
            connected = true;
            break;
        }
    }

    assert!(
        connected,
        "peers did not connect within 10s (a saw: {:?})",
        engine_a.p2p_status().await.connected_peer_ids
    );
}
