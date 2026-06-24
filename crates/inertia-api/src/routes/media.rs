use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use inertia_core::MediaFetchStatus;

use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/media/:root_hash/fetch", post(start_media_fetch))
        .route("/media/:root_hash/status", get(media_fetch_status))
}

async fn start_media_fetch(
    State(state): State<AppState>,
    Path(root_hash): Path<String>,
) -> Result<Json<MediaFetchStatus>, (axum::http::StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .start_media_fetch(&root_hash)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn media_fetch_status(
    State(state): State<AppState>,
    Path(root_hash): Path<String>,
) -> Result<Json<MediaFetchStatus>, (axum::http::StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .media_fetch_status(&root_hash)
        .await
        .map(Json)
        .map_err(api_err)
}
