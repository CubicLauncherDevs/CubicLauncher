use crate::core::errors::DownloadError;
use crate::core::{HTTP, PathManager};
use crate::services::DownloadQueue;
use aqua::{DownloadManager, FabricBatch};
use serde::{Deserialize, Serialize};
use tracing::info;

const MOJANG_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FORGE_MAVEN_METADATA_URL: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";

fn manifest_cache_path() -> std::path::PathBuf {
    PathManager::get().get_settings_dir().join("manifest.crep")
}

fn fabric_cache_path() -> std::path::PathBuf {
    PathManager::get().get_settings_dir().join("fabric.crep")
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ForgeGameVersion {
    pub version_id: String,
    pub game_version: String,
    pub forge_version: String,
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

    let cache_path = manifest_cache_path();
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
    let cache_path = manifest_cache_path();
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
    let cache_path = fabric_cache_path();
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
    let path = manifest_cache_path();

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

fn parse_maven_metadata(xml: &str) -> Vec<String> {
    let mut versions = Vec::new();
    let mut in_versions = false;
    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed == "<versions>" {
            in_versions = true;
            continue;
        }
        if trimmed == "</versions>" {
            in_versions = false;
            continue;
        }
        if in_versions
            && let Some(ver) = trimmed
                .strip_prefix("<version>")
                .and_then(|s| s.strip_suffix("</version>"))
        {
            versions.push(ver.to_string());
        }
    }
    versions
}

fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parts_a: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let parts_b: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
    parts_a.cmp(&parts_b)
}

fn group_forge_versions(all_versions: Vec<String>) -> Vec<ForgeGameVersion> {
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for v in all_versions {
        let Some(idx) = v.find('-') else {
            continue;
        };
        let mc_part = &v[..idx];
        let rest = &v[idx + 1..];
        if rest.is_empty() || mc_part.is_empty() {
            continue;
        }

        // Strip MC version suffix if present (e.g., "12.17.0.2317-1.9.4" → "12.17.0.2317")
        let suffix = format!("-{mc_part}");
        let forge_part = rest.strip_suffix(&suffix).unwrap_or(rest);

        if !forge_part.is_empty() {
            groups
                .entry(mc_part.to_string())
                .or_default()
                .push(forge_part.to_string());
        }
    }

    let mut result = Vec::new();
    for (mc_version, forge_versions) in groups.into_iter().rev() {
        let latest = forge_versions
            .into_iter()
            .max_by(|a, b| version_cmp(a, b))
            .unwrap_or_default();
        let version_id = format!("{mc_version}-forge-{latest}");
        result.push(ForgeGameVersion {
            version_id,
            game_version: mc_version,
            forge_version: latest,
        });
    }
    result
}

async fn fetch_forge_versions_from_maven() -> Result<Vec<ForgeGameVersion>, String> {
    let response = HTTP
        .get(FORGE_MAVEN_METADATA_URL)
        .send()
        .await
        .map_err(|e| DownloadError::Request(e.to_string()).to_string())?;

    let xml = response
        .text()
        .await
        .map_err(|e| DownloadError::ReadResponse(e.to_string()).to_string())?;

    let all_versions = parse_maven_metadata(&xml);
    info!(
        "Forge maven-metadata.xml parseado: {} versiones totales",
        all_versions.len()
    );
    Ok(group_forge_versions(all_versions))
}

fn get_forge_cache_path() -> std::path::PathBuf {
    PathManager::get().get_settings_dir().join("forge.crep")
}

async fn read_forge_cache() -> Option<Vec<ForgeGameVersion>> {
    let cache_path = get_forge_cache_path();
    tokio::task::spawn_blocking(move || {
        let repo = ablage::Repo::open(&cache_path);
        let entry = repo.get("forge")?;
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
    .unwrap_or(None)
}

async fn write_forge_cache(versions: &[ForgeGameVersion]) {
    let cache_path = get_forge_cache_path();
    let versions = versions.to_vec();
    tokio::task::spawn_blocking(move || {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut repo = ablage::Repo::open(&cache_path);
        if let Ok(data) = postcard::to_stdvec(&versions) {
            repo.put(
                "forge",
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
}

#[tauri::command]
pub async fn get_forge_versions() -> Result<Vec<ForgeGameVersion>, String> {
    if let Some(versions) = read_forge_cache().await {
        info!(
            "Usando cache de versiones de Forge ({} versiones)",
            versions.len()
        );
        return Ok(versions);
    }

    info!("Cache de Forge ausente, descargando desde maven.minecraftforge.net");
    let versions = fetch_forge_versions_from_maven().await?;
    write_forge_cache(&versions).await;
    Ok(versions)
}

#[tauri::command]
pub async fn refresh_forge_versions() -> Result<Vec<ForgeGameVersion>, String> {
    info!("Forzando actualizacion de versiones de Forge");
    let cache_path = get_forge_cache_path();
    tokio::task::spawn_blocking(move || {
        let mut repo = ablage::Repo::open(&cache_path);
        repo.remove("forge");
        let _ = repo.flush();
    })
    .await
    .ok();

    let versions = fetch_forge_versions_from_maven().await?;
    write_forge_cache(&versions).await;
    Ok(versions)
}
