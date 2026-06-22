use thiserror::Error;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("No se pudo leer el directorio '{path}': {source}")]
    ReadDir {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("No se pudo leer el archivo '{path}': {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("No se pudo crear el directorio '{path}': {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("No se pudo escribir el archivo '{path}': {source}")]
    WriteFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("No se pudo copiar '{from}' a '{to}': {source}")]
    Copy {
        from: String,
        to: String,
        #[source]
        source: std::io::Error,
    },

    #[error("No se pudo renombrar '{from}' a '{to}': {source}")]
    Rename {
        from: String,
        to: String,
        #[source]
        source: std::io::Error,
    },

    #[error("No se pudo eliminar '{path}': {source}")]
    Remove {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Archivo no encontrado: '{0}'")]
    NotFound(String),

    #[error("Ruta inválida: '{0}'")]
    InvalidPath(String),
}

impl FsError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ReadDir { .. } => "FS_READ_DIR",
            Self::ReadFile { .. } => "FS_READ_FILE",
            Self::CreateDir { .. } => "FS_CREATE_DIR",
            Self::WriteFile { .. } => "FS_WRITE_FILE",
            Self::Copy { .. } => "FS_COPY",
            Self::Rename { .. } => "FS_RENAME",
            Self::Remove { .. } => "FS_REMOVE",
            Self::NotFound(_) => "FS_NOT_FOUND",
            Self::InvalidPath(_) => "FS_INVALID_PATH",
        }
    }

    pub fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::ReadDir { path, source } => {
                vec![("path", path.clone()), ("error", source.to_string())]
            }
            Self::ReadFile { path, source } => {
                vec![("path", path.clone()), ("error", source.to_string())]
            }
            Self::CreateDir { path, source } => {
                vec![("path", path.clone()), ("error", source.to_string())]
            }
            Self::WriteFile { path, source } => {
                vec![("path", path.clone()), ("error", source.to_string())]
            }
            Self::Copy { from, to, source } => vec![
                ("from", from.clone()),
                ("to", to.clone()),
                ("error", source.to_string()),
            ],
            Self::Rename { from, to, source } => vec![
                ("from", from.clone()),
                ("to", to.clone()),
                ("error", source.to_string()),
            ],
            Self::Remove { path, source } => {
                vec![("path", path.clone()), ("error", source.to_string())]
            }
            Self::NotFound(p) => vec![("path", p.clone())],
            Self::InvalidPath(p) => vec![("path", p.clone())],
        }
    }
}
