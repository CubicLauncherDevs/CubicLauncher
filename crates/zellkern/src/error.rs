use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Manifest has no main class")]
    MainClassNotFound,

    #[error("Classpath is empty")]
    EmptyClasspath,

    #[error("Java binary not found: {0}")]
    JavaNotFound(String),

    #[error("Missing file: {0}")]
    MissingFile(String),

    #[error("Version file could not be loaded: {0}")]
    VersionLoad(String),

    #[error("{0}")]
    Custom(String),
}
