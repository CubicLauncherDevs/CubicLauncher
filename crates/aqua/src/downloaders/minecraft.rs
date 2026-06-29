use super::batch::{DownloadBatch, DownloadItemSpec};
use crate::AquaError;
use crate::manifest::{resolve_asset_index, resolve_version_data};
use crate::types::{DownloadProgress, NormalizedVersion, RESOURCES_BASE_URL};
use log::info;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::sync::mpsc::Sender;
use zellkern::resolvers::natives_subdir;

#[derive(Clone)]
struct DirPaths {
    natives_dir: PathBuf,
    objects_dir: PathBuf,
    libraries_dir: PathBuf,
    versions_dir: PathBuf,
    assets_indexes_dir: PathBuf,
}

fn compute_dirs(game_path: &Path, version_id: &str, version: &zellkern::MCVersion) -> DirPaths {
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
    items: Vec<DownloadItemSpec>,
    version_json_bytes: Vec<u8>,
    asset_index_bytes: Vec<u8>,
}

impl MinecraftBatch {
    pub async fn new(game_path: &Path, version_id: &str) -> Result<Self, AquaError> {
        let (version, version_json_bytes) = resolve_version_data(version_id).await?;
        let mc_version = &version.parsed_version;
        let dirs = compute_dirs(game_path, version_id, mc_version);

        let mut items: Vec<DownloadItemSpec> = Vec::new();

        for lib in &version.libraries {
            let lib_path = dirs.libraries_dir.join(&lib.path);
            items.push(DownloadItemSpec::new(&lib.url, lib_path, &lib.name).with_hash(&lib.sha1));
        }

        for native in &version.natives {
            let native_path = dirs.libraries_dir.join(&native.path);
            items.push(
                DownloadItemSpec::new(&native.url, native_path, &native.name)
                    .with_hash(&native.sha1),
            );
        }

        let (asset_index, asset_index_bytes) = resolve_asset_index(&version).await?;
        for (name, asset) in asset_index.into_vec() {
            let hash = asset.hash;
            let subhash = &hash[..2];
            let url = format!("{}{}/{}", RESOURCES_BASE_URL, subhash, hash);
            let path = dirs.objects_dir.join(subhash).join(&hash);
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
            items,
            version_json_bytes,
            asset_index_bytes,
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

    fn prepare(&self) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        let dirs = self.dirs.clone();
        let version_id = self.version.id.clone();
        let version_json_bytes = self.version_json_bytes.clone();
        let asset_index = self.version.asset_index.clone();
        let asset_index_bytes = self.asset_index_bytes.clone();
        Box::pin(async move {
            tokio::fs::create_dir_all(&dirs.natives_dir).await?;
            tokio::fs::create_dir_all(&dirs.objects_dir).await?;
            tokio::fs::create_dir_all(&dirs.libraries_dir).await?;
            tokio::fs::create_dir_all(&dirs.versions_dir).await?;
            tokio::fs::create_dir_all(&dirs.assets_indexes_dir).await?;

            // Write version JSON from cached bytes (avoids re-fetching)
            let vj_path = dirs.versions_dir.join(format!("{version_id}.json"));
            if !vj_path.exists() {
                tokio::fs::write(&vj_path, &version_json_bytes).await?;
                info!("Saved version JSON: {:?}", vj_path);
            }

            // Write asset index JSON from cached bytes (avoids re-fetching)
            let ai_path = dirs
                .assets_indexes_dir
                .join(format!("{}.json", asset_index.id));
            if !ai_path.exists() {
                tokio::fs::write(&ai_path, &asset_index_bytes).await?;
                info!("Saved asset index JSON: {:?}", ai_path);
            }

            Ok(())
        })
    }

    fn finalize(
        &self,
        _progress_tx: Option<Sender<DownloadProgress>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        let natives_dir = self.dirs.natives_dir.clone();
        let jar_paths: Vec<PathBuf> = self
            .version
            .natives
            .iter()
            .map(|n| self.dirs.libraries_dir.join(&n.path))
            .collect();

        Box::pin(async move {
            #[cfg(feature = "extract-natives")]
            {
                use crate::utilities::extract_native;
                use futures::stream::{FuturesUnordered, StreamExt};
                use std::sync::Arc;
                use tokio::sync::Semaphore;

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
