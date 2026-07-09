use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use inertia_core::MediaFetchStatus;
use serde::Deserialize;

use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/media/:root_hash/fetch", post(start_media_fetch))
        .route("/media/:root_hash/status", get(media_fetch_status))
}

#[derive(Deserialize)]
struct MediaFetchQuery {
    /// Contact id of the author when downloading a shared-folder file.
    #[serde(default)]
    author_contact_id: Option<String>,
    /// When true (archive downloads), require DCUtR/direct and refuse relay bulk transfer.
    #[serde(default)]
    direct_required: bool,
}

async fn start_media_fetch(
    State(state): State<AppState>,
    Path(root_hash): Path<String>,
    Query(query): Query<MediaFetchQuery>,
) -> Result<Json<MediaFetchStatus>, (axum::http::StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    // Archive downloads: direct/DCUtR only. Video/photo may still fall back to relay.
    if query.direct_required {
        let contact = query.author_contact_id.as_deref().ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: "author_contact_id required when direct_required=true".into(),
                    code: None,
                }),
            )
        })?;
        return engine
            .start_archive_fetch(&root_hash, contact)
            .await
            .map(Json)
            .map_err(api_err);
    }
    engine
        .start_media_fetch_from(&root_hash, query.author_contact_id.as_deref(), false)
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
