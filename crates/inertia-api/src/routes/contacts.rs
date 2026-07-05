use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use inertia_core::Contact;

use crate::dto::AddContactRequest;
use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/contacts", get(list_contacts).post(add_contact))
        .route("/contacts/:contact_id", get(get_contact))
        .route("/contacts/:contact_id/messages", get(list_conversation_messages))
}

async fn get_contact(
    State(state): State<AppState>,
    Path(contact_id): Path<String>,
) -> Result<Json<Contact>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine.get_contact(&contact_id).await.map(Json).map_err(api_err)
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

async fn list_conversation_messages(
    State(state): State<AppState>,
    Path(contact_id): Path<String>,
) -> Result<Json<Vec<inertia_core::ConversationMessage>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .list_conversation_messages(&contact_id)
        .await
        .map(Json)
        .map_err(api_err)
}
