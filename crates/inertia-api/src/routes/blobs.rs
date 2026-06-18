use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

use crate::error::{api_err, ApiError};
use crate::state::AppState;
use crate::util::blob_content_type;

pub fn routes() -> Router<AppState> {
    Router::new().route("/blobs/:hash", get(get_blob))
}

async fn get_blob(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Response, (StatusCode, axum::Json<ApiError>)> {
    let engine = state.engine.lock().await;
    let data = engine.read_blob(&hash).await.map_err(api_err)?;
    Ok((
        [(header::CONTENT_TYPE, blob_content_type(&data))],
        data,
    )
        .into_response())
}
