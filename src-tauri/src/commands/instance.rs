use crate::core::errors::{FsError, InstanceError};
use crate::core::{AppEvent, PathManager, emit};
use crate::services::{InstanceDto, InstanceManager, InstanceStatus, Launcher, signal_kill};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

fn validate_uuid(uuid: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(uuid).map_err(|_| format!("UUID inválido: '{}'", uuid))?;
    Ok(())
}

fn sanitize_sub_path(instance_dir: &Path, sub_path: &Path) -> Result<PathBuf, String> {
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

async fn read_screenshots_dir(instance_name: &str) -> Vec<PathBuf> {
    let screenshots_dir = PathManager::get()
        .get_instance_dir()
        .join(instance_name)
        .join("screenshots");

    tokio::task::spawn_blocking(move || {
        let mut screenshots: Vec<_> = match std::fs::read_dir(&screenshots_dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
                })
                .collect(),
            Err(_) => return Vec::new(),
        };

        screenshots.sort_by_key(|e| match e.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => std::time::SystemTime::UNIX_EPOCH,
        });

        screenshots.into_iter().map(|e| e.path()).collect()
    })
    .await
    .unwrap_or_default()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Lifecycle
// ═══════════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn launch(instance_id: String) -> Result<(), String> {
    validate_uuid(&instance_id)?;
    info!("Lanzando instancia {}", instance_id);
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&instance_id).await else {
        error!("Instancia {} no encontrada para lanzar", instance_id);
        return Err(InstanceError::NotFound.to_string());
    };

    Launcher::get().launch(handle.clone()).await.map_err(|e| {
        error!("Error lanzando instancia {}: {}", instance_id, e);
        e.to_string()
    })?;

    info!("Instancia {} lanzada exitosamente", instance_id);
    Ok(())
}

// ── Kill ─────────────────────────────────────────────────────────────────────

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

// ═══════════════════════════════════════════════════════════════════════════════
// Querying
// ═══════════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn get_instances() -> Vec<InstanceDto> {
    InstanceManager::get().get_all_dtos().await
}

// ── CRUD ─────────────────────────────────────────────────────────────────────

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

// ═══════════════════════════════════════════════════════════════════════════════
// Screenshots & Cover Image
// ═══════════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn get_instance_screenshot(instance_name: String) -> Option<String> {
    let screenshots = read_screenshots_dir(&instance_name).await;
    screenshots.last().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_all_instance_screenshots(instance_name: String) -> Vec<String> {
    let screenshots = read_screenshots_dir(&instance_name).await;
    screenshots
        .into_iter()
        .rev()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
}

#[tauri::command]
pub async fn set_instance_cover_image(instance_id: String, path: String) {
    if let Err(e) = validate_uuid(&instance_id) {
        warn!("{}", e);
        return;
    }
    info!(
        "Estableciendo cover image para instancia {}: {}",
        instance_id, path
    );
    let manager = InstanceManager::get();
    if let Some(handle) = manager.get_handle(&instance_id).await {
        handle.set_cover_image(Some(PathBuf::from(path))).await;
        if let Err(e) = handle.save_if_dirty().await {
            warn!(
                "Error guardando cover image de instancia {}: {:?}",
                instance_id, e
            );
        }
    } else {
        warn!(
            "Instancia {} no encontrada para establecer cover image",
            instance_id
        );
    }
}

#[tauri::command]
pub async fn reset_instance_cover_image(instance_id: String) {
    if let Err(e) = validate_uuid(&instance_id) {
        warn!("{}", e);
        return;
    }
    info!("Reseteando cover image para instancia {}", instance_id);
    let manager = InstanceManager::get();
    if let Some(handle) = manager.get_handle(&instance_id).await {
        handle.set_cover_image(None).await;
        if let Err(e) = handle.save_if_dirty().await {
            warn!(
                "Error guardando reset cover image de instancia {}: {:?}",
                instance_id, e
            );
        }
    } else {
        warn!(
            "Instancia {} no encontrada para resetear cover image",
            instance_id
        );
    }
}

#[tauri::command]
pub async fn get_instance_banner(instance_id: String) -> Option<String> {
    if validate_uuid(&instance_id).is_err() {
        return None;
    }
    let manager = InstanceManager::get();
    let handle = manager.get_handle(&instance_id).await?;

    if let Some(path) = handle.get_cover_image().await {
        return Some(path.to_string_lossy().to_string());
    }

    get_instance_screenshot(handle.get_name().await.to_string()).await
}

// ═══════════════════════════════════════════════════════════════════════════════
// Edición
// ═══════════════════════════════════════════════════════════════════════════════

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
        return Err(FsError::CreateDir { path: path.to_string_lossy().to_string(), source: e }.to_string());
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
        .update_instance(&id, Some(new_name), None, None)
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
) -> Result<(), String> {
    validate_uuid(&id)?;
    info!(
        "Actualizando instancia {}: name={:?}, version={:?}, icon={:?}",
        id, new_name, new_version, new_icon
    );
    let result = InstanceManager::get()
        .update_instance(&id, new_name, new_version, new_icon)
        .await;
    if let Err(ref e) = result {
        error!("Error actualizando instancia {}: {}", id, e);
    }
    result
}

