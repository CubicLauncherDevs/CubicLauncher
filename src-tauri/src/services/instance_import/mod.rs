//! Sistema genérico de importación de instancias desde archivos ZIP de otros launchers.
//!
//! La arquitectura se basa en un trait `InstanceImporter` y un registro de providers.
//! Para agregar soporte a un nuevo launcher, solo hay que implementar `InstanceImporter`
//! y registrarlo en `IMPORTERS`.

mod extractor;
mod multimc;
pub mod types;

pub use extractor::extract_instance_archive;
pub use types::{ImportError, InstanceImportPlan};

use crate::services::InstanceDto;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use tracing::{info, warn};

/// Trait que debe implementar cada launcher soportado.
pub trait InstanceImporter: Send + Sync {
    /// Identificador interno (snake_case).
    fn id(&self) -> &'static str;

    /// Nombre legible para la UI.
    fn display_name(&self) -> &'static str;

    /// Devuelve `true` si el directorio preview parece ser una instancia de este launcher.
    fn detect(&self, preview_dir: &Path) -> bool;

    /// Produce un plan de importación sin mutar nada externo.
    fn preview(&self, preview_dir: &Path) -> Result<InstanceImportPlan, ImportError>;

    /// Ejecuta la importación real.
    fn import<'a>(
        &'a self,
        preview_dir: &'a Path,
        target_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<InstanceDto, ImportError>> + Send + 'a>>;
}

/// Registro global de providers, ordenado por prioridad.
static IMPORTERS: &[&dyn InstanceImporter] = &[&multimc::MultimcProvider];

/// Detecta el formato de un archivo ZIP y devuelve un plan de importación.
pub async fn detect_instance_zip(archive_path: &Path) -> Result<InstanceImportPlan, ImportError> {
    let preview_dir = extract_instance_archive(archive_path)?;

    for importer in IMPORTERS {
        if importer.detect(&preview_dir) {
            info!(
                "Formato detectado '{}' para {:?}",
                importer.display_name(),
                archive_path
            );
            let mut plan = importer.preview(&preview_dir)?;
            plan.archive_path = archive_path.to_path_buf();
            return Ok(plan);
        }
    }

    warn!(
        "Ningún provider reconoció el archivo ZIP {:?}",
        archive_path
    );
    Err(ImportError::FormatUnknown)
}

/// Importa una instancia desde un archivo ZIP ya previeweado.
pub async fn import_instance_zip(
    archive_path: &Path,
    target_name: &str,
) -> Result<InstanceDto, ImportError> {
    let preview_dir = extract_instance_archive(archive_path)?;

    for importer in IMPORTERS {
        if importer.detect(&preview_dir) {
            return importer.import(&preview_dir, target_name).await;
        }
    }

    Err(ImportError::FormatUnknown)
}
