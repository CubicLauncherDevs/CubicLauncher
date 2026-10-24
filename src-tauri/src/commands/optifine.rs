use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use aqua::optifine::{OptiFineVersion, fetch_optifine_versions};
use tokio::sync::Mutex;

use crate::core::errors::{DownloadError, InstanceError};
use crate::services::DownloadQueue;

struct CatalogCache {
    fetched_at: Instant,
    versions: Arc<Vec<OptiFineVersion>>,
}

static CATALOG: LazyLock<Mutex<Option<CatalogCache>>> = LazyLock::new(|| Mutex::new(None));

async fn catalog(refresh: bool) -> Result<Arc<Vec<OptiFineVersion>>, String> {
    let mut cache = CATALOG.lock().await;
    if !refresh
        && let Some(cached) = &*cache
        && cached.fetched_at.elapsed() < Duration::from_secs(3600)
    {
        return Ok(cached.versions.clone());
    }
    let versions = Arc::new(
        fetch_optifine_versions()
            .await
            .map_err(|e| String::from(DownloadError::Request(e.to_string())))?,
    );
    *cache = Some(CatalogCache {
        fetched_at: Instant::now(),
        versions: versions.clone(),
    });
    Ok(versions)
}

#[tauri::command]
pub async fn get_optifine_versions() -> Result<Vec<OptiFineVersion>, String> {
    catalog(false).await.map(|versions| (*versions).clone())
}

#[tauri::command]
pub async fn refresh_optifine_versions() -> Result<Vec<OptiFineVersion>, String> {
    catalog(true).await.map(|versions| (*versions).clone())
}

pub(crate) async fn resolve_version(version_id: &str) -> Result<OptiFineVersion, String> {
    catalog(false)
        .await?
        .iter()
        .find(|v| v.version_id == version_id)
        .cloned()
        .ok_or_else(|| String::from(InstanceError::VersionNotFound(version_id.into())))
}

#[tauri::command]
pub async fn download_optifine(
    game_version: String,
    optifine_version: String,
) -> Result<(), String> {
    let version = resolve_version(&format!("{game_version}-OptiFine_{optifine_version}")).await?;
    DownloadQueue::get().enqueue(version.version_id).await;
    Ok(())
}
