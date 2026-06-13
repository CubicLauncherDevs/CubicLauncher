use crate::core::errors::DownloadError;
use crate::core::{HTTP, PathManager};
use crate::services::DownloadQueue;
use aqua::{DownloadManager, FabricBatch};
use serde::{Deserialize, Serialize};
use tracing::info;

const MOJANG_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MinecraftManifest {
    pub latest: LatestVersions,
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FabricGameVersion {
    pub version: String,
    pub stable: bool,
}

#[tauri::command]
pub async fn add_to_queue(version: String) {
    DownloadQueue::get().enqueue(version).await
}

pub async fn download_manifest() -> Result<Vec<MinecraftVersion>, String> {
    info!("Descargando manifiesto de versiones desde Mojang");
    let response = HTTP
        .get(MOJANG_MANIFEST_URL)
        .send()
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| DownloadError::ReadResponse(e.to_string()).to_string())?;

    info!(
        "Manifiesto descargado ({} bytes), cacheando en disco",
        bytes.len()
    );

    let manifest: MinecraftManifest = serde_json::from_slice(&bytes)
        .map_err(|e| DownloadError::ParseJson(e.to_string()).to_string())?;

    info!(
        "Manifiesto parseado: {} versiones disponibles",
        manifest.versions.len()
    );

    let cache_path = PathManager::get().get_settings_dir().join("versions.crep");
    let mut repo = ablage::Repo::open(&cache_path);
    if let Ok(data) = postcard::to_stdvec(&manifest) {
        repo.put(
            "manifest",
            ablage::Entry {
                version: 1,
                fingerprint: 0,
                data,
            },
        );
        let _ = repo.flush();
    }

    Ok(manifest.versions)
}

#[tauri::command]
pub async fn get_available_versions() -> Result<Vec<MinecraftVersion>, String> {
    let cache_path = PathManager::get().get_settings_dir().join("versions.crep");
    let repo = ablage::Repo::open(&cache_path);

    if let Some(entry) = repo.get("manifest")
        && let Ok(manifest) = postcard::from_bytes::<MinecraftManifest>(&entry.data)
    {
        info!("{} versiones cargadas desde cache", manifest.versions.len());
        return Ok(manifest.versions);
    }

    info!("No hay cache de manifiesto, descargando");
    download_manifest().await
}

#[tauri::command]
pub async fn get_fabric_versions() -> Result<Vec<FabricGameVersion>, String> {
    let cache_path = PathManager::get().get_settings_dir().join("versions.crep");
    let cache_path2 = cache_path.clone();

    let cached = tokio::task::spawn_blocking(move || -> Option<Vec<FabricGameVersion>> {
        let repo = ablage::Repo::open(&cache_path);
        let entry = repo.get("fabric")?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let cached_time = entry.fingerprint;
        if now.saturating_sub(cached_time) > 3600 {
            return None;
        }
        postcard::from_bytes(&entry.data).ok()
    })
    .await
    .unwrap_or(None);

    if let Some(versions) = cached {
        info!(
            "Usando cache de versiones de Fabric ({} versiones)",
            versions.len()
        );
        return Ok(versions);
    }

    info!("Cache de Fabric expirado o ausente, descargando desde meta.fabricmc.net");
    let url = "https://meta.fabricmc.net/v2/versions/game";
    let response = HTTP
        .get(url)
        .send()
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    let versions = response
        .json::<Vec<FabricGameVersion>>()
        .await
        .map_err(|e| DownloadError::ParseJson(e.to_string()).to_string())?;

    info!(
        "{} versiones de Fabric obtenidas, cacheando en disco",
        versions.len()
    );

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let write_versions = versions.clone();
    tokio::task::spawn_blocking(move || {
        let mut repo = ablage::Repo::open(&cache_path2);
        if let Ok(data) = postcard::to_stdvec(&write_versions) {
            repo.put(
                "fabric",
                ablage::Entry {
                    version: 1,
                    fingerprint: now,
                    data,
                },
            );
            let _ = repo.flush();
        }
    })
    .await
    .ok();

    Ok(versions)
}

#[tauri::command]
pub async fn download_fabric(
    game_version: String,
    loader_version: Option<String>,
) -> Result<(), String> {
    info!(
        "Iniciando descarga de Fabric para Minecraft {}",
        game_version
    );

    let loader_version = if let Some(specific) = loader_version {
        specific
    } else {
        FabricBatch::resolve_latest_loader(&game_version)
            .await
            .map_err(|e| DownloadError::Request(e.to_string()).to_string())?
    };

    let fabric_version_id = format!("fabric-loader-{}-{}", loader_version, game_version);
    info!("Loader: {}, ID: {}", loader_version, fabric_version_id);

    let shared_dir = PathManager::get().get_shared_dir();
    let json_path = shared_dir
        .join("versions")
        .join(&fabric_version_id)
        .join(format!("{}.json", fabric_version_id));

    if tokio::fs::try_exists(&json_path).await.unwrap_or(false) {
        info!(
            "Fabric {} ya instalado, encolando assets",
            fabric_version_id
        );
        DownloadQueue::get().enqueue(fabric_version_id).await;
        return Ok(());
    }

    let batch = FabricBatch::new(shared_dir, &game_version, &loader_version)
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    let manager = DownloadManager::new(shared_dir.to_path_buf());
    let handle = manager
        .prepare_batch(Box::new(batch))
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    handle
        .download_all(None)
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    info!("Fabric {} descargado correctamente", fabric_version_id);
    DownloadQueue::get().enqueue(fabric_version_id).await;

    Ok(())
}
#[tauri::command]
pub async fn refresh_versions() -> Result<Vec<MinecraftVersion>, String> {
    info!("Forzando actualizacion del manifiesto de versiones");
    let path = PathManager::get().get_settings_dir().join("versions.crep");

    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| format!("Error al eliminar cache: {}", e))?;
        info!("Cache de manifiesto eliminado: {:?}", path);
    }

    download_manifest().await
}

#[tauri::command]
pub async fn get_download_queue() -> Vec<crate::services::DownloadState> {
    crate::services::DownloadQueue::get()
        .get_active_downloads()
        .await
}
