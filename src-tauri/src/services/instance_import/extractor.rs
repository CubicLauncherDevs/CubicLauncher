//! Extracción y normalización de archivos ZIP de instancias externas.

use super::types::ImportError;
use std::fs::File;
use std::path::{Path, PathBuf};
use tracing::info;
use zip::read::ZipArchive;

/// Extrae un ZIP a un directorio temporal, normalizando la raíz si el ZIP
/// contiene una única carpeta de primer nivel.
pub fn extract_instance_archive(archive_path: &Path) -> Result<PathBuf, ImportError> {
    let file = File::open(archive_path)
        .map_err(|e| ImportError::InvalidArchive(format!("No se pudo abrir el archivo: {e}")))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| ImportError::InvalidArchive(format!("No es un archivo ZIP válido: {e}")))?;

    let temporary = tempfile::Builder::new()
        .prefix("cubic_instance_import_")
        .tempdir()?;
    zellkern::path_security::extract_zip(&mut archive, temporary.path())
        .map_err(|e| ImportError::ExtractionFailed(e.to_string()))?;
    let preview_dir = normalize_root(temporary.path())?;
    info!("ZIP extraído a {:?}", temporary.path());
    // Ownership is transferred to the preview session only after extraction.
    let _ = temporary.keep();
    Ok(preview_dir)
}

/// Si el directorio extraído contiene exactamente una subcarpeta (y nada más),
/// asume que esa subcarpeta es la raíz real de la instancia.
fn normalize_root(temp_dir: &Path) -> Result<PathBuf, ImportError> {
    let entries: Vec<_> = std::fs::read_dir(temp_dir)
        .map_err(|e| ImportError::ExtractionFailed(e.to_string()))?
        .filter_map(|e| e.ok())
        .collect();

    let folders: Vec<_> = entries.iter().filter(|e| e.path().is_dir()).collect();

    if folders.len() == 1 && entries.len() == 1 {
        return Ok(folders[0].path());
    }

    Ok(temp_dir.to_path_buf())
}
