use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use inertia_core::PurgeReport;

use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/expiry/sweep", post(expiry_sweep))
}

async fn expiry_sweep(
    State(state): State<AppState>,
) -> Result<Json<PurgeReport>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.run_expiry_sweep().await.map(Json).map_err(api_err)
}
