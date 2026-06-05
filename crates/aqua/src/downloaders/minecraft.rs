use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use log::info;
use uuid::Uuid;

use super::batch::{DownloadBatch, DownloadItemSpec};
use crate::manifest::{resolve_asset_index, resolve_version_data};
use crate::natives::natives_subdir;
use crate::types::{AssetMeta, NormalizedVersion, RESOURCES_BASE_URL};
use crate::utilities::download_file;
use crate::ProtonError;
use crate::utilities::HTTP_CLIENT;

#[derive(Clone)]
struct DirPaths {
    natives_dir: PathBuf,
    objects_dir: PathBuf,
    libraries_dir: PathBuf,
    versions_dir: PathBuf,
    assets_indexes_dir: PathBuf,
}

fn compute_dirs(game_path: &Path, version_id: &str, version: &crate::types::MCVersion) -> DirPaths {
    let sub = natives_subdir(version);
    DirPaths {
        natives_dir: game_path.join("natives").join(version_id).join(sub),
        objects_dir: game_path.join("assets").join("objects"),
        libraries_dir: game_path.join("libraries"),
        versions_dir: game_path.join("versions").join(version_id),
        assets_indexes_dir: game_path.join("assets").join("indexes"),
    }
}

pub struct MinecraftBatch {
    version: NormalizedVersion,
    dirs: DirPaths,
    temp_dir: PathBuf,
    items: Vec<DownloadItemSpec>,
}

impl MinecraftBatch {
    pub async fn new(game_path: &Path, version_id: &str) -> Result<Self, ProtonError> {
        let version = resolve_version_data(version_id).await?;
        let mc_version = &version.parsed_version;
        let dirs = compute_dirs(game_path, version_id, mc_version);

        let temp_dir = dirs
            .natives_dir
            .parent()
            .unwrap_or(game_path)
            .join("temp")
            .join(Uuid::new_v4().to_string());

        let mut items: Vec<DownloadItemSpec> = Vec::new();

        for lib in &version.libraries {
            let lib_path = dirs.libraries_dir.join(&lib.path);
            items.push(
                DownloadItemSpec::new(&lib.url, lib_path, &lib.name).with_hash(&lib.sha1),
            );
        }

        for native in &version.natives {
            let filename = native
                .path
                .split('/')
                .next_back()
                .unwrap_or(&native.path)
                .to_string();
            let temp_path = temp_dir.join(&filename);
            items.push(
                DownloadItemSpec::new(&native.url, temp_path, &native.name).with_hash(&native.sha1),
            );
        }

        let asset_index = resolve_asset_index(&version).await?;
        for (name, asset) in asset_index.into_vec() {
            let hash = asset.hash;
            let subhash: String = hash.chars().take(2).collect();
            let url = format!("{}/{}/{}", RESOURCES_BASE_URL, subhash, hash);
            let path = dirs.objects_dir.join(&subhash).join(&hash);
            items.push(DownloadItemSpec::new(url, path, name).with_hash(hash));
        }

        if !version.client_jar.url.is_empty() {
            let client_path = dirs.versions_dir.join(format!("{}.jar", version.id));
            items.push(
                DownloadItemSpec::new(
                    &version.client_jar.url,
                    client_path,
                    format!("minecraft-{}", version.id),
                )
                .with_hash(&version.client_jar.sha1),
            );
        }

        Ok(Self {
            version,
            dirs,
            temp_dir,
            items,
        })
    }

    pub fn version(&self) -> &NormalizedVersion {
        &self.version
    }
}

impl DownloadBatch for MinecraftBatch {
    fn name(&self) -> String {
        self.version.id.clone()
    }

    fn items(&self) -> &[DownloadItemSpec] {
        &self.items
    }

