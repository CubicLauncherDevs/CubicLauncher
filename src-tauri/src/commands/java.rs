use crate::core::emit;
use crate::core::AppEvent;
use crate::services::java_manager::JavaManager;
use aqua::JreStatus;
use smallvec::SmallVec;
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use tauri::command;
use tracing::info;

static INSTALLING_JRES: LazyLock<Mutex<HashSet<u8>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

#[command]
pub async fn get_jre_status(version: u8) -> Result<JreStatus, String> {
    info!("Getting JRE status for version {}", version);
    JavaManager::get_status(version)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_installing_jres() -> Vec<u8> {
    INSTALLING_JRES.lock().expect("poisoned INSTALLING_JRES lock").iter().copied().collect()
}

#[command]
pub async fn install_jre(version: u8) -> Result<(), String> {
    info!("Installing JRE {}", version);
    INSTALLING_JRES.lock().expect("poisoned INSTALLING_JRES lock").insert(version);
    let result = JavaManager::install(version).await.map_err(|e| e.to_string());
    INSTALLING_JRES.lock().expect("poisoned INSTALLING_JRES lock").remove(&version);
    result?;
    emit(AppEvent::DFinishRuntime {
        version: version.to_string().into(),
    });
    emit(AppEvent::JREChanged);
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
