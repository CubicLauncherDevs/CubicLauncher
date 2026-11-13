//! Inventario y purga del directorio de caché del launcher
//! (`.cubic/cache`): manifiestos de versiones, idiomas e iconos del market.
//! Nada de lo que hay adentro es un dato del usuario: se regenera o se
//! re-descarga solo.

use crate::core::PathManager;
use crate::services::curseforge_api;
use crate::services::shared_storage::{DirUsage, clear_dir, compute_usage};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

/// Estado de la caché para la UI de ajustes.
#[derive(Clone, serde::Serialize)]
pub struct CacheInfo {
    pub dir: String,
    /// Tamaño total de la caché.
    pub total: DirUsage,
    /// Desglose por subcarpeta, ordenado por nombre.
    pub breakdown: Vec<(String, DirUsage)>,
}

/// Uso de un directorio que puede no existir todavía.
fn dir_usage(dir: &Path) -> DirUsage {
    let mut usage = DirUsage::default();
    if dir.exists() {
        compute_usage(dir, &mut usage);
    }
    usage
}

fn breakdown(root: &Path) -> (DirUsage, Vec<(String, DirUsage)>) {
    let mut total = DirUsage::default();
    let mut items: Vec<(String, DirUsage)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let usage = dir_usage(&entry.path());
            total.bytes += usage.bytes;
            total.files += usage.files;
            total.blocked |= usage.blocked;
            items.push((name, usage));
        }
    }
    items.sort_by(|a, b| a.0.cmp(&b.0));
    (total, items)
}

pub async fn get_cache_info() -> CacheInfo {
    let root = PathManager::get().get_cache_dir().to_path_buf();
    let (total, breakdown) = tokio::task::spawn_blocking(move || breakdown(&root))
        .await
        .unwrap_or_default();
    CacheInfo {
        dir: PathManager::get()
            .get_cache_dir()
            .to_string_lossy()
            .to_string(),
        total,
        breakdown,
    }
}

fn clear_cache_at(root: PathBuf) -> Result<DirUsage, String> {
    let mut total = dir_usage(&root);
    if total.blocked {
        return Err(
            "La caché contiene archivos inaccesibles: bórralos a mano desde la carpeta".to_string(),
        );
    }
    let removed = if root.exists() {
        clear_dir(&root).map_err(|e| format!("No se pudo leer la caché: {e}"))?
    } else {
        Vec::new()
    };
    for name in &removed {
        error!("Entrada no borrada durante la purga de caché: {name}");
    }
    if !removed.is_empty() {
        return Err(format!(
            "No se pudieron borrar {} entrada(s) (¿archivos en uso?): {}",
            removed.len(),
            removed.join(", ")
        ));
    }
    // Las respuestas de red también se conservan en memoria; sin esto seguirían
    // sirviendo datos previos a la purga hasta que expiren.
    curseforge_api::clear_memory_cache();
    total.blocked = false;
    Ok(total)
}

/// Borra todo el contenido de `.cubic/cache`. Todo se regenera solo.
pub async fn purge_cache() -> Result<DirUsage, String> {
    let root = PathManager::get().get_cache_dir().to_path_buf();
    let result = tokio::task::spawn_blocking(move || clear_cache_at(root))
        .await
        .map_err(|e| format!("La tarea de purga falló: {e}"))?;
    match &result {
        Ok(usage) => info!(
            "Purga de caché completada: {} bytes en {} archivos liberados",
            usage.bytes, usage.files
        ),
        Err(e) => warn!("Purga de caché falló: {e}"),
    }
    result
}

#[cfg(test)]
#[path = "../tests/services/launcher_cache.rs"]
mod tests;
