use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use inertia_core::{Contact, InvitePreview, InviteResponse};

use crate::dto::{CreateInviteRequest, InviteInput};
use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/invite", post(create_invite))
        .route("/invite/preview", post(preview_invite))
        .route("/invite/accept", post(accept_invite))
}

async fn create_invite(
    State(state): State<AppState>,
    Json(body): Json<CreateInviteRequest>,
) -> Result<Json<InviteResponse>, (StatusCode, Json<ApiError>)> {
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
) -> Result<Json<InvitePreview>, (StatusCode, Json<ApiError>)> {
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
) -> Result<Json<Contact>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .accept_invite(&body.invite)
        .await
        .map(Json)
        .map_err(api_err)
}
