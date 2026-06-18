use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use inertia_core::AppSettings;

use crate::dto::UpdateSettingsRequest;
use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/settings", get(get_settings).patch(update_settings))
}

async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<AppSettings>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.get_settings().await.map(Json).map_err(api_err)
}

async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<UpdateSettingsRequest>,
) -> Result<Json<AppSettings>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .set_feed_history_enabled(body.feed_history_enabled)
        .await
        .map(Json)
        .map_err(api_err)
}
