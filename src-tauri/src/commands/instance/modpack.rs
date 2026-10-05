use crate::core::{AppEvent, emit};
use crate::services::modpack::{self, PackState};
use crate::services::modpack_update::{self, PackVersion, Preview};
use crate::services::{InstanceHandle, InstanceManager};
use std::collections::BTreeMap;

async fn instance(id: &str) -> Result<InstanceHandle, String> {
    super::launch::validate_uuid(id)?;
    InstanceManager::get()
        .get_handle(id)
        .await
        .ok_or_else(|| "Instancia no encontrada".into())
}

#[tauri::command]
pub async fn get_instance_modpack(id: String) -> Result<Option<PackState>, String> {
    let handle = instance(&id).await?;
    // The inventory is replaced atomically. Read-only catalog queries must not
    // contend with the background mod-enrichment filesystem lease.
    modpack::read_state(&handle.get_instance_dir().await).await
}

#[tauri::command]
pub async fn restore_modpack_inventory(id: String) -> Result<PackState, String> {
    tokio::spawn(async move {
        let handle = instance(&id).await?;
        let _guard = handle.try_lock_files()?;
        if handle.is_busy() {
            return Err("La instancia está en ejecución".into());
        }
        let root = handle.get_instance_dir().await;
        let state = modpack::read_state(&root)
            .await?
            .ok_or("La instancia no tiene modpack")?;
        let state = modpack_update::restore_legacy(&root, state).await?;
        emit(AppEvent::InstanceEdited {
            id: id.into(),
            dto: Some(handle.to_dto().await),
        });
        Ok(state)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_modpack_locked(id: String, locked: bool) -> Result<PackState, String> {
    tokio::spawn(async move {
        let handle = instance(&id).await?;
        let guard = handle.try_lock_files()?;
        if handle.is_busy() {
            return Err("La instancia está en ejecución".into());
        }
        let root = handle.get_instance_dir().await;
        let mut state = modpack::read_state(&root)
            .await?
            .ok_or("La instancia no tiene modpack")?;
        state.locked = locked;
        let state = tokio::task::spawn_blocking(move || {
            let _guard = guard;
            modpack::save(&root, &state)?;
            Ok::<_, String>(state)
        })
        .await
        .map_err(|e| e.to_string())??;
        emit(AppEvent::InstanceEdited {
            id: id.into(),
            dto: Some(handle.to_dto().await),
        });
        Ok(state)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_modpack_versions(id: String) -> Result<Vec<PackVersion>, String> {
    let handle = instance(&id).await?;
    let state = modpack::read_state(&handle.get_instance_dir().await)
        .await?
        .ok_or("La instancia no tiene modpack")?;
    modpack_update::versions(&state).await
}

#[tauri::command]
pub async fn preview_modpack_update(
    id: String,
    version_id: Option<String>,
    path: Option<String>,
) -> Result<Preview, String> {
    tokio::spawn(async move {
        let handle = instance(&id).await?;
        let _guard = handle.try_lock_files()?;
        if handle.is_busy() {
            return Err("La instancia está en ejecución".into());
        }
        modpack_update::prepare(&handle, version_id, path).await
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_modpack_update(
    id: String,
    token: String,
    resolutions: BTreeMap<String, String>,
) -> Result<(), String> {
    tokio::spawn(async move {
        let handle = instance(&id).await?;
        let _guard = handle.try_lock_files()?;
        if handle.is_busy() {
            return Err("La instancia está en ejecución".into());
        }
        modpack_update::apply(&handle, token, resolutions).await?;
        emit(AppEvent::InstanceEdited {
            id: id.into(),
            dto: Some(handle.to_dto().await),
        });
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn cancel_modpack_update(id: String, token: String) -> Result<(), String> {
    super::launch::validate_uuid(&id)?;
    modpack_update::cancel(&id, &token)
}
