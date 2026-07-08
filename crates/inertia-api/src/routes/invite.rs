use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use inertia_core::{Contact, Engine, FriendInvite, InvitePreview, InviteReadiness, InviteResponse};

use crate::dto::{CreateInviteRequest, InviteInput};
use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/invite/readiness", get(invite_readiness))
        .route("/invite", post(create_invite))
        .route("/invite/preview", post(preview_invite))
        .route("/invite/accept", post(accept_invite))
}

async fn invite_readiness(
    State(state): State<AppState>,
) -> Result<Json<InviteReadiness>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    Ok(Json(engine.invite_readiness().await))
}

// Invite create/accept use a two-phase relay bootstrap so the engine mutex is not held
// during dial + wait (those can take up to ~20s).
//
// Phase 1 (here): plan under a short lock, then `bootstrap_invite_relay_only` without the lock.
// Phase 2 (engine): `create_invite` / `accept_invite` call `ensure_invite_relay_ready` with
// `require_reservation: false` as a quick reconcile (connection check, apply relay list).
//
// Create waits for an inbound circuit reservation (`wait_for_reservation: true`) because the
// inviter must be dialable on the relay. Accept only needs an outbound relay session
// (`wait_for_reservation: false`) before redialing the inviter's circuit addresses.
async fn create_invite(
    State(state): State<AppState>,
    Json(body): Json<CreateInviteRequest>,
) -> Result<Json<InviteResponse>, (StatusCode, Json<ApiError>)> {
    let (p2p, relays, relay_multiaddr) = {
        let engine = state.engine.lock().await;
        engine
            .plan_invite_create_bootstrap()
            .await
            .map_err(api_err)?
    };

    let reserved = Engine::bootstrap_invite_relay_only(p2p, &relay_multiaddr, &relays, true).await;
    if !reserved {
        return Err(api_err(inertia_core::CoreError::Invite(
            "relay circuit slot not ready - stay on this screen with Relay OK, then try again"
                .into(),
        )));
    }

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
    let input = body.invite;
    let invite = FriendInvite::parse(&input).map_err(api_err)?;

    let (p2p, mut relays) = {
        let engine = state.engine.lock().await;
        engine
            .plan_invite_accept_bootstrap()
            .await
            .map_err(api_err)?
    };

    if !relays
        .iter()
        .any(|relay| relay.trim() == invite.relay_multiaddr.trim())
    {
        relays.push(invite.relay_multiaddr.clone());
    }

    let _ = Engine::bootstrap_invite_relay_only(p2p, &invite.relay_multiaddr, &relays, false).await;

    let engine = state.engine.lock().await;
    engine.accept_invite(&input).await.map(Json).map_err(api_err)
}
