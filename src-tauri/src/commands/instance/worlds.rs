use super::launch::validate_uuid;
use crate::services::{InstanceManager, world_manager as worlds};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::ipc::Channel;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorldAction {
    Open { folder: Option<String> },
    OpenDatapacks { folder: String },
    Import { path: PathBuf },
    Export { folder: String, path: PathBuf },
    Duplicate { folder: String },
    Rename { folder: String, name: String },
    Delete { folder: String },
    Size { folder: String },
    CopySeed { folder: String },
    ResetIcon { folder: String },
}

#[derive(Default, Serialize)]
pub struct WorldResult {
    folder: Option<String>,
    size: Option<u64>,
    seed: Option<String>,
}

#[tauri::command]
pub async fn get_instance_worlds(instance_id: String) -> Result<Vec<worlds::WorldDto>, String> {
    validate_uuid(&instance_id)?;
    let handle = InstanceManager::get()
        .get_handle(&instance_id)
        .await
        .ok_or("Instancia no encontrada")?;
    let guard = handle.try_lock_files()?;
    let dir = handle.get_instance_dir().await;
    let slot = worlds::IO_SLOTS
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let _slot = slot;
        worlds::list_worlds(&worlds::saves_dir(&dir, false)?)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn instance_world_action(
    instance_id: String,
    action: WorldAction,
    on_progress: Channel<worlds::WorldProgress>,
) -> Result<WorldResult, String> {
    validate_uuid(&instance_id)?;
    let handle = InstanceManager::get()
        .get_handle(&instance_id)
        .await
        .ok_or("Instancia no encontrada")?;
    let guard = handle.try_lock_files()?;
    if !matches!(action, WorldAction::Open { .. }) && handle.is_busy() {
        return Err("Cierra Minecraft antes de administrar sus mundos".into());
    }
    let dir = handle.get_instance_dir().await;
    let slot = worlds::IO_SLOTS
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || {
        // Keep the lock in the blocking task even if its async caller is cancelled.
        let _guard = guard;
        let _slot = slot;
        let saves = worlds::saves_dir(&dir, true)?;
        let mut callback = |event| {
            let _ = on_progress.send(event);
        };
        let mut progress = worlds::Progress::new(&mut callback);
        let mut result = WorldResult::default();
        match action {
            WorldAction::Open { folder } => {
                let path = match folder {
                    Some(folder) => worlds::world_dir(&saves, &folder)?,
                    None => saves,
                };
                #[cfg(target_os = "windows")]
                let program = "explorer";
                #[cfg(target_os = "macos")]
                let program = "open";
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                let program = "xdg-open";
                std::process::Command::new(program)
                    .arg(path)
                    .spawn()
                    .map_err(|e| e.to_string())?;
            }
            WorldAction::OpenDatapacks { folder } => {
                let path = worlds::world_dir(&saves, &folder)?.join("datapacks");
                std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
                #[cfg(target_os = "windows")]
                let program = "explorer";
                #[cfg(target_os = "macos")]
                let program = "open";
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                let program = "xdg-open";
                std::process::Command::new(program)
                    .arg(path)
                    .spawn()
                    .map_err(|e| e.to_string())?;
            }
            WorldAction::CopySeed { folder } => {
                result.seed = worlds::copy_world_seed(&worlds::world_dir(&saves, &folder)?)?;
            }
            WorldAction::ResetIcon { folder } => {
                worlds::reset_world_icon(&worlds::world_dir(&saves, &folder)?)?;
            }
            WorldAction::Import { path } => {
                result.folder = Some(worlds::import_world(&path, &saves, &mut progress)?)
            }
            WorldAction::Export { folder, path } => {
                worlds::export_zip(&worlds::world_dir(&saves, &folder)?, &path, &mut progress)?
            }
            WorldAction::Duplicate { folder } => {
                result.folder = Some(worlds::duplicate(
                    &worlds::world_dir(&saves, &folder)?,
                    &saves,
                    &mut progress,
                )?)
            }
            WorldAction::Rename { folder, name } => {
                worlds::rename_world(&worlds::world_dir(&saves, &folder)?, &name)?
            }
            WorldAction::Delete { folder } => {
                worlds::delete_world(&worlds::world_dir(&saves, &folder)?)?
            }
            WorldAction::Size { folder } => {
                result.size = Some(worlds::world_size(
                    &worlds::world_dir(&saves, &folder)?,
                    &mut progress,
                )?)
            }
        }
        progress.flush();
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
