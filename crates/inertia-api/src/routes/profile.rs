use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get};
use axum::{Json, Router};
use inertia_core::{ProfileComment, ProfilePhoto, PublishPhotoResult};

use crate::dto::{AddCommentRequest, UploadPhotoRequest};
use crate::error::{api_err, ApiError};
use crate::state::AppState;
use crate::util::{base64_decode, blob_too_large_err, MAX_BLOB_BYTES};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/profile/photos",
            get(list_profile_photos).post(upload_profile_photo),
        )
        .route("/profile/photos/:item_id", delete(delete_profile_photo))
        .route(
            "/profile/items/:item_id/comments",
            get(list_own_profile_comments).post(add_own_profile_comment),
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

async fn delete_profile_photo(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .delete_profile_photo(&item_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(api_err)
}

async fn list_own_profile_comments(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> Result<Json<Vec<ProfileComment>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .list_profile_comments(&item_id)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn add_own_profile_comment(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    Json(body): Json<AddCommentRequest>,
) -> Result<Json<ProfileComment>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .add_profile_comment(&item_id, &body.body, None)
        .await
        .map(Json)
        .map_err(api_err)
}
