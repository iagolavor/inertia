use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use inertia_core::InboxEntry;

use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/inbox", get(list_inbox))
}

async fn list_inbox(
    State(state): State<AppState>,
) -> Result<Json<Vec<InboxEntry>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.list_inbox().await.map(Json).map_err(api_err)
}
