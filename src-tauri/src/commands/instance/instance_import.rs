use crate::services::{InstanceDto, InstanceImportPlan};
use std::path::Path;

#[tauri::command]
pub async fn detect_instance_zip(path: String) -> Result<InstanceImportPlan, String> {
    tracing::info!("Detectando instancia en archivo ZIP: {}", path);
    crate::services::detect_instance_zip(Path::new(&path))
        .await
        .map_err(|e| {
            tracing::error!("Error detectando instancia en ZIP: {}", e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn import_instance_zip(path: String, name: String) -> Result<InstanceDto, String> {
    tracing::info!("Importando instancia desde ZIP '{}' como '{}'", path, name);
    crate::services::import_instance_zip(Path::new(&path), &name)
        .await
        .map_err(|e| {
            tracing::error!("Error importando instancia desde ZIP: {}", e);
            e.to_string()
        })
}
