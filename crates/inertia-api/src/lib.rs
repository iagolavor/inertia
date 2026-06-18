mod dto;
mod error;
mod routes;
mod state;
mod util;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use inertia_core::Engine;
use tokio::sync::{Mutex, Notify};
use tracing::info;

pub use error::ApiError;
pub use state::AppState;

pub async fn run() -> anyhow::Result<()> {
    let data_dir = std::env::var("INERTIA_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data"));

    let engine = Arc::new(Mutex::new(Engine::open(&data_dir).await?));
    let shutdown = Arc::new(Notify::new());
    let state = AppState {
        engine,
        shutdown: shutdown.clone(),
    };

    let app = routes::router().with_state(state);

    let addr: SocketAddr = std::env::var("INERTIA_API_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4783".into())
        .parse()?;

    info!(%addr, "inertia-api listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.notified().await;
            info!("inertia-api shutting down");
        })
        .await?;
    Ok(())
}
