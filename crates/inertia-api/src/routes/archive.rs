use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use inertia_core::{
    ArchiveEntry, ArchiveFolder, ArchiveUploadStatus, CHUNK_SIZE, MAX_ARCHIVE_FILE_BYTES,
};

use crate::dto::{AddArchiveEntryRequest, BeginArchiveUploadRequest, CreateArchiveFolderRequest};
use crate::error::{api_err, ApiError};
use crate::state::AppState;
use crate::util::base64_decode;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/archive/folders", get(list_folders).post(create_folder))
        .route("/archive/folders/:folder_id", delete(delete_folder))
        .route(
            "/archive/folders/:folder_id/entries",
            get(list_entries).post(add_entry),
        )
        .route(
            "/archive/folders/:folder_id/uploads",
            post(begin_upload),
        )
        .route("/archive/uploads/:upload_id", get(upload_status))
        .route(
            "/archive/uploads/:upload_id/complete",
            post(complete_upload),
        )
        .route("/archive/entries/:entry_id", delete(delete_entry))
}

/// Chunked ingest routes only (raw PUT body). Applied with a per-chunk body limit.
pub fn chunk_routes() -> Router<AppState> {
    Router::new().route(
        "/archive/uploads/:upload_id/chunks/:index",
        put(put_chunk),
    )
}

async fn list_folders(
    State(state): State<AppState>,
) -> Result<Json<Vec<ArchiveFolder>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.list_archive_folders().await.map(Json).map_err(api_err)
}

async fn create_folder(
    State(state): State<AppState>,
    Json(body): Json<CreateArchiveFolderRequest>,
) -> Result<Json<ArchiveFolder>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .create_archive_folder(&body.name)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn delete_folder(
    State(state): State<AppState>,
    Path(folder_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .delete_archive_folder(&folder_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(api_err)
}

async fn list_entries(
    State(state): State<AppState>,
    Path(folder_id): Path<String>,
) -> Result<Json<Vec<ArchiveEntry>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .list_archive_entries(&folder_id)
        .await
        .map(Json)
        .map_err(api_err)
}

/// Legacy base64 single-shot upload (kept for compat; Files tab uses chunked ingest).
async fn add_entry(
    State(state): State<AppState>,
    Path(folder_id): Path<String>,
    Json(body): Json<AddArchiveEntryRequest>,
) -> Result<Json<ArchiveEntry>, (StatusCode, Json<ApiError>)> {
    let data = base64_decode(&body.data_base64)?;
    if data.len() > MAX_ARCHIVE_FILE_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiError {
                error: format!(
                    "file exceeds {} MB limit (use chunked upload for larger files)",
                    MAX_ARCHIVE_FILE_BYTES / (1024 * 1024)
                ),
                code: None,
            }),
        ));
    }
    let mime = body
        .mime
        .unwrap_or_else(|| "application/octet-stream".into());
    let engine = state.engine.lock().await;
    engine
        .add_archive_entry(&folder_id, &body.name, &data, &mime)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn begin_upload(
    State(state): State<AppState>,
    Path(folder_id): Path<String>,
    Json(body): Json<BeginArchiveUploadRequest>,
) -> Result<Json<ArchiveUploadStatus>, (StatusCode, Json<ApiError>)> {
    let mime = body
        .mime
        .unwrap_or_else(|| "application/octet-stream".into());
    let engine = state.engine.lock().await;
    engine
        .begin_archive_upload(&folder_id, &body.name, &mime, body.total_bytes)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn upload_status(
    State(state): State<AppState>,
    Path(upload_id): Path<String>,
) -> Result<Json<ArchiveUploadStatus>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .archive_upload_status(&upload_id)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn put_chunk(
    State(state): State<AppState>,
    Path((upload_id, index)): Path<(String, u32)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ArchiveUploadStatus>, (StatusCode, Json<ApiError>)> {
    if body.len() > CHUNK_SIZE {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiError {
                error: format!("chunk exceeds {} bytes", CHUNK_SIZE),
                code: None,
            }),
        ));
    }
    let expected_hash = headers
        .get("x-chunk-hash")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: "x-chunk-hash header required".into(),
                    code: None,
                }),
            )
        })?;
    let engine = state.engine.lock().await;
    engine
        .put_archive_upload_chunk(&upload_id, index, expected_hash, &body)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn complete_upload(
    State(state): State<AppState>,
    Path(upload_id): Path<String>,
) -> Result<Json<ArchiveEntry>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .complete_archive_upload(&upload_id)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn delete_entry(
    State(state): State<AppState>,
    Path(entry_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .delete_archive_entry(&entry_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(api_err)
}