    fn prepare(&self) -> Pin<Box<dyn Future<Output = Result<(), ProtonError>> + Send + '_>> {
        let dirs = self.dirs.clone();
        let version_id = self.version.id.clone();
        let asset_index = self.version.asset_index.clone();
        let temp_dir = self.temp_dir.clone();
        Box::pin(async move {
            tokio::fs::create_dir_all(&dirs.natives_dir).await?;
            tokio::fs::create_dir_all(&dirs.objects_dir).await?;
            tokio::fs::create_dir_all(&dirs.libraries_dir).await?;
            tokio::fs::create_dir_all(&dirs.versions_dir).await?;
            tokio::fs::create_dir_all(&dirs.assets_indexes_dir).await?;
            tokio::fs::create_dir_all(&temp_dir).await?;

            download_version_json(&version_id, &dirs).await?;
            download_asset_index_json(&asset_index, &dirs).await?;

            Ok(())
        })
    }

    fn finalize(&self) -> Pin<Box<dyn Future<Output = Result<(), ProtonError>> + Send + '_>> {
        let temp_dir = self.temp_dir.clone();
        let natives_dir = self.dirs.natives_dir.clone();
        let jar_paths: Vec<PathBuf> = self
            .version
            .natives
            .iter()
            .map(|n| {
                let filename = n
                    .path
                    .split('/')
                    .next_back()
                    .unwrap_or(&n.path)
                    .to_string();
                temp_dir.join(&filename)
            })
            .collect();

        Box::pin(async move {
            #[cfg(feature = "extract-natives")]
            {
                use futures::stream::{FuturesUnordered, StreamExt};
                use tokio::sync::Semaphore;
                use std::sync::Arc;
                use crate::utilities::extract_native;

                const MAX_CONCURRENT_EXTRACTIONS: usize = 8;
                let extract_sem = Arc::new(Semaphore::new(MAX_CONCURRENT_EXTRACTIONS));
                let mut ext_tasks = FuturesUnordered::new();

                for jar_path in &jar_paths {
                    let sem = Arc::clone(&extract_sem);
                    let dest = natives_dir.clone();
                    let jp = jar_path.clone();
                    ext_tasks.push(tokio::spawn(async move {
                        let _p = sem.acquire_owned().await;
                        extract_native(&jp, &dest).await
                    }));
                }

                while let Some(res) = ext_tasks.next().await {
                    res??;
                }

                tokio::fs::remove_dir_all(&temp_dir).await?;
            }

            #[cfg(not(feature = "extract-natives"))]
            {
                let _ = jar_paths;
                let _ = natives_dir;
            }

            Ok(())
        })
    }
}

async fn download_version_json(
    version_id: &str,
    dirs: &DirPaths,
) -> Result<(), ProtonError> {
    let path = dirs.versions_dir.join(format!("{}.json", version_id));
    if path.exists() {
        return Ok(());
    }

    let v2_url = crate::types::MOJANG_MANIFEST_URL;
    let v2: serde_json::Value = HTTP_CLIENT.get(v2_url).send().await?.json().await?;
    let entry = v2["versions"]
        .as_array()
        .and_then(|arr| arr.iter().find(|v| v["id"] == version_id))
        .ok_or_else(|| ProtonError::VersionNotFound(version_id.to_string()))?;
    let detail_url = entry["url"]
        .as_str()
        .ok_or_else(|| ProtonError::Other("No URL in manifest".into()))?;

    let detail = HTTP_CLIENT.get(detail_url).send().await?.text().await?;
    tokio::fs::write(&path, detail).await?;
    info!("Saved version JSON: {:?}", path);
    Ok(())
}

async fn download_asset_index_json(
    asset_index: &AssetMeta,
    dirs: &DirPaths,
) -> Result<(), ProtonError> {
    let path = dirs
        .assets_indexes_dir
        .join(format!("{}.json", asset_index.id));
    if path.exists() {
        let ok = crate::utilities::verify_file_hash(&path, &asset_index.sha1)
            .await
            .unwrap_or(false);
        if ok {
            return Ok(());
        }
    }

    download_file(
        &asset_index.url,
        &path,
        &asset_index.sha1,
    )
    .await?;
    Ok(())
}
