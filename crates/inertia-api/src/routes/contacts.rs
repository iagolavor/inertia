use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use inertia_core::{
    ArchiveEntry, ArchiveFolderSummary, Contact, ProfileComment, ProfileManifest,
};

use crate::dto::{AddCommentRequest, AddContactRequest};
use crate::error::{api_err, ApiError};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/contacts", get(list_contacts).post(add_contact))
        .route("/contacts/:contact_id", get(get_contact))
        .route("/contacts/:contact_id/messages", get(list_conversation_messages))
        .route("/contacts/:contact_id/profile", get(fetch_friend_profile))
        .route(
            "/contacts/:contact_id/profile/items/:item_id/comments",
            get(list_friend_profile_comments).post(add_friend_profile_comment),
        )
        .route(
            "/contacts/:contact_id/archive/folders/:folder_id",
            get(fetch_friend_archive_folder),
        )
        .route(
            "/contacts/:contact_id/blobs/:hash/fetch",
            axum::routing::post(fetch_friend_blob),
        )
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

async fn fetch_friend_profile(
    State(state): State<AppState>,
    Path(contact_id): Path<String>,
) -> Result<Json<ProfileManifest>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .fetch_friend_profile(&contact_id)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn list_friend_profile_comments(
    State(state): State<AppState>,
    Path((contact_id, item_id)): Path<(String, String)>,
) -> Result<Json<Vec<ProfileComment>>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .fetch_profile_item_comments(Some(&contact_id), &item_id)
        .await
        .map(Json)
        .map_err(api_err)
}

async fn add_friend_profile_comment(
    State(state): State<AppState>,
    Path((contact_id, item_id)): Path<(String, String)>,
    Json(body): Json<AddCommentRequest>,
) -> Result<Json<ProfileComment>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .add_profile_comment(&item_id, &body.body, Some(&contact_id))
        .await
        .map(Json)
        .map_err(api_err)
}

async fn fetch_friend_archive_folder(
    State(state): State<AppState>,
    Path((contact_id, folder_id)): Path<(String, String)>,
) -> Result<Json<FriendArchiveFolderResponse>, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    let (folder, entries) = engine
        .fetch_friend_archive_folder(&contact_id, &folder_id)
        .await
        .map_err(api_err)?;
    Ok(Json(FriendArchiveFolderResponse { folder, entries }))
}

async fn fetch_friend_blob(
    State(state): State<AppState>,
    Path((contact_id, hash)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let engine = state.engine.lock().await;
    engine
        .fetch_blob_from_contact(&contact_id, &hash)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(api_err)
}

#[derive(serde::Serialize)]
struct FriendArchiveFolderResponse {
    folder: ArchiveFolderSummary,
    entries: Vec<ArchiveEntry>,
}
