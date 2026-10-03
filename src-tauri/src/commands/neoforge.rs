use crate::services::DownloadQueue;

/// Queues a NeoForge version installation. The actual work is done by the download queue.
#[tauri::command]
pub async fn install_neoforge(
    game_version: String,
    neoforge_version: String,
) -> Result<String, String> {
    let version_id = format!("{game_version}-neoforge-{neoforge_version}");

    DownloadQueue::get().enqueue(version_id.clone()).await;
    Ok(version_id)
}
