use crate::core::{PathManager, default_instances_dir};
use crate::services::shared_storage::{self, DirUsage, SharedDirInfo};
use crate::services::storage_migration::{self, InstancesDirInfo, MoveProgress, MoveResult};
use std::path::PathBuf;
use tauri::Manager;
use tauri::ipc::Channel;
use tracing::{error, info};

/// El asset protocol necesita permitir la carpeta activa, que puede estar
/// fuera de `$HOME/.cubic` tras una migración o al reiniciar el launcher.
pub(crate) fn allow_current_instances_asset_scope(app: &tauri::AppHandle) -> Result<(), String> {
    let dir = PathManager::get().get_instance_dir();
    let dir = dir
        .canonicalize()
        .map_err(|e| format!("No se pudo resolver {}: {e}", dir.display()))?;
    app.asset_protocol_scope()
        .allow_directory(&dir, true)
        .map_err(|e| format!("No se pudo permitir el acceso a {}: {e}", dir.display()))
}

/// Estado del directorio de instancias para la UI de ajustes.
#[tauri::command]
pub async fn get_instances_dir_info() -> Result<InstancesDirInfo, String> {
    Ok(storage_migration::get_instances_dir_info().await)
}

/// Cambia el directorio de instancias moviendo las existentes (hardlinks).
#[tauri::command]
pub async fn change_instances_dir(
    app: tauri::AppHandle,
    new_dir: String,
    on_progress: Channel<MoveProgress>,
) -> Result<MoveResult, String> {
    info!("Cambiando directorio de instancias a {}", new_dir);
    let result = storage_migration::change_instances_dir(PathBuf::from(new_dir), on_progress).await;
    if let Err(ref e) = result {
        error!("Error cambiando directorio de instancias: {e}");
    }
    if let Err(scope_error) = allow_current_instances_asset_scope(&app) {
        error!("Error configurando el asset protocol: {scope_error}");
        if result.is_ok() {
            return Err(scope_error);
        }
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
pub async fn reset_instances_dir(
    app: tauri::AppHandle,
    on_progress: Channel<MoveProgress>,
) -> Result<MoveResult, String> {
    info!("Restableciendo directorio de instancias al predeterminado");
    let result =
        storage_migration::change_instances_dir(default_instances_dir(), on_progress).await;
    if let Err(ref e) = result {
        error!("Error restableciendo directorio de instancias: {e}");
    }
    if let Err(scope_error) = allow_current_instances_asset_scope(&app) {
        error!("Error configurando el asset protocol: {scope_error}");
        if result.is_ok() {
            return Err(scope_error);
        }
    }
    result
}

/// Estado y uso de disco del directorio shared para la UI de ajustes.
#[tauri::command]
pub async fn get_shared_dir_info() -> Result<SharedDirInfo, String> {
    Ok(shared_storage::get_shared_dir_info().await)
}

/// Borra todo el contenido de shared; se re-descarga al lanzar.
#[tauri::command]
pub async fn purge_shared_dir() -> Result<DirUsage, String> {
    info!("Purgando directorio shared");
    let result = shared_storage::purge_shared_dir().await;
    if let Err(ref e) = result {
        error!("Error purgando directorio shared: {e}");
    }
    result
}

/// Cambia la ruta de shared: borra el contenido del destino y reapunta.
#[tauri::command]
pub async fn change_shared_dir(new_dir: String) -> Result<DirUsage, String> {
    info!("Cambiando directorio shared a {new_dir}");
    let result = shared_storage::change_shared_dir(PathBuf::from(new_dir)).await;
    if let Err(ref e) = result {
        error!("Error cambiando directorio shared: {e}");
    }
    result
}

/// Restablece shared a la ubicación predeterminada purgando el destino.
#[tauri::command]
pub async fn reset_shared_dir() -> Result<DirUsage, String> {
    info!("Restableciendo directorio shared al predeterminado");
    let result = shared_storage::change_shared_dir(crate::core::default_shared_dir()).await;
    if let Err(ref e) = result {
        error!("Error restableciendo directorio shared: {e}");
    }
    result
}
