use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use inertia_core::Engine;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Notify};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[derive(Clone)]
struct AppState {
    engine: Arc<Mutex<Engine>>,
    shutdown: Arc<Notify>,
}

#[derive(Deserialize)]
struct InitIdentityRequest {
    display_name: String,
}

#[derive(Deserialize)]
struct CreateInviteRequest {
    web_origin: Option<String>,
}

#[derive(Deserialize)]
struct InviteInput {
    invite: String,
}

#[derive(Deserialize)]
struct AddContactRequest {
    id: String,
    display_name: String,
    signing_pubkey: String,
    encryption_pubkey: String,
}

#[derive(Deserialize)]
struct SendMessageRequest {
    recipient_id: String,
    body: String,
}

#[derive(Deserialize)]
struct DialRequest {
    multiaddr: String,
}

#[derive(Deserialize)]
struct FriendRequestBody {
    contact_id: String,
    multiaddr: String,
}

#[derive(Deserialize)]
struct RetryOutboxRequest {
    content_id: String,
    recipient_id: String,
}

#[derive(Deserialize)]
struct StartP2pRequest {
    listen_port: Option<u16>,
}

#[derive(Deserialize)]
struct CreatePostRequest {
    body: String,
    media_base64: Option<String>,
}

#[derive(Deserialize)]
struct UploadPhotoRequest {
    data_base64: String,
    caption: Option<String>,
}

#[derive(Deserialize)]
struct UpdateSettingsRequest {
    feed_history_enabled: bool,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let data_dir = std::env::var("INERTIA_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data"));

    let engine = Arc::new(Mutex::new(Engine::open(&data_dir).await?));
    let shutdown = Arc::new(Notify::new());
    let state = AppState {
        engine,
        shutdown: shutdown.clone(),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/shutdown", post(shutdown_bridge))
        .route("/identity", get(get_identity).post(init_identity))
        .route("/invite", post(create_invite))
        .route("/invite/preview", post(preview_invite))
        .route("/invite/accept", post(accept_invite))
        .route("/contacts", get(list_contacts).post(add_contact))
        .route("/inbox", get(list_inbox))
        .route("/outbox", get(list_outbox))
        .route("/messages", post(send_message))
        .route("/posts", post(create_post))
        .route("/feed", get(list_feed))
        .route("/feed/backup", get(export_feed_backup))
        .route(
            "/feed/restore",
            post(restore_feed_backup).layer(DefaultBodyLimit::max(128 * 1024 * 1024)),
        )
        .route("/settings", get(get_settings).patch(update_settings))
        .route("/profile/photos", get(list_profile_photos).post(upload_profile_photo))
        .route("/blobs/:hash", get(get_blob))
        .route("/p2p/start", post(start_p2p))
        .route("/p2p/addresses", get(p2p_addresses))
        .route("/p2p/dial", post(dial_peer))
        .route("/p2p/friend-request", post(send_friend_request))
        .route("/outbox/retry", post(retry_outbox))
        .route("/expiry/sweep", post(expiry_sweep))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let addr: SocketAddr = std::env::var("INERTIA_API_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4783".into())
        .parse()?;

    info!(%addr, "inertia-api listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.notified().await;
            info!("inertia-api shutting down");
        })
        .await?;
    Ok(())
}

async fn shutdown_bridge(State(state): State<AppState>) -> StatusCode {
    state.shutdown.notify_one();
    StatusCode::NO_CONTENT
}

async fn health() -> &'static str {
    "ok"
}

async fn get_identity(
    State(state): State<AppState>,
) -> Result<Json<inertia_core::Identity>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    Ok(Json(engine.identity_snapshot().await))
}

async fn init_identity(
    State(state): State<AppState>,
    Json(body): Json<InitIdentityRequest>,
) -> Result<Json<inertia_core::Identity>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .initialize_identity(&body.display_name)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn list_contacts(
    State(state): State<AppState>,
) -> Result<Json<Vec<inertia_core::Contact>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.list_contacts().await.map(Json).map_err(api_err)
}

async fn add_contact(
    State(state): State<AppState>,
    Json(body): Json<AddContactRequest>,
) -> Result<Json<inertia_core::Contact>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .add_pending_contact(
            &body.id,
            &body.display_name,
            &body.signing_pubkey,
            &body.encryption_pubkey,
        )
        .await
        .map(Json)
        .map_err(api_err)
}

async fn create_invite(
    State(state): State<AppState>,
    Json(body): Json<CreateInviteRequest>,
) -> Result<Json<inertia_core::InviteResponse>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .create_invite(body.web_origin.as_deref())
        .await
        .map(Json)
        .map_err(api_err)
}

