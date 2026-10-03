use crate::core::AppEvent;
use crate::core::emit;
use crate::services::DownloadQueue;
use crate::services::java_manager::JavaManager;
use aqua::{JreBatch, JreStatus, JreVendor};
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
pub async fn install_jre(version: u8, vendor: Option<String>) -> Result<(), String> {
    let shared_guard = crate::services::shared_storage::acquire().await;
    let preferred = vendor.as_deref().and_then(JreVendor::from_id);
    if vendor.is_some() && preferred.is_none() {
        info!("Unknown JRE vendor requested, using the default chain");
    }
    info!(
        "Installing JRE {} (vendor: {:?})",
        version,
        preferred.map(|vendor| vendor.id())
    );

    let pkg = JavaManager::get_latest_package(version, preferred)
        .await
        .map_err(|e| e.to_string())?;
    let dest_dir = JavaManager::get_jre_dir(version);

    let batch = JreBatch::new(version, pkg, dest_dir);
    DownloadQueue::get()
        .enqueue_batch(format!("jre-{}", version), Box::new(batch), shared_guard)
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
pub async fn get_available_jre_vendors(version: u8) -> Result<Vec<String>, String> {
    info!("Getting available JRE vendors for version {}", version);
    JavaManager::get_available_vendors(version)
        .await
        .map(|vendors| {
            vendors
                .iter()
                .map(|vendor| vendor.id().to_string())
                .collect()
        })
        .map_err(|e| e.to_string())
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
                    vendor: None,
                });
            }
        }
    }
    Ok(results.into_vec())
}
