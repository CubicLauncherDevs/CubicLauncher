//! Tipos compartidos del sistema genérico de importación de instancias.

use serde::Serialize;
use std::path::PathBuf;

/// Información de preview de un archivo de instancia detectado.
#[derive(Debug, Clone, Serialize)]
pub struct InstanceImportPlan {
    pub format_id: &'static str,
    pub format_name: &'static str,
    pub archive_path: PathBuf,
    pub preview_dir: PathBuf,
    pub original_name: String,
    pub sanitized_name: String,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub warnings: Vec<String>,
}

/// Errores específicos del sistema de importación de instancias.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("Formato de instancia no reconocido")]
    FormatUnknown,

    #[error("Archivo .zip inválido o corrupto: {0}")]
    InvalidArchive(String),

    #[error("No se pudo extraer el archivo: {0}")]
    ExtractionFailed(String),

    #[error("No se pudo determinar la versión de Minecraft")]
    UnknownMinecraftVersion,

    #[error("Loader no soportado: {0}")]
    UnsupportedLoader(String),

    #[error("Error del provider {provider}: {message}")]
    ProviderError { provider: String, message: String },

    #[error(transparent)]
    Instance(#[from] crate::core::errors::instance::InstanceError),

    #[error(transparent)]
    Fs(#[from] crate::core::errors::fs::FsError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<ImportError> for String {
    fn from(e: ImportError) -> Self {
        crate::core::errors::instance::InstanceError::from(e).to_string()
    }
}

impl From<ImportError> for crate::core::errors::instance::InstanceError {
    fn from(e: ImportError) -> Self {
        match e {
            ImportError::FormatUnknown => Self::ImportInstanceFormatUnknown,
            ImportError::InvalidArchive(msg) => Self::ImportInstanceArchiveInvalid(msg),
            ImportError::ExtractionFailed(msg) => Self::ImportInstanceExtractionFailed(msg),
            ImportError::UnknownMinecraftVersion => Self::ImportInstanceUnknownMinecraftVersion,
            ImportError::UnsupportedLoader(l) => Self::ImportInstanceUnsupportedLoader(l),
            ImportError::ProviderError { provider, message } => {
                Self::ImportInstanceProviderError { provider, message }
            }
            ImportError::Instance(inner) => inner,
            ImportError::Fs(inner) => Self::Fs(inner),
            ImportError::Io(e) => Self::Fs(crate::core::errors::fs::FsError::ReadFile {
                path: "<unknown>".into(),
                source: e,
            }),
        }
    }
}

/// Convierte un nombre arbitrario al formato restringido de CubicLauncher.
pub fn sanitize_instance_name(name: &str) -> String {
    use crate::services::instance_manager::data::MAX_LEN;

    let mut clean: String = name
        .chars()
        .filter(|c| {
            c.is_ascii()
                && !matches!(
                    c,
                    '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\0'
                )
        })
        .collect();

    clean = clean.replace("..", "").trim().to_string();

    if clean.is_empty() {
        clean = "Imported".to_string();
    }

    let max_len = MAX_LEN as usize;
    if clean.len() > max_len {
        clean.truncate(max_len);
        clean = clean.trim_end().to_string();
    }

    clean
}
