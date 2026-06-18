use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/shutdown", post(shutdown_bridge))
}

async fn health() -> &'static str {
    "ok"
}

async fn shutdown_bridge(State(state): State<AppState>) -> StatusCode {
    state.shutdown.notify_one();
    StatusCode::NO_CONTENT
}
