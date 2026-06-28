use axum::http::StatusCode;
use axum::Json;

use crate::error::ApiError;

pub const MAX_BLOB_BYTES: usize = 2 * 1024 * 1024;

pub fn base64_decode(input: &str) -> Result<Vec<u8>, (StatusCode, Json<ApiError>)> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: format!("invalid base64: {e}"),
                    code: None,
                }),
            )
        })
}

pub fn blob_content_type(data: &[u8]) -> &'static str {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png"
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        "image/gif"
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "image/webp"
    } else if data.len() >= 12 && &data[4..8] == b"ftyp" {
        "video/mp4"
    } else {
        "application/octet-stream"
    }
}

pub fn blob_too_large_err() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::PAYLOAD_TOO_LARGE,
        Json(ApiError {
            error: format!(
                "image exceeds {} MB limit",
                MAX_BLOB_BYTES / (1024 * 1024)
            ),
            code: None,
        }),
    )
}
