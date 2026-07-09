use axum::http::StatusCode;
use axum::Json;
use inertia_core::{CoreError, ErrorCode};
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

pub fn api_err(e: CoreError) -> (StatusCode, Json<ApiError>) {
    let facing = e.user_facing();
    let status = status_for_error(&e, facing.code);
    (
        status,
        Json(ApiError {
            error: facing.message,
            code: facing.code.map(|c| c.as_str().to_string()),
        }),
    )
}

fn status_for_error(e: &CoreError, code: Option<ErrorCode>) -> StatusCode {
    match code {
        Some(ErrorCode::P2pNotStarted) | Some(ErrorCode::RelayUnreachable) => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        Some(ErrorCode::InviterOffline) | Some(ErrorCode::FriendOffline) => {
            StatusCode::GATEWAY_TIMEOUT
        }
        None => match e {
            CoreError::Invite(_) => StatusCode::BAD_REQUEST,
            CoreError::ProfileAlreadyExists => StatusCode::CONFLICT,
            CoreError::ContactNotFound(_) | CoreError::ContentNotFound(_) => StatusCode::NOT_FOUND,
            CoreError::IdentityNotInitialized => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
    }
}
