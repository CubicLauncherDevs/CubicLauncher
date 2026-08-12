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

    let temp_dir = std::env::temp_dir().join(format!(
        "cubic_instance_import_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        ImportError::ExtractionFailed(format!(
            "No se pudo crear directorio temporal {:?}: {e}",
            temp_dir
        ))
    })?;

    archive
        .extract(&temp_dir)
        .map_err(|e| ImportError::ExtractionFailed(e.to_string()))?;

    info!("ZIP extraído a {:?}", temp_dir);

    let preview_dir = normalize_root(&temp_dir)?;
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
