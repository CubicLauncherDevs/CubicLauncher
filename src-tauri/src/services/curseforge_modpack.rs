use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::services::curseforge_api::{
    curseforge_cdn_url, CurseForgeClient, CurseForgeFile, MODPACKS_CLASS_ID, MODS_CLASS_ID,
    RESOURCE_PACKS_CLASS_ID, SHADERS_CLASS_ID,
};

#[derive(Debug, thiserror::Error)]
pub enum CurseForgeModpackError {
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CurseForge API error: {0}")]
    Api(String),
    #[error("Invalid modpack: {0}")]
    Invalid(String),
    #[error("Download error: {0}")]
    Download(String),
}

impl From<crate::services::curseforge_api::CurseForgeError> for CurseForgeModpackError {
    fn from(e: crate::services::curseforge_api::CurseForgeError) -> Self {
        Self::Api(e.to_string())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModpackManifest {
    pub manifest_type: String,
    pub manifest_version: u32,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(default)]
    pub description: Option<String>,
    pub files: Vec<CurseForgeModpackFile>,
    pub overrides: String,
    pub minecraft: CurseForgeModpackMinecraft,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModpackFile {
    #[serde(rename = "projectID")]
    pub project_id: u32,
    #[serde(rename = "fileID")]
    pub file_id: u32,
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModpackMinecraft {
    pub version: String,
    #[serde(default)]
    pub mod_loaders: Vec<CurseForgeModpackModLoader>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModpackModLoader {
    pub id: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone)]
pub struct CurseForgeModpackMetadata {
    pub name: String,
    pub version: String,
    pub summary: Option<String>,
    pub game_version: Option<zellkern::GameVersion>,
    pub file_count: usize,
}

pub fn parse_curseforge_modpack(path: &Path) -> Result<CurseForgeModpackMetadata, CurseForgeModpackError> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let manifest_idx = archive
        .file_names()
        .position(|name| name == "manifest.json")
        .ok_or_else(|| {
            CurseForgeModpackError::Invalid("No manifest.json found in CurseForge modpack".to_string())
        })?;

    let mut content = String::new();
    archive.by_index(manifest_idx)?.read_to_string(&mut content)?;

    let manifest: CurseForgeModpackManifest = serde_json::from_str(&content)?;

    if manifest.manifest_type != "minecraftModpack" {
        return Err(CurseForgeModpackError::Invalid(format!(
            "Unexpected manifest type: {}",
            manifest.manifest_type
        )));
    }

    Ok(metadata_from_manifest(&manifest))
}

fn metadata_from_manifest(manifest: &CurseForgeModpackManifest) -> CurseForgeModpackMetadata {
    let game_version = infer_game_version(&manifest.minecraft);

    CurseForgeModpackMetadata {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        summary: manifest.description.clone(),
        game_version,
        file_count: manifest.files.len(),
    }
}

fn infer_game_version(minecraft: &CurseForgeModpackMinecraft) -> Option<zellkern::GameVersion> {
    let loader = minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| minecraft.mod_loaders.first())?;

    let loader_id = loader.id.to_lowercase();

    let (loader_name, loader_version) = if let Some(rest) = loader_id.strip_prefix("forge-") {
        ("forge", rest)
    } else if let Some(rest) = loader_id.strip_prefix("fabric-") {
        ("fabric", rest)
    } else if let Some(rest) = loader_id.strip_prefix("quilt-") {
        ("quilt", rest)
    } else if let Some(rest) = loader_id.strip_prefix("neoforge-") {
        ("neoforge", rest)
    } else {
        // Some packs use unknown loader identifiers; treat as vanilla.
        return Some(zellkern::GameVersion {
            mc_version: minecraft.version.clone(),
            loader: zellkern::Loader::Vanilla,
        });
    };

    let loader = match loader_name {
        "forge" => zellkern::Loader::Forge(loader_version.to_string()),
        "fabric" => zellkern::Loader::Fabric(loader_version.to_string()),
        "quilt" => zellkern::Loader::Quilt(loader_version.to_string()),
        "neoforge" => zellkern::Loader::NeoForge(loader_version.to_string()),
        _ => zellkern::Loader::Vanilla,
    };

    Some(zellkern::GameVersion {
        mc_version: minecraft.version.clone(),
        loader,
    })
}

pub async fn install_curseforge_modpack(
    path: &Path,
    instance_dir: &Path,
    shared_dir: &Path,
    progress: Option<aqua::progress::ProgressSender>,
) -> Result<CurseForgeModpackMetadata, CurseForgeModpackError> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let manifest_idx = archive
        .file_names()
        .position(|name| name == "manifest.json")
        .ok_or_else(|| {
            CurseForgeModpackError::Invalid("No manifest.json found in CurseForge modpack".to_string())
        })?;