// ── Versiones ────────────────────────────────────────────────────────────────

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

// ═══════════════════════════════════════════════════════════════════════════════
// Mods
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(serde::Serialize)]
pub struct ModDto {
    pub name: String,
    pub filename: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
    pub icon: Option<String>,
    pub enabled: bool,
}

#[tauri::command]
pub async fn get_instance_mods(id: String) -> Vec<ModDto> {
    if let Err(e) = validate_uuid(&id) {
        warn!("{}", e);
        return Vec::new();
    }
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        warn!("Instancia {} no encontrada para listar mods", id);
        return Vec::new();
    };

    let mods_dir = handle.get_instance_dir().await.join("mods");
    info!("Listando mods de instancia {} en {:?}", id, mods_dir);

    let mod_paths = tokio::task::spawn_blocking(move || -> Vec<PathBuf> {
        match std::fs::read_dir(&mods_dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| e.path().is_file())
                .map(|e| e.path())
                .collect(),
            Err(_) => Vec::new(),
        }
    })
    .await
    .unwrap_or_default();

    // Filtrar y preparar la info básica de cada path (síncrono, barato)
    struct ModEntry {
        path: PathBuf,
        filename: String,
        display_name: String,
        enabled: bool,
    }

    let entries: Vec<ModEntry> = mod_paths
        .into_iter()
        .filter_map(|path| {
            let ext = path.extension()?.to_string_lossy().to_lowercase();
            let file_name = path.file_name()?.to_string_lossy().to_string();
            let file_name_lower = file_name.to_lowercase();

            let (is_mod, enabled) = if ext == "jar" || ext == "zip" {
                (true, true)
            } else if ext == "disabled"
                && (file_name_lower.ends_with(".jar.disabled")
                    || file_name_lower.ends_with(".zip.disabled"))
            {
                (true, false)
            } else {
                (false, false)
            };

            if !is_mod {
                return None;
            }

            let display_name = file_name
                .strip_suffix(".disabled")
                .map(|s| s.to_string())
                .unwrap_or_else(|| file_name.clone());

            Some(ModEntry {
                path,
                filename: file_name,
                display_name,
                enabled,
            })
        })
        .collect();

    // Parsear todos los ZIPs en paralelo
    let handles: Vec<_> = entries
        .iter()
        .map(|e| {
            let path = e.path.clone();
            tokio::task::spawn_blocking(move || crate::services::AddonManager::get_mod_info(&path))
        })
        .collect();

    let metadatas = futures::future::join_all(handles).await;

    // Combinar resultados
    let mut mods: Vec<ModDto> = entries
        .into_iter()
        .zip(metadatas)
        .map(|(entry, meta_result)| {
            let metadata = meta_result.unwrap_or(None);
            let (md_name, md_version, md_desc, md_authors, md_icon) = match metadata {
                Some(m) => (m.name, m.version, m.description, m.authors, m.icon),
                None => (entry.display_name, None, None, None, None),
            };
            ModDto {
                name: md_name,
                filename: entry.filename,
                version: md_version,
                description: md_desc,
                authors: md_authors,
                icon: md_icon.map(|s| (*s).clone()),
                enabled: entry.enabled,
            }
        })
        .collect();

    mods.sort_by_key(|a| a.name.to_lowercase());
    info!("{} mods encontrados en instancia {}", mods.len(), id);
    mods
}

