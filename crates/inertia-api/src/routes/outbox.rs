use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use inertia_core::OutboxEntry;

use crate::dto::RetryOutboxRequest;
use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/outbox", get(list_outbox))
        .route("/outbox/retry", post(retry_outbox))
}

async fn list_outbox(
    State(state): State<AppState>,
) -> Result<Json<Vec<OutboxEntry>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.list_outbox().await.map(Json).map_err(api_err)
}

async fn retry_outbox(
    State(state): State<AppState>,
    Json(body): Json<RetryOutboxRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .retry_outbox(&body.content_id, &body.recipient_id)
        .await
        .map_err(api_err)?;
    Ok(StatusCode::NO_CONTENT)
}
