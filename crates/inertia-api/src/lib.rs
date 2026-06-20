mod dto;
mod error;
mod routes;
mod state;
mod util;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use inertia_core::Engine;
use tokio::sync::{Mutex, Notify};
use tower_http::services::{ServeDir, ServeFile};
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

    let api = routes::router();
    let mut app = Router::new()
        .nest("/api", api.clone())
        .merge(api)
        .with_state(state);

    if let Some(web_dir) = resolve_web_dir() {
        if web_dir.is_dir() {
            let index = web_dir.join("index.html");
            info!(dir = %web_dir.display(), "serving web UI");
            app = Router::new()
                .merge(app)
                .fallback_service(
                    ServeDir::new(&web_dir).not_found_service(ServeFile::new(index)),
                );
        } else {
            tracing::warn!(dir = %web_dir.display(), "INERTIA_WEB_DIR is not a directory");
        }
    }

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

/// Static UI directory for packaged installs (`web/` next to the binary).
fn resolve_web_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("INERTIA_WEB_DIR") {
        return Some(PathBuf::from(dir));
    }
    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("web")));
    beside_exe.filter(|p| p.is_dir())
}
