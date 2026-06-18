use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("identity not initialized")]
    IdentityNotInitialized,
    #[error("a profile already exists on this device")]
    ProfileAlreadyExists,
    #[error("contact not found: {0}")]
    ContactNotFound(String),
    #[error("content not found: {0}")]
    ContentNotFound(String),
    #[error("invite error: {0}")]
    Invite(String),
    #[error("p2p error: {0}")]
    P2p(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type CoreResult<T> = std::result::Result<T, CoreError>;
