use crate::core::AppEvent;
use crate::core::emit;
use crate::services::java_manager::JavaManager;
use crate::services::DownloadQueue;
use aqua::{JreBatch, JreStatus};
use smallvec::SmallVec;
use tauri::command;
use tracing::info;

#[command]
pub async fn get_jre_status(version: u8) -> Result<JreStatus, String> {
    info!("Getting JRE status for version {}", version);
    JavaManager::get_status(version)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn install_jre(version: u8) -> Result<(), String> {
    info!("Installing JRE {}", version);

    let pkg = JavaManager::get_latest_package(version)
        .await
        .map_err(|e| e.to_string())?;
    let dest_dir = JavaManager::get_jre_dir(version);

    let batch = JreBatch::new(version, pkg, dest_dir);
    DownloadQueue::get()
        .enqueue_batch(format!("jre-{}", version), Box::new(batch))
        .await;

    Ok(())
}

#[command]
pub async fn uninstall_jre(version: u8) -> Result<(), String> {
    info!("Uninstalling JRE {}", version);
    JavaManager::uninstall(version)
        .await
        .map_err(|e| e.to_string())?;
    emit(AppEvent::JREChanged);
    Ok(())
}

#[command]
pub async fn get_jre_versions() -> Result<Vec<JreStatus>, String> {
    info!("Getting status for all JRE versions");
    let versions = [8u8, 17, 21, 25];
    let mut results = SmallVec::<[JreStatus; 4]>::new();
    for v in versions {
        match JavaManager::get_status(v).await {
            Ok(status) => results.push(status),
            Err(e) => {
                results.push(JreStatus {
                    version: v,
                    installed: false,
                    java_version: Some(format!("error: {}", e)),
                });
            }
        }
    }
    Ok(results.into_vec())
}
