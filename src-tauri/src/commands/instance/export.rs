use super::launch::validate_uuid;
use crate::core::InstanceError;
use crate::services::InstanceManager;
use crate::services::instance_export::{export_to_zip, prepare_export};
use std::path::{Component, PathBuf};
use tracing::{error, info};

#[tauri::command]
pub async fn export_instance_zip(id: String, dest: String) -> Result<String, String> {
    validate_uuid(&id)?;
    info!("Exportando instancia {} a '{}'", id, dest);

    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        error!("Instancia {} no encontrada para exportar", id);
        return Err(InstanceError::NotFound.into());
    };

    if handle.is_busy() {
        error!("Intento de exportar instancia ocupada {}", id);
        return Err(InstanceError::Busy.into());
    }

    let input = prepare_export(&handle)
        .await
        .map_err(|e| format!("Error preparando exportación: {e}"))?;
    let dest_path = PathBuf::from(dest);

    if dest_path.file_name().is_none() {
        return Err("Ruta de destino inválida".into());
    }
    if dest_path.extension().map(|e| e != "zip").unwrap_or(true) {
        return Err("La exportación debe ser un archivo .zip".into());
    }

    // Reject destinations containing parent directory references to avoid overwriting
    // files outside the intended export location.
    for component in dest_path.components() {
        if matches!(component, Component::ParentDir) {
            return Err("Ruta de destino inválida: contiene '..'".into());
        }
    }

    let output = tokio::task::spawn_blocking(move || export_to_zip(&input, &dest_path))
        .await
        .map_err(|e| format!("Tarea de exportación fallida: {e}"))?
        .map_err(|e| format!("Error exportando instancia: {e}"))?;

    let out_str = output.to_string_lossy().to_string();
    info!("Instancia {} exportada a '{}'", id, out_str);
    Ok(out_str)
}
