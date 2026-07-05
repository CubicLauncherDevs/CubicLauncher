use super::launch::sanitize_sub_path;
use super::launch::validate_uuid;
use crate::core::errors::{FsError, InstanceError};
use crate::core::{AppEvent, PathManager, emit};
use crate::services::InstOverrides;
use crate::services::InstanceManager;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

#[tauri::command]
pub async fn create_instance(
    name: String,
    version: String,
    icon: Option<String>,
) -> Result<(), String> {
    info!(
        "Creando instancia: name={}, version={}, icon={:?}",
        name, version, icon
    );
    match InstanceManager::get()
        .create_instance(name, version, icon)
        .await
    {
        Ok(d) => {
            info!("Instancia creada: uuid={}", d.uuid);
            emit(AppEvent::InstanceCreated {
                id: d.uuid.to_string().into(),
                dto: d.to_dto().await,
            });
            Ok(())
        }
        Err(e) => {
            error!("Error creando instancia: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn delete_instance(id: String) -> Result<(), String> {
    validate_uuid(&id)?;
    info!("Eliminando instancia {}", id);
    let result = InstanceManager::get().delete_instance(&id).await;
    if let Err(ref e) = result {
        error!("Error eliminando instancia {}: {}", id, e);
    } else {
        info!("Instancia {} eliminada", id);
    }
    result
}

#[tauri::command]
pub async fn open_instance_dir(id: String, sub_dir: Option<String>) -> Result<(), String> {
    validate_uuid(&id)?;
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        warn!(
            "Intento de abrir directorio de instancia {} no encontrada",
            id
        );
        return Err(InstanceError::NotFound.to_string());
    };

    let instance_dir = handle.get_instance_dir().await;

    let path = match sub_dir {
        Some(ref sub) => sanitize_sub_path(&instance_dir, Path::new(sub))?,
        None => instance_dir,
    };

    info!("Abriendo directorio: {:?}", path);

    if !path.exists()
        && let Err(e) = tokio::fs::create_dir_all(&path).await
    {
        error!("No se pudo crear el directorio {:?}: {}", path, e);
        return Err(FsError::CreateDir {
            path: path.to_string_lossy().to_string(),
            source: e,
        }
        .to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn rename_instance(id: String, new_name: String) -> Result<(), String> {
    validate_uuid(&id)?;
    info!("Renombrando instancia {} a '{}'", id, new_name);
    let result = InstanceManager::get()
        .update_instance(&id, Some(new_name), None, None, None)
        .await;
    if let Err(ref e) = result {
        error!("Error renombrando instancia {}: {}", id, e);
    }
    result
}

#[tauri::command]
pub async fn update_instance(
    id: String,
    new_name: Option<String>,
    new_version: Option<String>,
    new_icon: Option<Option<String>>,
    new_overrides: Option<InstOverrides>,
) -> Result<(), String> {
    validate_uuid(&id)?;
    info!(
        "Actualizando instancia {}: name={:?}, version={:?}, icon={:?}",
        id, new_name, new_version, new_icon
    );
    let result = InstanceManager::get()
        .update_instance(&id, new_name, new_version, new_icon, new_overrides)
        .await;
    if let Err(ref e) = result {
        error!("Error actualizando instancia {}: {}", id, e);
    }
    result
}

#[tauri::command]
pub async fn get_installed_versions() -> Vec<String> {
    let versions_dir = PathManager::get().get_shared_dir().join("versions");
    let mut versions = tokio::task::spawn_blocking(move || -> Vec<String> {
        match std::fs::read_dir(&versions_dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect(),
            Err(_) => Vec::new(),
        }
    })
    .await
    .unwrap_or_default();
    versions.sort_by(|a, b| b.cmp(a));
    versions
}

#[tauri::command]
pub async fn delete_instance_file(
    id: String,
    sub_dir: String,
    filename: String,
) -> Result<(), String> {
    validate_uuid(&id)?;
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        warn!("Instancia {} no encontrada para eliminar archivo", id);
        return Err(InstanceError::NotFound.to_string());
    };

    let instance_dir = handle.get_instance_dir().await;
    let sub_path = sanitize_sub_path(&instance_dir, Path::new(&sub_dir))?;
    let file_path = sub_path.join(&filename);
    info!("Eliminando archivo {:?} de instancia {}", file_path, id);
    if file_path.exists() {
        tokio::fs::remove_file(&file_path).await.map_err(|e| {
            error!("Error eliminando archivo {:?}: {}", file_path, e);
            FsError::Remove {
                path: file_path.to_string_lossy().to_string(),
                source: e,
            }
            .to_string()
        })?;
    } else {
        warn!("Archivo {:?} no existe, nada que eliminar", file_path);
    }
    Ok(())
}

#[tauri::command]
pub async fn add_instance_file(
    id: String,
    sub_dir: String,
    source_path: String,
) -> Result<(), String> {
    validate_uuid(&id)?;
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        warn!("Instancia {} no encontrada para agregar archivo", id);
        return Err(InstanceError::NotFound.to_string());
    };

    let instance_dir = handle.get_instance_dir().await;
    let dest_dir = sanitize_sub_path(&instance_dir, Path::new(&sub_dir))?;
    info!(
        "Agregando archivo '{}' a instancia {} en sub_dir '{}'",
        source_path, id, sub_dir
    );
    if !dest_dir.exists() {
        tokio::fs::create_dir_all(&dest_dir).await.map_err(|e| {
            FsError::CreateDir {
                path: dest_dir.to_string_lossy().to_string(),
                source: e,
            }
            .to_string()
        })?;
    }

    let src = PathBuf::from(&source_path);
    let filename = src
        .file_name()
        .ok_or_else(|| InstanceError::InvalidSourcePath.to_string())?;
    let dest_path = dest_dir.join(filename);

    tokio::fs::copy(&src, &dest_path).await.map_err(|e| {
        error!("Error copiando archivo a {:?}: {}", dest_path, e);
        FsError::Copy {
            from: src.to_string_lossy().to_string(),
            to: dest_path.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;
    info!("Archivo copiado a {:?}", dest_path);
    Ok(())
}

// ─── Dependency checks ───────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct VersionIntegrity {
    pub version_id: String,
    pub dependencies: Vec<String>,
    pub missing: Vec<String>,
    pub complete: bool,
}

#[derive(Serialize, Clone)]
pub struct VersionStatus {
    pub version_id: String,
    pub complete: bool,
    pub missing_deps: Vec<String>,
}

/// Check if a version and all its dependencies are fully installed.
#[tauri::command]
pub async fn check_version_integrity(version_id: String) -> Result<VersionIntegrity, String> {
    let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
    let deps = zellkern::resolve_dependencies(&version_id);

    let missing = tokio::task::spawn_blocking(move || {
        deps.iter()
            .filter(|dep| {
                let json_path = shared_dir.join(format!("versions/{dep}/{dep}.json"));
                !json_path.exists()
            })
            .cloned()
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| e.to_string())?;

    let complete = missing.is_empty();
    let dependencies = zellkern::resolve_dependencies(missing.first().unwrap_or(&version_id));

    Ok(VersionIntegrity {
        version_id,
        dependencies,
        missing,
        complete,
    })
}

/// List all installed versions with their integrity status.
#[tauri::command]
pub async fn get_installed_versions_with_status() -> Vec<VersionStatus> {
    let versions_dir = PathManager::get().get_shared_dir().join("versions");
    let shared_dir = PathManager::get().get_shared_dir().to_path_buf();

    let versions = tokio::task::spawn_blocking(move || -> Vec<String> {
        match std::fs::read_dir(&versions_dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect(),
            Err(_) => Vec::new(),
        }
    })
    .await
    .unwrap_or_default();

    tokio::task::spawn_blocking(move || {
        versions
            .into_iter()
            .map(|v| {
                let deps = zellkern::resolve_dependencies(&v);
                let missing: Vec<String> = deps
                    .iter()
                    .filter(|dep| {
                        let p = shared_dir.join(format!("versions/{dep}/{dep}.json"));
                        !p.exists()
                    })
                    .cloned()
                    .collect();
                VersionStatus {
                    version_id: v,
                    complete: missing.is_empty(),
                    missing_deps: missing,
                }
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn reinstall_version(version: String) -> Result<(), String> {
    info!("Reinstalando versión: {}", version);
    let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
    let deps = zellkern::resolve_dependencies(&version);

    for dep in &deps {
        let version_dir = shared_dir.join(format!("versions/{}", dep));
        if version_dir.exists() {
            info!("Eliminando directorio de versión: {:?}", version_dir);
            tokio::fs::remove_dir_all(&version_dir).await.map_err(|e| {
                error!("Error eliminando versión {}: {}", dep, e);
                e.to_string()
            })?;
        }
    }

    crate::services::DownloadQueue::get()
        .enqueue(version.clone())
        .await;
    info!("Versión {} encolada para re-descarga", version);
    Ok(())
}
