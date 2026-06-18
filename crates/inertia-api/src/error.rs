use axum::http::StatusCode;
use axum::Json;
use inertia_core::CoreError;
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

pub fn api_err(e: CoreError) -> (StatusCode, Json<ApiError>) {
    let status = match &e {
        CoreError::Invite(_) => StatusCode::BAD_REQUEST,
        CoreError::ProfileAlreadyExists => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ApiError {
            error: e.to_string(),
        }),
    )
}
