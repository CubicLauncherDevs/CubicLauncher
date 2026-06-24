use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Error al obtener el device code: {0}")]
    DeviceCodeFailed(String),

    #[error("Error al autenticar con Microsoft: {0}")]
    AuthFailed(String),

    #[error("Error al guardar los tokens: {0}")]
    SaveTokensFailed(String),

    #[error("Error al eliminar los tokens: {0}")]
    DeleteTokensFailed(String),

    #[error("La tarea bloqueante falló: {0}")]
    SpawnBlocking(String),

    #[error("La autenticación fue cancelada o expiró: {0}")]
    AuthCancelled(String),

    #[error(transparent)]
    CoreError(#[from] crate::core::errors::CoreError),
}

impl AuthError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DeviceCodeFailed(_) => "AUTH_DEVICE_CODE",
            Self::AuthFailed(_) => "AUTH_FAILED",
            Self::SaveTokensFailed(_) => "AUTH_TOKENS_SAVE",
            Self::DeleteTokensFailed(_) => "AUTH_TOKENS_DEL",
            Self::SpawnBlocking(_) => "AUTH_BLOCKED",
            Self::AuthCancelled(_) => "AUTH_CANCELLED",
            Self::CoreError(e) => e.code(),
        }
    }

    pub fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::DeviceCodeFailed(s)
            | Self::AuthFailed(s)
            | Self::SaveTokensFailed(s)
            | Self::DeleteTokensFailed(s)
            | Self::SpawnBlocking(s)
            | Self::AuthCancelled(s) => vec![("error", s.clone())],
            Self::CoreError(e) => e.params(),
        }
    }
}
