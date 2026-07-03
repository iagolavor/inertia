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
    let relay_update = body.relay_multiaddrs.map(|relays| {
        relays
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
    });
    let announce_update = body.p2p_announce.as_ref().map(|value| {
        if value.trim().is_empty() {
            None
        } else {
            Some(value.trim().to_string())
        }
    });
    let web_origin_update = body.web_origin.as_ref().map(|value| {
        if value.trim().is_empty() {
            None
        } else {
            Some(value.trim().to_string())
        }
    });

    let engine = state.engine.lock().await;
    engine
        .update_settings(
            body.feed_history_enabled,
            body.p2p_listen_port,
            relay_update,
            announce_update,
            web_origin_update,
        )
        .await
        .map(Json)
        .map_err(api_err)
}
