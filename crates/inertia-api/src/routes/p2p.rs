use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::dto::{DialRequest, FriendRequestBody, StartP2pRequest};
use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/p2p/start", post(start_p2p))
        .route("/p2p/addresses", get(p2p_addresses))
        .route("/p2p/status", get(p2p_status))
        .route("/p2p/activity", get(p2p_activity))
        .route("/p2p/share-address", get(p2p_share_address))
        .route("/p2p/dial", post(dial_peer))
        .route("/p2p/friend-request", post(send_friend_request))
}

async fn start_p2p(
    State(state): State<AppState>,
    Json(body): Json<StartP2pRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let port = body
        .listen_port
        .filter(|&p| p > 0)
        .unwrap_or_else(inertia_core::default_p2p_listen_port);
    let engine = state.engine.lock().await;
    let peer_id = engine.start_p2p(port).await.map_err(api_err)?;
    let addresses = engine.p2p_listen_addresses().await.map_err(api_err)?;
    Ok(Json(serde_json::json!({
        "peer_id": peer_id,
        "addresses": addresses,
        "listen_port": port,
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

async fn p2p_status(
    State(state): State<AppState>,
) -> Result<Json<inertia_core::P2pStatus>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    Ok(Json(engine.p2p_status().await))
}

async fn p2p_activity(
    State(state): State<AppState>,
) -> Result<Json<inertia_core::P2pActivitySnapshot>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    Ok(Json(engine.p2p_activity().await))
}

async fn p2p_share_address(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    let multiaddr = engine.connection_share_multiaddr().await.map_err(api_err)?;
    Ok(Json(serde_json::json!({ "multiaddr": multiaddr })))
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
