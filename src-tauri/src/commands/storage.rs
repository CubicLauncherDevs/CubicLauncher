use crate::core::default_instances_dir;
use crate::services::storage_migration::{self, InstancesDirInfo, MoveProgress, MoveResult};
use std::path::PathBuf;
use tauri::ipc::Channel;
use tracing::{error, info};

/// Estado del directorio de instancias para la UI de ajustes.
#[tauri::command]
pub async fn get_instances_dir_info() -> Result<InstancesDirInfo, String> {
    Ok(storage_migration::get_instances_dir_info().await)
}

/// Cambia el directorio de instancias moviendo las existentes (hardlinks).
#[tauri::command]
pub async fn change_instances_dir(
    new_dir: String,
    on_progress: Channel<MoveProgress>,
) -> Result<MoveResult, String> {
    info!("Cambiando directorio de instancias a {}", new_dir);
    let result = storage_migration::change_instances_dir(PathBuf::from(new_dir), on_progress).await;
    if let Err(ref e) = result {
        error!("Error cambiando directorio de instancias: {e}");
    }
    result
}

/// Cancela una migración de directorio en curso.
#[tauri::command]
pub fn cancel_instances_dir_change() {
    storage_migration::cancel_instances_dir_change();
}

/// Restablece el directorio de instancias al predeterminado moviendo las existentes.
#[tauri::command]
pub async fn reset_instances_dir(on_progress: Channel<MoveProgress>) -> Result<MoveResult, String> {
    info!("Restableciendo directorio de instancias al predeterminado");
    let result =
        storage_migration::change_instances_dir(default_instances_dir(), on_progress).await;
    if let Err(ref e) = result {
        error!("Error restableciendo directorio de instancias: {e}");
    }
    result
}
