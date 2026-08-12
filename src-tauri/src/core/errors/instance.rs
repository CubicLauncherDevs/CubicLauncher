use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("Instancia no encontrada")]
    NotFound,

    #[error("Versión no encontrada: {0}")]
    VersionNotFound(String),

    #[error("Mod no encontrado")]
    ModNotFound,

    #[error("La instancia ya ha sido lanzada")]
    AlreadyStarted,

    #[error("La instancia ya existe")]
    AlreadyExists,

    #[error("No se puede modificar una instancia mientras está en ejecución")]
    Busy,

    #[error("Archivo de registro no encontrado")]
    LogNotFound,

    #[error("Error al procesar el nombre del archivo")]
    FilenameParse,

    #[error("Error al procesar el nombre de la instancia: {0}")]
    InstNameParse(String),

    #[error("Ruta de origen inválida")]
    InvalidSourcePath,

    #[error("No hay ningun runtime de Java seteado, se necesita Java {0}.")]
    JreNotFound(String),

    #[error("Formato de instancia no reconocido en el archivo proporcionado")]
    ImportInstanceFormatUnknown,

    #[error("Archivo de instancia inválido o corrupto: {0}")]
    ImportInstanceArchiveInvalid(String),

    #[error("No se pudo extraer el archivo de instancia: {0}")]
    ImportInstanceExtractionFailed(String),

    #[error("No se pudo determinar la versión de Minecraft")]
    ImportInstanceUnknownMinecraftVersion,

    #[error("Loader no soportado en la instancia: {0}")]
    ImportInstanceUnsupportedLoader(String),

    #[error("Error del provider de importación {provider}: {message}")]
    ImportInstanceProviderError { provider: String, message: String },

    #[error(transparent)]
    Fs(#[from] crate::core::errors::fs::FsError),
}

impl InstanceError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "INST_NOT_FOUND",
            Self::VersionNotFound(_) => "INST_VERSION_NOT_FOUND",
            Self::ModNotFound => "INST_MOD_NOT_FOUND",
            Self::AlreadyStarted => "INST_ALREADY_STARTED",
            Self::AlreadyExists => "INST_EXISTS",
            Self::Busy => "INST_BUSY",
            Self::LogNotFound => "INST_LOG_NOT_FOUND",
            Self::FilenameParse => "INST_FILENAME_PARSE",
            Self::InstNameParse(_) => "INST_NAME_INVALID",
            Self::InvalidSourcePath => "INST_SRC_INVALID",
            Self::JreNotFound(_) => "INST_JRE_MISSING",
            Self::ImportInstanceFormatUnknown => "INST_IMPORT_FORMAT_UNKNOWN",
            Self::ImportInstanceArchiveInvalid(_) => "INST_IMPORT_ARCHIVE_INVALID",
            Self::ImportInstanceExtractionFailed(_) => "INST_IMPORT_EXTRACTION_FAILED",
            Self::ImportInstanceUnknownMinecraftVersion => "INST_IMPORT_UNKNOWN_MINECRAFT_VERSION",
            Self::ImportInstanceUnsupportedLoader(_) => "INST_IMPORT_UNSUPPORTED_LOADER",
            Self::ImportInstanceProviderError { .. } => "INST_IMPORT_PROVIDER_ERROR",
            Self::Fs(e) => e.code(),
        }
    }

    pub fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::VersionNotFound(v) => vec![("version", v.clone())],
            Self::InstNameParse(s) => vec![("error", s.clone())],
            Self::JreNotFound(v) => vec![("version", v.clone())],
            Self::ImportInstanceArchiveInvalid(d) => vec![("error", d.clone())],
            Self::ImportInstanceExtractionFailed(d) => vec![("error", d.clone())],
            Self::ImportInstanceUnsupportedLoader(l) => vec![("loader", l.clone())],
            Self::ImportInstanceProviderError { provider, message } => {
                vec![("provider", provider.clone()), ("error", message.clone())]
            }
            Self::Fs(e) => e.params(),
            _ => vec![],
        }
    }
}