async fn preview_invite(
    State(state): State<AppState>,
    Json(body): Json<InviteInput>,
) -> Result<Json<inertia_core::InvitePreview>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .preview_invite(&body.invite)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn accept_invite(
    State(state): State<AppState>,
    Json(body): Json<InviteInput>,
) -> Result<Json<inertia_core::Contact>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .accept_invite(&body.invite)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn list_inbox(
    State(state): State<AppState>,
) -> Result<Json<Vec<inertia_core::InboxEntry>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.list_inbox().await.map(Json).map_err(api_err)
}

async fn list_outbox(
    State(state): State<AppState>,
) -> Result<Json<Vec<inertia_core::OutboxEntry>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.list_outbox().await.map(Json).map_err(api_err)
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

async fn create_post(
    State(state): State<AppState>,
    Json(body): Json<CreatePostRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;

    let media_ref = if let Some(ref b64) = body.media_base64 {
        let data = base64_decode(b64)?;
        if data.len() > MAX_BLOB_BYTES {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(ApiError {
                    error: format!("image exceeds {} MB limit", MAX_BLOB_BYTES / (1024 * 1024)),
                }),
            ));
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

async fn list_feed(
    State(state): State<AppState>,
) -> Result<Json<Vec<inertia_core::FeedItem>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.list_feed().await.map(Json).map_err(api_err)
}

async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<inertia_core::AppSettings>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.get_settings().await.map(Json).map_err(api_err)
}

async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<UpdateSettingsRequest>,
) -> Result<Json<inertia_core::AppSettings>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .set_feed_history_enabled(body.feed_history_enabled)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn export_feed_backup(
    State(state): State<AppState>,
) -> Result<Json<inertia_core::FeedBackup>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.export_feed_backup().await.map(Json).map_err(api_err)
}

async fn restore_feed_backup(
    State(state): State<AppState>,
    Json(body): Json<inertia_core::FeedBackup>,
) -> Result<Json<inertia_core::FeedRestoreReport>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .import_feed_backup(body)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn list_profile_photos(
    State(state): State<AppState>,
) -> Result<Json<Vec<inertia_core::ProfilePhoto>>, (StatusCode, Json<ApiError>)> {
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
) -> Result<Json<inertia_core::PublishPhotoResult>, (StatusCode, Json<ApiError>)> {
    let data = base64_decode(&body.data_base64)?;
    if data.len() > MAX_BLOB_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiError {
                error: format!("image exceeds {} MB limit", MAX_BLOB_BYTES / (1024 * 1024)),
            }),
        ));
    }
    let engine = state.engine.lock().await;
    engine
        .publish_profile_photo(&data, body.caption.as_deref())
        .await
        .map(Json)
        .map_err(api_err)
}

const MAX_BLOB_BYTES: usize = 2 * 1024 * 1024;

async fn get_blob(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Response, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    let data = engine.read_blob(&hash).await.map_err(api_err)?;
    Ok((
        [(header::CONTENT_TYPE, blob_content_type(&data))],
        data,
    )
        .into_response())
}

fn blob_content_type(data: &[u8]) -> &'static str {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png"
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        "image/gif"
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn base64_decode(input: &str) -> Result<Vec<u8>, (StatusCode, Json<ApiError>)> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: format!("invalid base64: {e}"),
                }),
            )
        })
}

async fn start_p2p(
    State(state): State<AppState>,
    Json(body): Json<StartP2pRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let port = body.listen_port.unwrap_or(0);
    let peer_id = {
        let mut engine = state.engine.lock().await;
        engine.start_p2p(port).await.map_err(api_err)?
    };
    let addresses = {
        let engine = state.engine.lock().await;
        engine.p2p_listen_addresses().await.map_err(api_err)?
    };
    Ok(Json(serde_json::json!({
        "peer_id": peer_id,
        "addresses": addresses,
    })))
}

async fn p2p_addresses(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    let peer_id = engine.peer_id().await;
    let addresses = engine.p2p_listen_addresses().await.map_err(api_err)?;
    Ok(Json(serde_json::json!({
        "peer_id": peer_id,
        "addresses": addresses,
    })))
}

async fn dial_peer(
    State(state): State<AppState>,
    Json(body): Json<DialRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.dial_peer(&body.multiaddr).await.map_err(api_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn send_friend_request(
    State(state): State<AppState>,
    Json(body): Json<FriendRequestBody>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .send_friend_request(&body.contact_id, &body.multiaddr)
        .await
        .map_err(api_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn retry_outbox(
    State(state): State<AppState>,
    Json(body): Json<RetryOutboxRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .retry_outbox(&body.content_id, &body.recipient_id)
        .await
        .map_err(api_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn expiry_sweep(
    State(state): State<AppState>,
) -> Result<Json<inertia_core::PurgeReport>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.run_expiry_sweep().await.map(Json).map_err(api_err)
}

fn api_err(e: inertia_core::CoreError) -> (StatusCode, Json<ApiError>) {
    let status = match &e {
        inertia_core::CoreError::Invite(_) => StatusCode::BAD_REQUEST,
        inertia_core::CoreError::ProfileAlreadyExists => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ApiError {
            error: e.to_string(),
        }),
    )
}
