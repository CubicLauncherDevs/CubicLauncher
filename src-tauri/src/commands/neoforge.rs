use crate::core::path_manager::PathManager;
use crate::services::DownloadQueue;

/// Queues a NeoForge version installation. The actual work is done by the download queue.
#[tauri::command]
pub async fn install_neoforge(
    game_version: String,
    neoforge_version: String,
) -> Result<String, String> {
    let shared_dir = PathManager::get().get_shared_dir();
    let version_id = format!("{game_version}-neoforge-{neoforge_version}");

    let versions_dir = shared_dir.join("versions").join(&version_id);
    if versions_dir.join(format!("{version_id}.json")).exists() {
        return Ok(version_id);
    }

    DownloadQueue::get().enqueue(version_id.clone()).await;
    Ok(version_id)
}

