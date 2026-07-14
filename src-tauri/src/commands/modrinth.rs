use aqua::{DownloadItemSpec, DownloadManager, GenericBatch};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::commands::instance::mods::{PerFileCacheEntry, repo_path};
use crate::core::PathManager;
use crate::core::errors::{DownloadError, FsError, InstanceError};
use crate::services::InstanceManager;
use crate::services::compute_file_sha1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDownloadInfo {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
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
            let mut spec = DownloadItemSpec::new(m.url.clone(), mods_dir.join(&m.filename), "mod");
            if let Some(hash) = &m.sha1
                && !hash.is_empty()
            {
                spec = spec.with_hash(hash.clone());
            }
            info!(
                "Encolando mod: {} -> {:?}{}",
                m.filename,
                mods_dir.join(&m.filename),
                m.sha1
                    .as_ref()
                    .map(|h| format!(" (sha1: {})", &h[..8.min(h.len())]))
                    .unwrap_or_default()
            );
            spec
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

    // Post-download: compute SHA1 in parallel and cache metadata in ablage
    let cache_path = repo_path(&mods_dir);
    let mut repo = ablage::Repo::open(&cache_path);

    let sha1_futs: Vec<_> = mods
        .iter()
        .map(|m| {
            let file_path = mods_dir.join(&m.filename);
            tokio::task::spawn_blocking(move || {
                let sha1 = compute_file_sha1(&file_path).unwrap_or_default();
                (file_path, sha1)
            })
        })
        .collect();

    let sha1_results: Vec<_> = futures::future::join_all(sha1_futs)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    for (m, (file_path, sha1)) in mods.iter().zip(sha1_results.iter()) {
        if !file_path.exists() || sha1.is_empty() {
            if sha1.is_empty() {
                warn!("No se pudo computar SHA1 para {}", m.filename);
            }
            continue;
        }

        let source = if let (Some(project_id), Some(version_id)) = (&m.project_id, &m.version_id) {
            crate::services::ModSource::Modrinth {
                project_id: project_id.clone(),
                version_id: version_id.clone(),
                slug: None,
            }
        } else if let Some(project_id) = &m.project_id {
            crate::services::ModSource::CurseForge {
                project_id: project_id.clone(),
                file_id: m.version_id.clone().unwrap_or_default(),
            }
        } else {
            crate::services::ModSource::Local
        };

        let entry = PerFileCacheEntry {
            sha1: sha1.clone(),
            metadata: None,
            source,
        };

        if let Ok(data) = postcard::to_stdvec(&entry)
            && repo.get(sha1).is_none()
        {
            repo.put(
                sha1.clone(),
                ablage::Entry {
                    version: 1,
                    fingerprint: 0,
                    data,
                },
            );
        }
    }

    let _ = repo.flush();

    info!("{} mods descargados y cacheados en {:?}", count, mods_dir);
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
        .iter()
        .map(|m| {
            info!(
                "Encolando resourcepack: {} -> {:?}",
                m.filename,
                rp_dir.join(&m.filename)
            );
            DownloadItemSpec::new(m.url.clone(), rp_dir.join(&m.filename), "resourcepack")
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

    // Cache metadata via ablage
    for m in &packs {
        if let (Some(project_id), Some(version_id)) = (&m.project_id, &m.version_id) {
            crate::services::save_pack_cache(
                &rp_dir,
                &m.filename,
                &crate::services::PackCacheEntry {
                    source: crate::services::ModSource::Modrinth {
                        project_id: project_id.clone(),
                        version_id: version_id.clone(),
                        slug: None,
                    },
                },
            );
        }
    }

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
        .iter()
        .map(|m| {
            info!(
                "Encolando shaderpack: {} -> {:?}",
                m.filename,
                sp_dir.join(&m.filename)
            );
            DownloadItemSpec::new(m.url.clone(), sp_dir.join(&m.filename), "shaderpack")
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

    // Cache metadata via ablage
    for m in &packs {
        if let (Some(project_id), Some(version_id)) = (&m.project_id, &m.version_id) {
            crate::services::save_pack_cache(
                &sp_dir,
                &m.filename,
                &crate::services::PackCacheEntry {
                    source: crate::services::ModSource::Modrinth {
                        project_id: project_id.clone(),
                        version_id: version_id.clone(),
                        slug: None,
                    },
                },
            );
        }
    }

    info!(
        "{} shaderpacks descargados correctamente en {:?}",
        count, sp_dir
    );
    Ok(())
}

#[tauri::command]
pub async fn download_mrpack(url: String, version_id: String) -> Result<String, String> {
    info!("Downloading mrpack from {} (version: {})", url, version_id);

    let cache_dir = std::env::temp_dir()
        .join("cubiclauncher")
        .join("mrpack-cache");
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;

    let filename = format!("{}.mrpack", version_id);
    let dest = cache_dir.join(&filename);

    let item = aqua::DownloadItemSpec::new(url, dest.clone(), "mrpack");
    let batch = aqua::GenericBatch::new(format!("mrpack-{}", version_id), vec![item]);

    let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
    let dm = aqua::DownloadManager::new(shared_dir);
    let handle = dm
        .prepare_batch(Box::new(batch))
        .await
        .map_err(|e| format!("Failed to prepare mrpack download: {}", e))?;

    handle
        .download_all(None)
        .await
        .map_err(|e| format!("Failed to download mrpack: {}", e))?;

    let path_str = dest.to_string_lossy().to_string();
    info!("Mrpack downloaded to {}", path_str);
    Ok(path_str)
}
