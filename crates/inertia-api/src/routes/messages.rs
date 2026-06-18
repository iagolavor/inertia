use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};

use crate::dto::SendMessageRequest;
use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/messages", post(send_message))
}

async fn send_message(
    State(state): State<AppState>,
    Json(body): Json<SendMessageRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    let content_id = engine
        .send_message(&body.recipient_id, &body.body)
        .await
        .map_err(api_err)?;
    Ok(Json(serde_json::json!({ "content_id": content_id })))
}
