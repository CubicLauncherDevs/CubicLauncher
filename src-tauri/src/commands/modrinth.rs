use aqua::{DownloadItemSpec, DownloadManager, GenericBatch};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::core::PathManager;
use crate::core::errors::{DownloadError, FsError, InstanceError};
use crate::services::InstanceManager;

#[derive(Debug, Serialize, Deserialize)]
pub struct ModDownloadInfo {
    pub url: String,
    pub filename: String,
}

#[tauri::command]
pub async fn download_mods(instance_id: String, mods: Vec<ModDownloadInfo>) -> Result<(), String> {
    let manager = InstanceManager::get();
    let handle = manager
        .get_handle(&instance_id)
        .await
        .ok_or_else(|| InstanceError::NotFound.to_string())?;
    let mods_dir = handle.get_instance_dir().await.join("mods");

    tokio::fs::create_dir_all(&mods_dir).await.map_err(|e| {
        FsError::CreateDir {
            path: mods_dir.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;

    let count = mods.len();
    let items: Vec<DownloadItemSpec> = mods
        .into_iter()
        .map(|m| {
            info!(
                "Encolando mod: {} -> {:?}",
                m.filename,
                mods_dir.join(&m.filename)
            );
            DownloadItemSpec::new(m.url, mods_dir.join(m.filename), "mod")
        })
        .collect();

    let batch = GenericBatch::new(format!("mods-{}", instance_id), items);

    let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
    let dm = DownloadManager::new(shared_dir);
    let handle = dm
        .prepare_batch(Box::new(batch))
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    handle
        .download_all(None)
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    info!("{} mods descargados correctamente en {:?}", count, mods_dir);
    Ok(())
}

#[tauri::command]
pub async fn download_resourcepacks(
    instance_id: String,
    packs: Vec<ModDownloadInfo>,
) -> Result<(), String> {
    let manager = InstanceManager::get();
    let handle = manager
        .get_handle(&instance_id)
        .await
        .ok_or_else(|| InstanceError::NotFound.to_string())?;
    let rp_dir = handle.get_instance_dir().await.join("resourcepacks");

    tokio::fs::create_dir_all(&rp_dir).await.map_err(|e| {
        FsError::CreateDir {
            path: rp_dir.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;

    let count = packs.len();
    let items: Vec<DownloadItemSpec> = packs
        .into_iter()
        .map(|m| {
            info!(
                "Encolando resourcepack: {} -> {:?}",
                m.filename,
                rp_dir.join(&m.filename)
            );
            DownloadItemSpec::new(m.url, rp_dir.join(m.filename), "resourcepack")
        })
        .collect();

    let batch = GenericBatch::new(format!("resourcepacks-{}", instance_id), items);

    let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
    let dm = DownloadManager::new(shared_dir);
    let handle = dm
        .prepare_batch(Box::new(batch))
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    handle
        .download_all(None)
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    info!(
        "{} resourcepacks descargados correctamente en {:?}",
        count, rp_dir
    );
    Ok(())
}

#[tauri::command]
pub async fn download_shaderpacks(
    instance_id: String,
    packs: Vec<ModDownloadInfo>,
) -> Result<(), String> {
    let manager = InstanceManager::get();
    let handle = manager
        .get_handle(&instance_id)
        .await
        .ok_or_else(|| InstanceError::NotFound.to_string())?;
    let sp_dir = handle.get_instance_dir().await.join("shaderpacks");

    tokio::fs::create_dir_all(&sp_dir).await.map_err(|e| {
        FsError::CreateDir {
            path: sp_dir.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;

    let count = packs.len();
    let items: Vec<DownloadItemSpec> = packs
        .into_iter()
        .map(|m| {
            info!(
                "Encolando shaderpack: {} -> {:?}",
                m.filename,
                sp_dir.join(&m.filename)
            );
            DownloadItemSpec::new(m.url, sp_dir.join(m.filename), "shaderpack")
        })
        .collect();

    let batch = GenericBatch::new(format!("shaderpacks-{}", instance_id), items);

    let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
    let dm = DownloadManager::new(shared_dir);
    let handle = dm
        .prepare_batch(Box::new(batch))
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    handle
        .download_all(None)
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    info!(
        "{} shaderpacks descargados correctamente en {:?}",
        count, sp_dir
    );
    Ok(())
}
