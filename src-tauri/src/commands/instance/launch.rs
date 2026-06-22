use crate::core::errors::{AppError, FsError, InstanceError};
use crate::services::{InstanceDto, InstanceManager, InstanceStatus, Launcher, signal_kill};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

pub fn validate_uuid(uuid: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(uuid).map_err(|_| format!("UUID inválido: '{}'", uuid))?;
    Ok(())
}

pub fn sanitize_sub_path(instance_dir: &Path, sub_path: &Path) -> Result<PathBuf, String> {
    if sub_path.is_absolute() {
        return Err(FsError::InvalidPath(sub_path.to_string_lossy().to_string()).to_string());
    }
    for component in sub_path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(FsError::InvalidPath(sub_path.to_string_lossy().to_string()).to_string());
        }
    }
    Ok(instance_dir.join(sub_path))
}

#[tauri::command]
pub async fn launch(instance_id: String) -> Result<(), String> {
    validate_uuid(&instance_id)?;
    info!("Lanzando instancia {}", instance_id);
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&instance_id).await else {
        error!("Instancia {} no encontrada para lanzar", instance_id);
        return Err(AppError::Instance(InstanceError::NotFound).to_json());
    };

    Launcher::get().launch(handle.clone()).await.map_err(|e| {
        error!("Error lanzando instancia {}: {}", instance_id, e);
        e.to_json()
    })?;

    info!("Instancia {} lanzada exitosamente", instance_id);
    Ok(())
}

#[tauri::command]
pub async fn kill_instance(uuid: String) -> Result<(), String> {
    validate_uuid(&uuid)?;
    info!("Matando instancia {}", uuid);
    let manager = InstanceManager::get();
    let handle = manager
        .get_handle(&uuid)
        .await
        .ok_or_else(|| InstanceError::NotFound.to_string())?;

    if !signal_kill(&uuid) {
        warn!("Instancia {} no estaba corriendo, forzando Off", uuid);
        handle.set_status(InstanceStatus::Off);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_instances() -> Vec<InstanceDto> {
    InstanceManager::get().get_all_dtos().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_uuid_valid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
    }

    #[test]
    fn test_validate_uuid_invalid() {
        assert!(validate_uuid("not-a-uuid").is_err());
    }

    #[test]
    fn test_validate_uuid_empty() {
        assert!(validate_uuid("").is_err());
    }

    #[test]
    fn test_sanitize_absolute_path() {
        let base = Path::new("/tmp/instances/test");
        let result = sanitize_sub_path(base, Path::new("/etc/passwd"));
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_parent_dir() {
        let base = Path::new("/tmp/instances/test");
        let result = sanitize_sub_path(base, Path::new("../malicious"));
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_dotdot_nested() {
        let base = Path::new("/tmp/instances/test");
        let result = sanitize_sub_path(base, Path::new("mods/../../secrets"));
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_valid_sub_path() {
        let base = PathBuf::from("/tmp/instances/test");
        let result = sanitize_sub_path(&base, Path::new("mods"));
        assert_eq!(result.unwrap(), base.join("mods"));
    }

    #[test]
    fn test_sanitize_nested_valid() {
        let base = PathBuf::from("/tmp/instances/test");
        let result = sanitize_sub_path(&base, Path::new("screenshots/2025-01-01"));
        assert_eq!(result.unwrap(), base.join("screenshots/2025-01-01"));
    }
}
