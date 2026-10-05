use super::launch::validate_uuid;
use crate::core::InstanceError;
use crate::services::InstanceManager;
use crate::services::mrpack_export::{self, ExportPreview, ExportRequest};
use std::path::PathBuf;

/// Preview is local-only; no hashing or network requests until export is confirmed.
#[tauri::command]
pub async fn preview_mrpack_export(id: String) -> Result<ExportPreview, String> {
    validate_uuid(&id)?;
    let handle = InstanceManager::get()
        .get_handle(&id)
        .await
        .ok_or_else(|| String::from(InstanceError::NotFound))?;
    let guard = handle.try_lock_files()?;
    if handle.is_busy() {
        return Err(InstanceError::Busy.into());
    }
    let input = mrpack_export::prepare_export(&handle).await?;
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        mrpack_export::preview(&input)
    })
    .await
    .map_err(|e| format!("No se pudo listar la instancia: {e}"))?
}

#[tauri::command]
pub async fn export_instance_mrpack(
    id: String,
    dest: String,
    request: ExportRequest,
) -> Result<String, String> {
    validate_uuid(&id)?;
    let handle = InstanceManager::get()
        .get_handle(&id)
        .await
        .ok_or_else(|| String::from(InstanceError::NotFound))?;
    let guard = handle.try_lock_files()?;
    if handle.is_busy() {
        return Err(InstanceError::Busy.into());
    }
    let input = mrpack_export::prepare_export(&handle).await?;
    // Move the guard into each blocking job: cancellation must not release the
    // instance lock while filesystem work is still running on the worker thread.
    let (mut snapshot, guard) = tokio::task::spawn_blocking(move || {
        mrpack_export::snapshot(&input, request, &PathBuf::from(dest))
            .map(|snapshot| (snapshot, guard))
    })
    .await
    .map_err(|e| format!("No se pudo preparar la exportación: {e}"))??;
    mrpack_export::resolve_downloads(&mut snapshot).await?;
    let path = tokio::task::spawn_blocking(move || {
        let _guard = guard;
        mrpack_export::write_archive(snapshot)
    })
    .await
    .map_err(|e| format!("No se pudo exportar el modpack: {e}"))??;
    Ok(path.to_string_lossy().into_owned())
}