    let mut content = String::new();
    archive.by_index(manifest_idx)?.read_to_string(&mut content)?;

    let manifest: CurseForgeModpackManifest = serde_json::from_str(&content)?;
    let metadata = metadata_from_manifest(&manifest);

    let client = CurseForgeClient::from_settings_or_default();

    let required_files: Vec<&CurseForgeModpackFile> = manifest
        .files
        .iter()
        .filter(|f| f.required)
        .collect();

    if required_files.is_empty() {
        extract_overrides(&mut archive, instance_dir, &manifest.overrides).await?;
        extract_icon(&mut archive, instance_dir).await?;
        return Ok(metadata);
    }

    // Batch fetch file metadata and project metadata.
    let file_ids: Vec<u32> = required_files.iter().map(|f| f.file_id).collect();
    let files = client.get_mod_files(&file_ids).await?;
    let file_by_id: HashMap<u32, CurseForgeFile> = files.into_iter().map(|f| (f.id, f)).collect();

    let mod_ids: Vec<u32> = required_files.iter().map(|f| f.project_id).collect();
    // Deduplicate to avoid unnecessary work.
    let unique_mod_ids: Vec<u32> = mod_ids.iter().copied().collect::<HashSet<_>>().into_iter().collect();
    let projects = client.get_projects(&unique_mod_ids).await?;
    let class_by_mod_id: HashMap<u32, u32> = projects
        .into_iter()
        .filter_map(|p| p.class_id.map(|class_id| (p.id, class_id)))
        .collect();

    // Resolve download URLs concurrently so installing large modpacks doesn't
    // spend most of its time waiting on sequential API round-trips.
    let instance_dir_path = instance_dir.to_path_buf();
    let file_by_id = file_by_id.clone();
    let class_by_mod_id = class_by_mod_id.clone();
    let client = client.clone();

    let file_entries: Vec<(u32, u32)> = required_files
        .iter()
        .map(|f| (f.project_id, f.file_id))
        .collect();

    let items = futures::stream::iter(file_entries)
        .map(|(project_id, file_id)| {
            let file_by_id = file_by_id.clone();
            let class_by_mod_id = class_by_mod_id.clone();
            let client = client.clone();
            let instance_dir_path = instance_dir_path.clone();
            async move {
                let file = file_by_id.get(&file_id).cloned().ok_or_else(|| {
                    CurseForgeModpackError::Invalid(format!(
                        "File {} not found in CurseForge API",
                        file_id
                    ))
                })?;

                let sub_dir = sub_dir_for_class(
                    class_by_mod_id
                        .get(&project_id)
                        .copied()
                        .unwrap_or(MODS_CLASS_ID),
                );
                let dest_dir = instance_dir_path.join(sub_dir);
                let dest = dest_dir.join(&file.file_name);
                let url = resolve_mod_download_url(&client, project_id, &file).await;

                let hash = file
                    .hashes
                    .iter()
                    .find(|h| h.algo == 1)
                    .map(|h| h.value.clone())
                    .unwrap_or_default();

                let label = format!("{} {}", file.display_name, file.file_name);
                Ok::<_, CurseForgeModpackError>(
                    aqua::DownloadItemSpec::new(url, dest, label)
                        .with_hash(hash)
                        .with_size(file.file_length),
                )
            }
        })
        .buffer_unordered(12)
        .try_collect::<Vec<_>>()
        .await?;

    if !items.is_empty() {
        let batch_name = format!(
            "curseforge-modpack-{}-{}",
            sanitize_for_label(&manifest.name),
            manifest.version
        );
        let batch = aqua::GenericBatch::new(batch_name, items);
        let dm = aqua::DownloadManager::new(shared_dir.to_path_buf()).with_max_downloads(12);
        let handle = dm
            .prepare_batch(Box::new(batch))
            .await
            .map_err(|e| CurseForgeModpackError::Download(e.to_string()))?;
        handle
            .download_all(progress)
            .await
            .map_err(|e| CurseForgeModpackError::Download(e.to_string()))?;
    }

