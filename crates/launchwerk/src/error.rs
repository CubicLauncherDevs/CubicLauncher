use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Zellkern(#[from] zellkern::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[cfg(feature = "auth")]
    #[error("Auth error: {0}")]
    AuthError(String),

    #[cfg(feature = "auth")]
    #[error("HTTP request error: {0}")]
    Reqwest(#[from] reqwest::Error),
}
