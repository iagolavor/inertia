use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use inertia_core::{FeedItem, PostComment, MAX_THUMB_BYTES, MAX_VIDEO_BYTES};

use crate::dto::{AddCommentRequest, CreatePostRequest, CreateVideoPostRequest};
use crate::error::{api_err, ApiError};
use crate::state::AppState;
use crate::util::{base64_decode, blob_too_large_err, MAX_BLOB_BYTES};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/posts", post(create_post))
        .route("/posts/:id", get(get_post))
        .route(
            "/posts/:id/comments",
            get(list_post_comments).post(add_post_comment),
        )
}

pub fn video_routes() -> Router<AppState> {
    Router::new().route("/posts/video", post(create_video_post))
}

async fn create_post(
    State(state): State<AppState>,
    Json(body): Json<CreatePostRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;

    let media_ref = if let Some(ref b64) = body.media_base64 {
        let data = base64_decode(b64)?;
        if data.len() > MAX_BLOB_BYTES {
            return Err(blob_too_large_err());
        }
        Some(engine.store_blob(&data).await.map_err(api_err)?)
    } else {
        None
    };

    let content_id = engine
        .send_post(&body.body, media_ref.as_deref())
        .await
        .map_err(api_err)?;
    Ok(Json(serde_json::json!({ "content_id": content_id })))
}

async fn create_video_post(
    State(state): State<AppState>,
    Json(body): Json<CreateVideoPostRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;

    let video = base64_decode(&body.video_base64)?;
    if video.len() > MAX_VIDEO_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiError {
                error: format!(
                    "video exceeds {} MB limit",
                    MAX_VIDEO_BYTES / (1024 * 1024)
                ),
                code: None,
            }),
        ));
    }

    let thumb = base64_decode(&body.thumb_base64)?;
    if thumb.len() > MAX_THUMB_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiError {
                error: format!(
                    "thumbnail exceeds {} KB limit",
                    MAX_THUMB_BYTES / 1024
                ),
                code: None,
            }),
        ));
    }

    let content_id = engine
        .send_video_post(&body.body, &thumb, &video, body.duration_ms)
        .await
        .map_err(api_err)?;
    Ok(Json(serde_json::json!({ "content_id": content_id })))
}

async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<FeedItem>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .get_feed_item(&id)
        .await
        .map_err(api_err)?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    error: "post not found".into(),
                    code: None,
                }),
            )
        })
        .map(Json)
}

async fn list_post_comments(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<PostComment>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .list_post_comments(&id)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn add_post_comment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AddCommentRequest>,
) -> Result<Json<PostComment>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .add_post_comment(&id, &body.body)
        .await
        .map(Json)
        .map_err(api_err)
}
