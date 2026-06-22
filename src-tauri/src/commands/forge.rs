use crate::core::path_manager::PathManager;
use crate::services::DownloadQueue;
use crate::services::java_manager::JavaManager;

/// Install a Forge version. Downloads the installer, extracts, downloads libraries,
/// and runs post-processors. The version will be available for launch afterward.
#[tauri::command]
pub async fn install_forge(game_version: String, forge_version: String) -> Result<String, String> {
    let shared_dir = PathManager::get().get_shared_dir();
    let version_id = format!("{game_version}-forge-{forge_version}");

    let versions_dir = shared_dir.join("versions").join(&version_id);
    if versions_dir.join(format!("{version_id}.json")).exists() {
        return Ok(version_id);
    }

    if !JavaManager::is_installed(21) {
        JavaManager::install(21)
            .await
            .map_err(|e| format!("No se pudo instalar Java 21: {e}"))?;
    }
    let java_path = Some(JavaManager::get_java_binary(21));

    let manifest = aqua::ForgeBatch::install(shared_dir, &game_version, &forge_version, java_path)
        .await
        .map_err(|e| format!("Forge installation failed: {e}"))?;

    Ok(manifest.id_raw)
}

/// Queue a Forge installation through the download queue.
#[tauri::command]
pub async fn download_forge(game_version: String, forge_version: String) -> Result<(), String> {
    let version_id = format!("{game_version}-forge-{forge_version}");
    DownloadQueue::get().enqueue(version_id).await;
    Ok(())
}
