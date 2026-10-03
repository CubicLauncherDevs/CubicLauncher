use crate::services::DownloadQueue;

/// Queues a Forge version installation. The actual work is done by the download queue.
#[tauri::command]
pub async fn install_forge(game_version: String, forge_version: String) -> Result<String, String> {
    let version_id = format!("{game_version}-forge-{forge_version}");

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
