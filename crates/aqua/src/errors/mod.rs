use thiserror::Error;

#[derive(Error, Debug)]
pub enum AquaError {
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

    #[error("Forge installer download failed: {0}")]
    ForgeInstallerDownload(String),

    #[error("Forge installer extraction failed: {0}")]
    ForgeExtract(String),

    #[error("Forge profile parse failed: {0}")]
    ForgeProfileParse(String),

    #[error("Forge processor '{processor}' failed: {detail}")]
    ForgeProcessor { processor: String, detail: String },

    #[error("Forge output verification failed: {file} (expected {expected}, got {actual})")]
    ForgeOutputVerification {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("{0}")]
    Other(String),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AquaError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        AquaError::Other(err.to_string())
    }
}

impl From<zellkern::Error> for AquaError {
    fn from(err: zellkern::Error) -> Self {
        match err {
            zellkern::Error::Io(e) => AquaError::IoError(e),
            zellkern::Error::Json(e) => AquaError::JsonError(e),
            zellkern::Error::MainClassNotFound => AquaError::MainClassNotFound,
            zellkern::Error::EmptyClasspath => AquaError::EmptyClasspath,
            zellkern::Error::JavaNotFound(p) => AquaError::JavaNotFound(p),
            zellkern::Error::MissingFile(p) => AquaError::MissingFile(p),
            zellkern::Error::VersionLoad(v) => AquaError::VersionNotFound(v),
            zellkern::Error::Custom(s) => AquaError::Other(s),
        }
    }
}
