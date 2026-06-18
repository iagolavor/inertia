use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use inertia_core::Contact;

use crate::dto::AddContactRequest;
use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/contacts", get(list_contacts).post(add_contact))
}

async fn list_contacts(
    State(state): State<AppState>,
) -> Result<Json<Vec<Contact>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.list_contacts().await.map(Json).map_err(api_err)
}

async fn add_contact(
    State(state): State<AppState>,
    Json(body): Json<AddContactRequest>,
) -> Result<Json<Contact>, (StatusCode, Json<ApiError>)> {
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