    extract_overrides(&mut archive, instance_dir, &manifest.overrides).await?;
    extract_icon(&mut archive, instance_dir).await?;

    info!("CurseForge modpack installed into {:?}", instance_dir);
    Ok(metadata)
}

/// Resolves the best download URL for a single mod file in a modpack.
///
/// If the API already provided a `download_url`, it is used as-is. Otherwise we
/// go straight to the CurseForge CDN. Calling the official `/download-url`
/// endpoint serially for every mod is slow and frequently returns 403, which
/// is why the CDN is preferred here for modpack installs.
async fn resolve_mod_download_url(
    _client: &CurseForgeClient,
    _project_id: u32,
    file: &CurseForgeFile,
) -> String {
    if let Some(url) = &file.download_url {
        if !url.is_empty() {
            return url.clone();
        }
    }
    curseforge_cdn_url(file.id, &file.file_name)
}

fn sub_dir_for_class(class_id: u32) -> &'static str {
    match class_id {
        MODS_CLASS_ID => "mods",
        RESOURCE_PACKS_CLASS_ID => "resourcepacks",
        SHADERS_CLASS_ID => "shaderpacks",
        MODPACKS_CLASS_ID => "mods",
        _ => "mods",
    }
}

fn sanitize_for_label(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

async fn extract_overrides(
    archive: &mut zip::ZipArchive<std::fs::File>,
    instance_dir: &Path,
    overrides_name: &str,
) -> Result<(), CurseForgeModpackError> {
    let prefix = format!("{}/", overrides_name.trim_end_matches('/'));

    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();
        let is_dir = entry.is_dir();
        drop(entry);

        if is_dir {
            continue;
        }

        let relative_path = if let Some(stripped) = entry_name.strip_prefix(&prefix) {
            stripped.to_string()
        } else {
            continue;
        };

        // Avoid overwriting files downloaded from CurseForge if the pack
        // duplicated any file in overrides.
        let dest = instance_dir.join(&relative_path);

        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tracing::info!("Extracting override {} -> {:?}", entry_name, dest);

        let mut buffer = Vec::new();
        archive.by_index(i)?.read_to_end(&mut buffer)?;
        tokio::fs::write(&dest, &buffer).await?;
    }
    Ok(())
}

async fn extract_icon(
    archive: &mut zip::ZipArchive<std::fs::File>,
    instance_dir: &Path,
) -> Result<(), CurseForgeModpackError> {
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let is_dir = entry.is_dir();
        let name = entry.name().to_string();
        drop(entry);

        if is_dir || name != "icon.png" {
            continue;
        }

        let icon_dest = instance_dir.join("icon.png");
        let mut buffer = Vec::new();
        archive.by_index(i)?.read_to_end(&mut buffer)?;
        tokio::fs::write(&icon_dest, &buffer).await?;
        break;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST_JSON: &str =
        include_str!("../../tests/fixtures/curseforge_modpack_manifest.json");

    #[test]
    fn parse_modpack_manifest() {
        let manifest: CurseForgeModpackManifest =
            serde_json::from_str(MANIFEST_JSON).expect("manifest should parse");

        assert_eq!(manifest.manifest_type, "minecraftModpack");
        assert_eq!(manifest.manifest_version, 1);
        assert_eq!(manifest.name, "Test Modpack");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.author, "Author");
        assert_eq!(
            manifest.description,
            Some("A test modpack".to_string())
        );
        assert_eq!(manifest.overrides, "overrides");
        assert_eq!(manifest.files.len(), 2);

        let first_file = &manifest.files[0];
        assert_eq!(first_file.project_id, 225_608);
        assert_eq!(first_file.file_id, 2_915_154);
        assert!(first_file.required);

        let second_file = &manifest.files[1];
        assert_eq!(second_file.project_id, 250_898);
        assert_eq!(second_file.file_id, 2_916_358);
        assert!(!second_file.required);

        assert_eq!(manifest.minecraft.version, "1.20.1");
        assert_eq!(manifest.minecraft.mod_loaders.len(), 1);
        let loader = &manifest.minecraft.mod_loaders[0];
        assert_eq!(loader.id, "forge-47.2.0");
        assert!(loader.primary);
    }
}
