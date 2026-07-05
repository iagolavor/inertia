//! SSE `/p2p/events` streams UI activity pushed from the engine broadcast bus.

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use inertia_api::{routes, AppState};
use inertia_core::Engine;
use tempfile::tempdir;
use tokio::sync::{Mutex, Notify};

#[tokio::test]
async fn p2p_events_sse_streams_ui_activity() {
    let dir = tempdir().expect("tempdir");
    let engine = Arc::new(Mutex::new(Engine::open(dir.path()).await.expect("open engine")));
    let state = AppState {
        engine: engine.clone(),
        shutdown: Arc::new(Notify::new()),
    };
    let app = routes::router().with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let client = reqwest::Client::new();
    let url = format!("http://{addr}/p2p/events");
    let response = client
        .get(&url)
        .header("Accept", "text/event-stream")
        .send()
        .await
        .expect("sse request");
    assert_eq!(response.status(), 200);

    let mut stream = response.bytes_stream();

    {
        let eng = engine.lock().await;
        eng.push_ui_activity("message_received", "Alice: hello")
            .await;
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut buf = String::new();
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                buf.push_str(&String::from_utf8_lossy(&chunk));
                if buf.contains("message_received") && buf.contains("Alice: hello") {
                    return;
                }
            }
            Ok(Some(Err(e))) => panic!("stream error: {e}"),
            Ok(None) => break,
            Err(_) => continue,
        }
    }
    panic!("did not receive SSE event in time: {buf}");
}
