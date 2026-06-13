use crate::core::PathManager;
use crate::services::InstanceManager;
use std::path::PathBuf;
use tracing::{info, warn};

use super::launch::validate_uuid;

async fn read_screenshots_dir(instance_name: &str) -> Vec<PathBuf> {
    let screenshots_dir = PathManager::get()
        .get_instance_dir()
        .join(instance_name)
        .join("screenshots");

    tokio::task::spawn_blocking(move || {
        let mut screenshots: Vec<_> = match std::fs::read_dir(&screenshots_dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
                })
                .collect(),
            Err(_) => return Vec::new(),
        };

        screenshots.sort_by_key(|e| match e.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => std::time::SystemTime::UNIX_EPOCH,
        });

        screenshots.into_iter().map(|e| e.path()).collect()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn get_instance_screenshot(instance_name: String) -> Option<String> {
    let screenshots = read_screenshots_dir(&instance_name).await;
    screenshots.last().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_all_instance_screenshots(instance_name: String) -> Vec<String> {
    let screenshots = read_screenshots_dir(&instance_name).await;
    screenshots
        .into_iter()
        .rev()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
}

#[tauri::command]
pub async fn set_instance_cover_image(instance_id: String, path: String) {
    if let Err(e) = validate_uuid(&instance_id) {
        warn!("{}", e);
        return;
    }
    info!(
        "Estableciendo cover image para instancia {}: {}",
        instance_id, path
    );
    let manager = InstanceManager::get();
    if let Some(handle) = manager.get_handle(&instance_id).await {
        handle.set_cover_image(Some(PathBuf::from(path))).await;
        if let Err(e) = handle.save_if_dirty().await {
            warn!(
                "Error guardando cover image de instancia {}: {:?}",
                instance_id, e
            );
        }
    } else {
        warn!(
            "Instancia {} no encontrada para establecer cover image",
            instance_id
        );
    }
}

#[tauri::command]
pub async fn reset_instance_cover_image(instance_id: String) {
    if let Err(e) = validate_uuid(&instance_id) {
        warn!("{}", e);
        return;
    }
    info!("Reseteando cover image para instancia {}", instance_id);
    let manager = InstanceManager::get();
    if let Some(handle) = manager.get_handle(&instance_id).await {
        handle.set_cover_image(None).await;
        if let Err(e) = handle.save_if_dirty().await {
            warn!(
                "Error guardando reset cover image de instancia {}: {:?}",
                instance_id, e
            );
        }
    } else {
        warn!(
            "Instancia {} no encontrada para resetear cover image",
            instance_id
        );
    }
}

#[tauri::command]
pub async fn get_instance_banner(instance_id: String) -> Option<String> {
    if validate_uuid(&instance_id).is_err() {
        return None;
    }
    let manager = InstanceManager::get();
    let handle = manager.get_handle(&instance_id).await?;

    if let Some(path) = handle.get_cover_image().await {
        return Some(path.to_string_lossy().to_string());
    }

    get_instance_screenshot(handle.get_name().await.to_string()).await
}