// ── Toggle mod ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn toggle_instance_mod(id: String, filename: String, enable: bool) -> Result<(), String> {
    validate_uuid(&id)?;
    info!(
        "Cambiando estado del mod '{}' en instancia {}: enable={}",
        filename, id, enable
    );
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        error!("Instancia {} no encontrada para toggle mod", id);
        return Err(InstanceError::NotFound.to_string());
    };

    let mods_dir = handle.get_instance_dir().await.join("mods");
    let file_path = mods_dir.join(&filename);

    if !file_path.exists() {
        error!("Mod '{}' no encontrado en instancia {}", filename, id);
        return Err(InstanceError::ModNotFound.to_string());
    }

    let is_currently_disabled = filename.ends_with(".disabled");

    if enable && is_currently_disabled {
        let new_filename = filename
            .strip_suffix(".disabled")
            .ok_or_else(|| InstanceError::FilenameParse.to_string())?;
        let new_path = mods_dir.join(new_filename);
        tokio::fs::rename(file_path, new_path)
            .await
            .map_err(|e| e.to_string())?;
        info!("Mod '{}' habilitado en instancia {}", new_filename, id);
    } else if !enable && !is_currently_disabled {
        let new_filename = format!("{}.disabled", filename);
        let new_path = mods_dir.join(new_filename);
        tokio::fs::rename(file_path, new_path)
            .await
            .map_err(|e| e.to_string())?;
        info!("Mod '{}' deshabilitado en instancia {}", filename, id);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Resource Packs
// ═══════════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn get_instance_resourcepacks(id: String) -> Vec<ModDto> {
    if let Err(e) = validate_uuid(&id) {
        warn!("{}", e);
        return Vec::new();
    }
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        warn!("Instancia {} no encontrada para listar resourcepacks", id);
        return Vec::new();
    };

    let resourcepacks_dir = handle.get_instance_dir().await.join("resourcepacks");

    let rp_paths = tokio::task::spawn_blocking(move || -> Vec<PathBuf> {
        match std::fs::read_dir(&resourcepacks_dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| e.path().is_file())
                .map(|e| e.path())
                .collect(),
            Err(_) => Vec::new(),
        }
    })
    .await
    .unwrap_or_default();

    let mut resourcepacks = Vec::new();
    for path in rp_paths {
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let filename = file_name.to_string_lossy().to_string();
        let path_clone = path.clone();
        let metadata = tokio::task::spawn_blocking(move || {
            crate::services::AddonManager::get_resourcepack_info(&path_clone)
        })
        .await
        .unwrap_or(None);

        let (md_name, md_desc, md_icon) = match metadata {
            Some(m) => (m.name, m.description, m.icon),
            None => (filename.clone(), None, None),
        };

        resourcepacks.push(ModDto {
            name: md_name,
            filename,
            version: None,
            description: md_desc,
            authors: None,
            icon: md_icon.map(|s| (*s).clone()),
            enabled: true,
        });
    }
    resourcepacks.sort_by_key(|a| a.name.to_lowercase());
    info!(
        "{} resourcepacks encontrados en instancia {}",
        resourcepacks.len(),
        id
    );
    resourcepacks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── validate_uuid ─────────────────────────────────────────────────────

    /// Un UUID válido en formato estándar debe pasar la validación.
    #[test]
    fn test_validate_uuid_valid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
    }

    /// Un string que no es un UUID debe ser rechazado.
    /// Previene errores silenciosos aguas abajo.
    #[test]
    fn test_validate_uuid_invalid() {
        assert!(validate_uuid("not-a-uuid").is_err());
    }

    /// Un string vacío no es un UUID válido.
    #[test]
    fn test_validate_uuid_empty() {
        assert!(validate_uuid("").is_err());
    }

    // ── sanitize_sub_path ─────────────────────────────────────────────────

    /// Una ruta absoluta debe ser rechazada para prevenir path traversal.
    /// El usuario no debería poder acceder a `/etc/passwd` usando este argumento.
    #[test]
    fn test_sanitize_absolute_path() {
        let base = Path::new("/tmp/instances/test");
        let result = sanitize_sub_path(base, Path::new("/etc/passwd"));
        assert!(result.is_err());
    }

    /// `../` debe ser rechazado para prevenir salir del directorio de la instancia.
    #[test]
    fn test_sanitize_parent_dir() {
        let base = Path::new("/tmp/instances/test");
        let result = sanitize_sub_path(base, Path::new("../malicious"));
        assert!(result.is_err());
    }

    /// `../` anidado en medio de una ruta también debe ser rechazado.
    /// `mods/../../secrets` sigue siendo path traversal aunque no empiece con `..`.
    #[test]
    fn test_sanitize_dotdot_nested() {
        let base = Path::new("/tmp/instances/test");
        let result = sanitize_sub_path(base, Path::new("mods/../../secrets"));
        assert!(result.is_err());
    }

    /// Un subdirectorio simple como `mods` debe concatenarse correctamente
    /// con el directorio base de la instancia.
    #[test]
    fn test_sanitize_valid_sub_path() {
        let base = PathBuf::from("/tmp/instances/test");
        let result = sanitize_sub_path(&base, Path::new("mods"));
        assert_eq!(result.unwrap(), base.join("mods"));
    }

    /// Rutas anidadas válidas (sin `..`) deben funcionar correctamente.
    #[test]
    fn test_sanitize_nested_valid() {
        let base = PathBuf::from("/tmp/instances/test");
        let result = sanitize_sub_path(&base, Path::new("screenshots/2025-01-01"));
        assert_eq!(result.unwrap(), base.join("screenshots/2025-01-01"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// File Operations
// ═══════════════════════════════════════════════════════════════════════════════

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
            FsError::Remove { path: file_path.to_string_lossy().to_string(), source: e }.to_string()
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
        tokio::fs::create_dir_all(&dest_dir)
            .await
            .map_err(|e| FsError::CreateDir { path: dest_dir.to_string_lossy().to_string(), source: e }.to_string())?;
    }

    let src = PathBuf::from(&source_path);
    let filename = src.file_name().ok_or_else(|| InstanceError::InvalidSourcePath.to_string())?;
    let dest_path = dest_dir.join(filename);

    tokio::fs::copy(&src, &dest_path).await.map_err(|e| {
        error!("Error copiando archivo a {:?}: {}", dest_path, e);
        FsError::Copy { from: src.to_string_lossy().to_string(), to: dest_path.to_string_lossy().to_string(), source: e }.to_string()
    })?;
    info!("Archivo copiado a {:?}", dest_path);
    Ok(())
}
