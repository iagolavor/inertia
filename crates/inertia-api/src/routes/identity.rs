use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use inertia_core::Identity;

use crate::dto::{InitIdentityRequest, UpdateProfileRequest, UploadPhotoRequest};
use crate::error::{api_err, ApiError};
use crate::state::AppState;
use crate::util::{base64_decode, blob_too_large_err, MAX_BLOB_BYTES};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/identity",
            get(get_identity)
                .post(init_identity)
                .patch(update_profile),
        )
        .route("/identity/update", post(update_profile))
        .route("/identity/avatar", post(upload_avatar))
}

async fn get_identity(
    State(state): State<AppState>,
) -> Result<Json<Identity>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    Ok(Json(engine.identity_snapshot().await))
}

async fn init_identity(
    State(state): State<AppState>,
    Json(body): Json<InitIdentityRequest>,
) -> Result<Json<Identity>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .initialize_identity(&body.display_name)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn update_profile(
    State(state): State<AppState>,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<Identity>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .update_profile(&body.display_name, body.bio.unwrap_or_default())
        .await
        .map(Json)
        .map_err(api_err)
}

async fn upload_avatar(
    State(state): State<AppState>,
    Json(body): Json<UploadPhotoRequest>,
) -> Result<Json<Identity>, (StatusCode, Json<ApiError>)> {
    let data = base64_decode(&body.data_base64)?;
    if data.len() > MAX_BLOB_BYTES {
        return Err(blob_too_large_err());
    }
    let engine = state.engine.lock().await;
    engine.set_avatar(&data).await.map(Json).map_err(api_err)
}
