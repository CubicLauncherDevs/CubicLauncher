use crate::core::errors::{CoreError, InstanceError};
use crate::core::{AppEvent, emit};
use crate::services::{InstanceManager, options_profiles};
use options_profiles::BackupMode;
use tauri::command;
use tracing::info;

use super::launch::validate_uuid;

/// Aplica un perfil de `options.txt` a una instancia.
///
/// Con `overwrite = false` no sobrescribe: devuelve `INST_OPTIONS_EXISTS` para
/// que el frontend confirme con el usuario antes de reintentar con `true`.
#[command]
pub async fn apply_options_profile(
    id: String,
    profile_id: String,
    overwrite: bool,
) -> Result<options_profiles::ApplyOutcome, String> {
    validate_uuid(&id)?;
    let profile = options_profiles::get(&profile_id)
        .ok_or_else(|| CoreError::Other("Perfil no encontrado".into()).to_string())?;

    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        return Err(InstanceError::NotFound.to_string());
    };

    let files_guard = handle.try_lock_files()?;
    if handle.is_busy() {
        return Err(InstanceError::Busy.to_string());
    }
    let instance_dir = handle.get_instance_dir().await;
    if !overwrite && instance_dir.join("options.txt").exists() {
        return Err(InstanceError::OptionsExists.into());
    }

    let backup_root = options_profiles::backup_root();
    let key = handle.uuid.to_string();
    let content = profile.content;
    let result = tokio::task::spawn_blocking(move || {
        options_profiles::apply_to_instance(
            &instance_dir,
            &key,
            &content,
            BackupMode::Timestamped,
            &backup_root,
        )
    })
    .await
    .map_err(|e| CoreError::Other(e.to_string()).to_string())?;
    drop(files_guard);

    let outcome = result.map_err(|e| e.to_json())?;
    info!(
        "Perfil '{}' aplicado a instancia {} (cambio: {})",
        profile_id, id, outcome.changed
    );
    Ok(outcome)
}

/// Asigna (o quita) el perfil de `options.txt` de una instancia.
#[command]
pub async fn set_instance_options_profile(
    id: String,
    profile_id: Option<String>,
    auto_apply: bool,
) -> Result<(), String> {
    validate_uuid(&id)?;
    if let Some(pid) = profile_id.as_deref()
        && options_profiles::get(pid).is_none()
    {
        return Err(CoreError::Other("Perfil no encontrado".into()).to_string());
    }

    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        return Err(InstanceError::NotFound.to_string());
    };

    let files_guard = handle.try_lock_files()?;
    handle
        .set_options_profile(profile_id.map(|s| s.into()), auto_apply)
        .await;
    files_guard
        .save_if_dirty()
        .await
        .map_err(|e| format!("Error al guardar la instancia: {}", e))?;

    let dto = handle.to_dto().await;
    emit(AppEvent::InstanceEdited {
        id: handle.uuid.to_string().into(),
        dto: Some(dto),
    });
    Ok(())
}
