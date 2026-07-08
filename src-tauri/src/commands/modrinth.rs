use aqua::{DownloadItemSpec, DownloadManager, GenericBatch};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::commands::instance::mods::{
    read_mods_metadata, write_mods_metadata, ModSourceMetadata,
};
use crate::core::PathManager;
use crate::core::errors::{DownloadError, FsError, InstanceError};
use crate::services::InstanceManager;

const USER_AGENT: &str = concat!("CubicLauncher/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Serialize, Deserialize)]
pub struct ModDownloadInfo {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
}

#[tauri::command]
pub async fn download_mods(instance_id: String, mods: Vec<ModDownloadInfo>) -> Result<(), String> {
    let manager = InstanceManager::get();
    let handle = manager
        .get_handle(&instance_id)
        .await
        .ok_or_else(|| InstanceError::NotFound.to_string())?;
    let instance_dir = handle.get_instance_dir().await;
    let mods_dir = instance_dir.join("mods");

    tokio::fs::create_dir_all(&mods_dir).await.map_err(|e| {
        FsError::CreateDir {
            path: mods_dir.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;

    let count = mods.len();
    let items: Vec<DownloadItemSpec> = mods
        .iter()
        .map(|m| {
            info!(
                "Encolando mod: {} -> {:?}",
                m.filename,
                mods_dir.join(&m.filename)
            );
            DownloadItemSpec::new(m.url.clone(), mods_dir.join(&m.filename), "mod")
        })
        .collect();

    let batch = GenericBatch::new(format!("mods-{}", instance_id), items);

    let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
    let dm = DownloadManager::new(shared_dir);
    let dl_handle = dm
        .prepare_batch(Box::new(batch))
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    dl_handle
        .download_all(None)
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    // Persist source metadata for installed mods so the market can match local files to projects.
    let mut metadata = read_mods_metadata(&instance_dir).await?.unwrap_or_default();
    for m in &mods {
        if let (Some(project_id), Some(version_id)) = (&m.project_id, &m.version_id) {
            metadata.insert(
                m.filename.clone(),
                ModSourceMetadata {
                    project_id: project_id.clone(),
                    version_id: version_id.clone(),
                },
            );
        }
    }
    write_mods_metadata(&instance_dir, metadata).await?;

    info!("{} mods descargados y metadata persistida en {:?}", count, mods_dir);
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

#[tauri::command]
pub async fn download_mrpack(url: String, version_id: String) -> Result<String, String> {
    info!("Downloading mrpack from {} (version: {})", url, version_id);

    let cache_dir = std::env::temp_dir().join("cubiclauncher").join("mrpack-cache");
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;

    let filename = format!("{}.mrpack", version_id);
    let dest = cache_dir.join(&filename);

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} when downloading mrpack", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    tokio::fs::write(&dest, &bytes)
        .await
        .map_err(|e| format!("Failed to write mrpack file: {}", e))?;

    let path_str = dest.to_string_lossy().to_string();
    info!("Mrpack downloaded to {}", path_str);
    Ok(path_str)
}
