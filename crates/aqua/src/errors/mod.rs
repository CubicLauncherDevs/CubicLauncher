use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtonError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Version '{0}' not found in manifest")]
    VersionNotFound(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("SHA-1 hash mismatch (expected {expected}, got {actual})")]
    HashMismatch { expected: String, actual: String },

    #[error("No hash provided for: {0}")]
    MissingHash(String),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Manifest has no main class")]
    MainClassNotFound,

    #[error("Classpath is empty")]
    EmptyClasspath,

    #[error("Java binary not found: {0}")]
    JavaNotFound(String),

    #[error("Missing file: {0}")]
    MissingFile(String),

    #[error("Download cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ProtonError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ProtonError::Other(err.to_string())
    }
}

impl From<zellkern::Error> for ProtonError {
    fn from(err: zellkern::Error) -> Self {
        match err {
            zellkern::Error::Io(e) => ProtonError::IoError(e),
            zellkern::Error::Json(e) => ProtonError::JsonError(e),
            zellkern::Error::MainClassNotFound => ProtonError::MainClassNotFound,
            zellkern::Error::EmptyClasspath => ProtonError::EmptyClasspath,
            zellkern::Error::JavaNotFound(p) => ProtonError::JavaNotFound(p),
            zellkern::Error::MissingFile(p) => ProtonError::MissingFile(p),
            zellkern::Error::VersionLoad(v) => ProtonError::VersionNotFound(v),
            zellkern::Error::Custom(s) => ProtonError::Other(s),
        }
    }
}
