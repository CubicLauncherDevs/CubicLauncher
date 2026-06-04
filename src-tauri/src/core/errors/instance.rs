use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("Instancia no encontrada")]
    NotFound,

    #[error("Mod no encontrado")]
    ModNotFound,

    #[error("La instancia ya ha sido lanzada")]
    AlreadyStarted,

    #[error("La instancia ya existe")]
    AlreadyExists,

    #[error("Archivo de registro no encontrado")]
    LogNotFound,

    #[error("Error al procesar el nombre del archivo")]
    FilenameParse,

    #[error("Error al procesar el nombre de la instancia: {0}")]
    InstNameParse(String),

    #[error("Ruta de origen inválida")]
    InvalidSourcePath,

    #[error("No hay ningun runtime de Java seteado, se necesita Java {0}.")]
    JreDoesntExists(String),

    #[error(transparent)]
    Fs(#[from] crate::core::errors::fs::FsError),
}
