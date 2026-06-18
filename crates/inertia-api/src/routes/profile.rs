use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use inertia_core::{ProfilePhoto, PublishPhotoResult};

use crate::dto::UploadPhotoRequest;
use crate::error::{api_err, ApiError};
use crate::state::AppState;
use crate::util::{base64_decode, blob_too_large_err, MAX_BLOB_BYTES};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/profile/photos",
        get(list_profile_photos).post(upload_profile_photo),
    )
}

async fn list_profile_photos(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProfilePhoto>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .list_profile_photos()
        .await
        .map(Json)
        .map_err(api_err)
}

async fn upload_profile_photo(
    State(state): State<AppState>,
    Json(body): Json<UploadPhotoRequest>,
) -> Result<Json<PublishPhotoResult>, (StatusCode, Json<ApiError>)> {
    let data = base64_decode(&body.data_base64)?;
    if data.len() > MAX_BLOB_BYTES {
        return Err(blob_too_large_err());
    }
    let engine = state.engine.lock().await;
    engine
        .publish_profile_photo(&data, body.caption.as_deref())
        .await
        .map(Json)
        .map_err(api_err)
}
