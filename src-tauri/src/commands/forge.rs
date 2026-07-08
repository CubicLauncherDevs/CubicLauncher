use crate::core::path_manager::PathManager;
use crate::services::DownloadQueue;

/// Queues a Forge version installation. The actual work is done by the download queue.
#[tauri::command]
pub async fn install_forge(game_version: String, forge_version: String) -> Result<String, String> {
    let shared_dir = PathManager::get().get_shared_dir();
    let version_id = format!("{game_version}-forge-{forge_version}");

    let versions_dir = shared_dir.join("versions").join(&version_id);
    if versions_dir.join(format!("{version_id}.json")).exists() {
        return Ok(version_id);
    }

    DownloadQueue::get().enqueue(version_id.clone()).await;
    Ok(version_id)
}

/// Queue a Forge installation through the download queue.
#[tauri::command]
pub async fn download_forge(game_version: String, forge_version: String) -> Result<(), String> {
    let version_id = format!("{game_version}-forge-{forge_version}");
    DownloadQueue::get().enqueue(version_id).await;
    Ok(())
}
