use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::core::validate_filename;
use crate::services::curseforge_api::{
    CurseForgeClient, CurseForgeFile, MODPACKS_CLASS_ID, MODS_CLASS_ID, RESOURCE_PACKS_CLASS_ID,
    SHADERS_CLASS_ID, curseforge_cdn_url,
};
use zellkern::path_security::{ConfinedDir, archive_path, validate_component, validate_version};

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

pub fn parse_curseforge_modpack(
    path: &Path,
) -> Result<CurseForgeModpackMetadata, CurseForgeModpackError> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let manifest_idx = archive
        .file_names()
        .position(|name| name == "manifest.json")
        .ok_or_else(|| {
            CurseForgeModpackError::Invalid(
                "No manifest.json found in CurseForge modpack".to_string(),
            )
        })?;

    let mut content = String::new();
    archive
        .by_index(manifest_idx)?
        .read_to_string(&mut content)?;

    let manifest: CurseForgeModpackManifest = serde_json::from_str(&content)?;

    validate_manifest(&manifest)?;
    override_entries(&mut archive, &manifest.overrides)?;

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
        .or_else(|| minecraft.mod_loaders.first());
    let Some(loader) = loader else {
        return Some(zellkern::GameVersion {
            mc_version: minecraft.version.clone(),
            loader: zellkern::Loader::Vanilla,
        });
    };

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
    reuse_dir: Option<&Path>,
) -> Result<CurseForgeModpackMetadata, CurseForgeModpackError> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let manifest_idx = archive
        .file_names()
        .position(|name| name == "manifest.json")
        .ok_or_else(|| {
            CurseForgeModpackError::Invalid(
                "No manifest.json found in CurseForge modpack".to_string(),
            )
        })?;

    let mut content = String::new();
    archive
        .by_index(manifest_idx)?
        .read_to_string(&mut content)?;

    let manifest: CurseForgeModpackManifest = serde_json::from_str(&content)?;
    validate_manifest(&manifest)?;
    override_entries(&mut archive, &manifest.overrides)?;
    let metadata = metadata_from_manifest(&manifest);

    // Downloads and extraction write to a private staging subdir, never to the
    // target's own files. It lives on the target's filesystem so files that are
    // already installed can be hardlinked instead of re-downloaded, and TempDir
    // removes it on any failure.
    let target_dir = instance_dir;
    let destination = ConfinedDir::open(target_dir)?;
    let staging = tempfile::Builder::new()
        .prefix("cubic-curseforge-")
        .tempdir_in(target_dir)?;
    let instance_dir = staging.path();

    let client = CurseForgeClient::from_settings_or_default();

    let required_files: Vec<&CurseForgeModpackFile> =
        manifest.files.iter().filter(|f| f.required).collect();

    if required_files.is_empty() {
        extract_overrides(&mut archive, instance_dir, &manifest.overrides).await?;
        extract_icon(&mut archive, instance_dir).await?;
        publish_staging(instance_dir, &destination)?;
        return Ok(metadata);
    }

    // Batch fetch file metadata and project metadata.
    let file_ids: Vec<u32> = required_files.iter().map(|f| f.file_id).collect();
    let files = client.get_mod_files(&file_ids).await?;
    let file_by_id: HashMap<u32, CurseForgeFile> = files.into_iter().map(|f| (f.id, f)).collect();

    let mod_ids: Vec<u32> = required_files.iter().map(|f| f.project_id).collect();
    // Deduplicate to avoid unnecessary work.
    let unique_mod_ids: Vec<u32> = mod_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
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

                validate_filename(&file.file_name).map_err(|e| {
                    CurseForgeModpackError::Invalid(format!("Invalid file_name in modpack: {}", e))
                })?;
                validate_component(&file.file_name)?;

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

    // Reuse files already present in the target so the downloader skips them.
    // Best-effort: any failure simply falls back to downloading the file.
    if let Some(reuse) = reuse_dir {
        for item in &items {
            let Ok(relative) = item.destination.strip_prefix(instance_dir) else {
                continue;
            };
            let source = reuse.join(relative);
            if !source.is_file() {
                continue;
            }
            if let Some(parent) = item.destination.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::hard_link(&source, &item.destination).is_err()
                && std::fs::copy(&source, &item.destination).is_err()
            {
                tracing::warn!(
                    "No se pudo reutilizar {}; se descargará",
                    item.destination.display()
                );
            }
        }
    }

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
    publish_staging(instance_dir, &destination)?;

    info!("CurseForge modpack installed into {:?}", target_dir);
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
    if let Some(url) = &file.download_url
        && !url.is_empty()
    {
        return url.clone();
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
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn validate_manifest(manifest: &CurseForgeModpackManifest) -> Result<(), CurseForgeModpackError> {
    if manifest.manifest_type != "minecraftModpack" || manifest.manifest_version != 1 {
        return Err(CurseForgeModpackError::Invalid(
            "Unsupported CurseForge manifest".into(),
        ));
    }
    validate_component(&manifest.overrides)?;
    validate_version(&manifest.minecraft.version)?;
    for loader in &manifest.minecraft.mod_loaders {
        validate_version(&loader.id)?;
        for prefix in ["forge-", "neoforge-", "fabric-", "quilt-"] {
            if let Some(version) = loader.id.to_lowercase().strip_prefix(prefix) {
                validate_version(version)?;
            }
        }
    }
    if let Some(version) = infer_game_version(&manifest.minecraft) {
        validate_version(&version.to_version_id())?;
    }
    Ok(())
}

fn validate_override_destination(path: &Path) -> Result<(), CurseForgeModpackError> {
    let first = path
        .iter()
        .next()
        .and_then(|p| p.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        first.as_str(),
        "instance.cub" | "upstream.json" | "cubic-manifest.json" | "cubic-jar-cache"
    ) || first.starts_with(".cubic-")
    {
        return Err(CurseForgeModpackError::Invalid(format!(
            "Reserved launcher path: {}",
            path.display()
        )));
    }
    Ok(())
}

fn override_entries(
    archive: &mut zip::ZipArchive<std::fs::File>,
    overrides_name: &str,
) -> Result<Vec<(usize, PathBuf)>, CurseForgeModpackError> {
    validate_component(overrides_name)?;
    let mut entries = Vec::new();
    let mut names = HashSet::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let path = archive_path(entry.name())?;
        if entry.is_symlink() {
            return Err(CurseForgeModpackError::Invalid(
                "ZIP symlinks are not allowed".into(),
            ));
        }
        if let Ok(relative) = path.strip_prefix(overrides_name) {
            if relative.as_os_str().is_empty() {
                continue;
            }
            validate_override_destination(relative)?;
            if !entry.is_dir() {
                if !names.insert(relative.to_string_lossy().to_lowercase()) {
                    return Err(CurseForgeModpackError::Invalid(
                        "Duplicate override path".into(),
                    ));
                }
                entries.push((i, relative.to_path_buf()));
            }
        }
    }
    Ok(entries)
}

async fn extract_overrides(
    archive: &mut zip::ZipArchive<std::fs::File>,
    instance_dir: &Path,
    overrides_name: &str,
) -> Result<(), CurseForgeModpackError> {
    let entries = override_entries(archive, overrides_name)?;
    let root = ConfinedDir::open(instance_dir)?;
    for (i, relative) in entries {
        // Downloaded, verified files win over duplicates in overrides.
        if root.file_exists(&relative)? {
            continue;
        }
        root.copy_from(&relative, &mut archive.by_index(i)?)?;
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

        ConfinedDir::open(instance_dir)?
            .copy_from(Path::new("icon.png"), &mut archive.by_index(i)?)?;
        break;
    }
    Ok(())
}

fn publish_staging(
    staging: &Path,
    destination: &ConfinedDir,
) -> Result<(), CurseForgeModpackError> {
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        for entry in std::fs::read_dir(staging.join(&relative))? {
            let entry = entry?;
            let path = relative.join(entry.file_name());
            validate_override_destination(&path)?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                destination.copy_from(&path, &mut std::fs::File::open(entry.path())?)?;
            } else {
                return Err(CurseForgeModpackError::Invalid(
                    "Special file in modpack staging".into(),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/services/curseforge_modpack.rs"]
mod tests;
