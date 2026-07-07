use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use log::info;
use serde::Deserialize;

use super::batch::{DownloadBatch, DownloadItemSpec};
use crate::AquaError;
use crate::progress::DownloadStage;
use crate::utilities::HTTP_CLIENT;

#[derive(Deserialize)]
#[allow(dead_code)]
struct QuiltLoaderVersion {
    version: String,
}

#[derive(Deserialize)]
struct QuiltLoaderResponse {
    loader: QuiltLoaderVersion,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct QuiltProfile {
    id: String,
    libraries: Vec<QuiltLibrary>,
}

#[derive(Deserialize)]
struct QuiltLibrary {
    name: String,
    url: String,
}

pub struct QuiltBatch {
    quilt_version_id: String,
    game_version: String,
    loader_version: String,
    shared_dir: PathBuf,
    items: Vec<DownloadItemSpec>,
    profile_json: String,
}

impl QuiltBatch {
    pub async fn resolve_latest_loader(game_version: &str) -> Result<String, AquaError> {
        let loader_url = format!(
            "https://meta.quiltmc.org/v3/versions/loader/{}",
            game_version
        );
        let response = HTTP_CLIENT
            .get(&loader_url)
            .send()
            .await
            .map_err(|e| AquaError::Other(format!("Error fetching Quilt loaders: {}", e)))?;

        let loaders: Vec<QuiltLoaderResponse> = response
            .json()
            .await
            .map_err(|e| AquaError::Other(format!("Error parsing Quilt loaders: {}", e)))?;

        // Prefer stable releases (no -beta, -alpha suffix)
        let stable = loaders.iter().find(|r| !r.loader.version.contains('-'));
        match stable {
            Some(r) => Ok(r.loader.version.clone()),
            None => loaders
                .first()
                .map(|r| r.loader.version.clone())
                .ok_or_else(|| AquaError::Other("No Quilt loader found for this version".into())),
        }
    }

    pub async fn new(
        shared_dir: &Path,
        game_version: &str,
        loader_version: &str,
    ) -> Result<Self, AquaError> {
        let quilt_version_id = format!("quilt-loader-{}-{}", loader_version, game_version);

        let profile_url = format!(
            "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
            game_version, loader_version
        );

        let profile_text = HTTP_CLIENT
            .get(&profile_url)
            .send()
            .await
            .map_err(|e| AquaError::Other(format!("Error fetching Quilt profile: {}", e)))?
            .text()
            .await
            .map_err(|e| AquaError::Other(format!("Error reading Quilt profile: {}", e)))?;

        let profile: QuiltProfile = serde_json::from_str(&profile_text)
            .map_err(|e| AquaError::Other(format!("Error parsing Quilt profile: {}", e)))?;

        info!(
            "Quilt profile loaded: {} libraries for {}",
            profile.libraries.len(),
            quilt_version_id
        );

        let lib_base_dir = shared_dir.join("libraries");
        let mut items = Vec::with_capacity(profile.libraries.len());

        for lib in &profile.libraries {
            let parts: Vec<&str> = lib.name.split(':').collect();
            if parts.len() != 3 {
                continue;
            }

            let group = parts[0].replace('.', "/");
            let artifact = parts[1];
            let version = parts[2];

            let rel_path = format!(
                "{}/{}/{}/{}-{}.jar",
                group, artifact, version, artifact, version
            );
            let dest_path = lib_base_dir.join(&rel_path);
            let download_url = format!("{}{}", lib.url, rel_path);

            items.push(
                DownloadItemSpec::new(download_url, dest_path, &lib.name)
                    .with_stage(DownloadStage::Library),
            );
        }

        Ok(Self {
            quilt_version_id,
            game_version: game_version.to_string(),
            loader_version: loader_version.to_string(),
            shared_dir: shared_dir.to_path_buf(),
            items,
            profile_json: profile_text,
        })
    }

    pub fn quilt_version_id(&self) -> &str {
        &self.quilt_version_id
    }

    pub fn game_version(&self) -> &str {
        &self.game_version
    }

    pub fn loader_version(&self) -> &str {
        &self.loader_version
    }
}

impl DownloadBatch for QuiltBatch {
    fn name(&self) -> String {
        self.quilt_version_id.clone()
    }

    fn items(&self) -> &[DownloadItemSpec] {
        &self.items
    }

    fn prepare(&self) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        let version_dir = self
            .shared_dir
            .join("versions")
            .join(&self.quilt_version_id);
        let json_path = version_dir.join(format!("{}.json", self.quilt_version_id));
        let json = self.profile_json.clone();

        Box::pin(async move {
            if json_path.exists() {
                return Ok(());
            }
            tokio::fs::create_dir_all(&version_dir)
                .await
                .map_err(|e| AquaError::Other(format!("Error creating version dir: {}", e)))?;
            tokio::fs::write(&json_path, &json)
                .await
                .map_err(|e| AquaError::Other(format!("Error saving profile JSON: {}", e)))?;
            info!("Saved Quilt profile JSON: {:?}", json_path);
            Ok(())
        })
    }
}
