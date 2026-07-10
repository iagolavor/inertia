use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;

use crate::error::{api_err, ApiError};
use crate::state::AppState;
use crate::util::{blob_content_type, content_disposition_attachment};

pub fn routes() -> Router<AppState> {
    Router::new().route("/blobs/:hash", get(get_blob))
}

#[derive(Debug, Deserialize)]
struct BlobQuery {
    download: Option<String>,
}

async fn get_blob(
    State(state): State<AppState>,
    Path(hash): Path<String>,
    Query(query): Query<BlobQuery>,
) -> Result<Response, (StatusCode, axum::Json<ApiError>)> {
    let engine = state.engine.lock().await;
    let data = engine.read_blob(&hash).await.map_err(api_err)?;
    let mut response = (
        [(header::CONTENT_TYPE, blob_content_type(&data))],
        data,
    )
        .into_response();
    if let Some(name) = query.download.filter(|n| !n.is_empty()) {
        if let Ok(value) = HeaderValue::from_str(&content_disposition_attachment(&name)) {
            response.headers_mut().insert(header::CONTENT_DISPOSITION, value);
        }
    }
    Ok(response)
}
