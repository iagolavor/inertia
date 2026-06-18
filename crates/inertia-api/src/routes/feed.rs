use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use inertia_core::{FeedBackup, FeedItem, FeedRestoreReport};

use crate::error::{api_err, ApiError};
use crate::state::AppState;

const FEED_RESTORE_BODY_LIMIT: usize = 128 * 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/feed", get(list_feed))
        .route("/feed/backup", get(export_feed_backup))
        .route(
            "/feed/restore",
            post(restore_feed_backup).layer(DefaultBodyLimit::max(FEED_RESTORE_BODY_LIMIT)),
        )
}

async fn list_feed(
    State(state): State<AppState>,
) -> Result<Json<Vec<FeedItem>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.list_feed().await.map(Json).map_err(api_err)
}

async fn export_feed_backup(
    State(state): State<AppState>,
) -> Result<Json<FeedBackup>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.export_feed_backup().await.map(Json).map_err(api_err)
}

async fn restore_feed_backup(
    State(state): State<AppState>,
    Json(body): Json<FeedBackup>,
) -> Result<Json<FeedRestoreReport>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .import_feed_backup(body)
        .await
        .map(Json)
        .map_err(api_err)
}
