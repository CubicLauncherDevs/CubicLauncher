use crate::core::errors::{CoreError, InstanceError};
use crate::services::{InstanceManager, options_profiles};
use tauri::command;
use tracing::info;

use super::instance::validate_uuid;

#[command]
pub fn list_options_profiles() -> Vec<options_profiles::OptionsProfileDto> {
    options_profiles::list()
}

#[command]
pub fn create_options_profile(
    name: String,
    content: String,
) -> Result<options_profiles::OptionsProfileDto, String> {
    info!("Creando perfil de options.txt '{}'", name);
    options_profiles::create(&name, &content).map_err(|e| e.to_json())
}

#[command]
pub fn update_options_profile(
    id: String,
    name: String,
    content: String,
) -> Result<options_profiles::OptionsProfileDto, String> {
    info!("Actualizando perfil de options.txt '{}'", id);
    options_profiles::update(&id, &name, &content).map_err(|e| e.to_json())
}

#[command]
pub fn delete_options_profile(id: String) -> Result<(), String> {
    info!("Eliminando perfil de options.txt '{}'", id);
    options_profiles::delete(&id).map_err(|e| e.to_json())
}

/// Importa un `options.txt` desde un archivo elegido por el usuario.
/// Si `name` está vacío se usa el nombre del archivo.
#[command]
pub fn import_options_profile_from_file(
    path: String,
    name: String,
) -> Result<options_profiles::OptionsProfileDto, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| CoreError::Other(format!("No se pudo leer el archivo: {e}")).to_string())?;
    let name = if name.trim().is_empty() {
        std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("options")
            .to_string()
    } else {
        name
    };
    info!("Importando perfil de options.txt desde '{}'", path);
    options_profiles::create(&name, &content).map_err(|e| e.to_json())
}

/// Guarda el `options.txt` actual de una instancia como un perfil nuevo.
#[command]
pub async fn create_options_profile_from_instance(
    instance_id: String,
    name: String,
) -> Result<options_profiles::OptionsProfileDto, String> {
    validate_uuid(&instance_id)?;
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&instance_id).await else {
        return Err(InstanceError::NotFound.to_string());
    };
    if handle.is_busy() {
        return Err(InstanceError::Busy.to_string());
    }
    let instance_dir = handle.get_instance_dir().await;
    let content = options_profiles::read_instance_options(&instance_dir).ok_or_else(|| {
        CoreError::Other("La instancia no tiene un options.txt todavía".into()).to_string()
    })?;
    options_profiles::create(&name, &content).map_err(|e| e.to_json())
}
