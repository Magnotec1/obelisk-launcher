use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid instance path: {0}")]
    InvalidPath(String),

    #[error("Instance configuration error: {0}")]
    Config(String),

    #[error("Component not found: {0}")]
    ComponentNotFound(String),

    #[error("Security violation: {0}")]
    Security(String),

    #[error("Modloader error: {0}")]
    Modloader(String),

    #[error("General error: {0}")]
    General(String),
}

pub type InstanceResult<T> = Result<T, InstanceError>;
