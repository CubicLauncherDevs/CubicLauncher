use crate::services::{InstanceManager, mod_versions};

#[tauri::command]
pub async fn replace_instance_mod(
    id: String,
    request: mod_versions::ReplaceRequest,
) -> Result<String, String> {
    super::launch::validate_uuid(&id)?;
    tokio::spawn(async move {
        let handle = InstanceManager::get()
            .get_handle(&id)
            .await
            .ok_or("Instancia no encontrada")?;
        let _guard = handle.try_lock_files()?;
        if handle.is_busy() {
            return Err(
                "No se puede cambiar un mod mientras la instancia está en ejecución".into(),
            );
        }
        let filename = mod_versions::replace(&handle, request).await?;
        crate::core::emit(crate::core::AppEvent::ModsEnriched { id: id.into() });
        Ok(filename)
    })
    .await
    .map_err(|e| e.to_string())?
}
