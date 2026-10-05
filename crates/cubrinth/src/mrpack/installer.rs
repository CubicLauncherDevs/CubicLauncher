use std::io::Read;
use std::path::{Path, PathBuf};

use super::pack_format::{MrpackMetadata, PackFormat};
use crate::utils::path::safe_join;

#[derive(Debug, thiserror::Error)]
pub enum MrpackError {
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Download error: {0}")]
    Download(String),
    #[error("Invalid mrpack: {0}")]
    Invalid(String),
}

pub fn parse_mrpack(path: &Path) -> Result<MrpackMetadata, MrpackError> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let manifest_idx = archive
        .file_names()
        .position(|name| name == "modrinth.index.json")
        .ok_or_else(|| {
            MrpackError::Invalid("No modrinth.index.json found in mrpack".to_string())
        })?;

    let mut content = String::new();
    archive
        .by_index(manifest_idx)?
        .read_to_string(&mut content)?;

    let pack: PackFormat = serde_json::from_str(&content)?;

    pack.validate().map_err(MrpackError::Invalid)?;

    Ok(pack.extract_metadata())
}

pub async fn install_mrpack(
    path: &Path,
    instance_dir: &Path,
    shared_dir: &Path,
    progress: Option<aqua::progress::ProgressSender>,
) -> Result<MrpackMetadata, MrpackError> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let manifest_idx = archive
        .file_names()
        .position(|name| name == "modrinth.index.json")
        .ok_or_else(|| {
            MrpackError::Invalid("No modrinth.index.json found in mrpack".to_string())
        })?;

    let mut content = String::new();
    archive
        .by_index(manifest_idx)?
        .read_to_string(&mut content)?;

    let pack: PackFormat = serde_json::from_str(&content)?;

    pack.validate().map_err(MrpackError::Invalid)?;

    let metadata = pack.extract_metadata();

    // Preflight every destination before starting downloads or extracting files.
    let items = download_items(&pack, instance_dir)?;
    let overrides = override_entries(&mut archive, instance_dir)?;

    if !items.is_empty() {
        let batch = aqua::GenericBatch::new(format!("mrpack-{}", metadata.version_id), items);

        let dm = aqua::DownloadManager::new(shared_dir.to_path_buf());
        let handle = dm
            .prepare_batch(Box::new(batch))
            .await
            .map_err(|e| MrpackError::Download(e.to_string()))?;

        handle
            .download_all(progress)
            .await
            .map_err(|e| MrpackError::Download(e.to_string()))?;
    }

    extract_overrides(&mut archive, overrides).await?;
    extract_icon(&mut archive, instance_dir).await?;

    Ok(metadata)
}

fn install_path(instance_dir: &Path, relative: &str) -> Result<PathBuf, MrpackError> {
    // Reject Windows separators/prefixes on every platform as well as traversal.
    if relative.is_empty()
        || relative.contains(['\\', ':', '\0'])
        || !Path::new(relative)
            .components()
            .any(|c| matches!(c, std::path::Component::Normal(_)))
    {
        return Err(MrpackError::Invalid(format!(
            "Invalid pack path: '{}'",
            relative
        )));
    }
    safe_join(instance_dir, relative).map_err(MrpackError::Invalid)
}

fn download_items(
    pack: &PackFormat,
    instance_dir: &Path,
) -> Result<Vec<aqua::DownloadItemSpec>, MrpackError> {
    pack.files
        .iter()
        .filter(|f| {
            f.env
                .as_ref()
                .and_then(|env| env.get("client"))
                .is_none_or(|v| v != "unsupported")
        })
        .map(|f| {
            let dest = install_path(instance_dir, &f.path)?;
            let url = f
                .downloads
                .first()
                .filter(|url| !url.trim().is_empty())
                .ok_or_else(|| MrpackError::Invalid(format!("No download URL for '{}'", f.path)))?;
            let hash = f.hashes.get("sha1").map(|s| s.as_str()).unwrap_or("");
            Ok(aqua::DownloadItemSpec::new(url.clone(), dest, &f.path)
                .with_hash(hash)
                .with_size(f.file_size))
        })
        .collect()
}

fn override_entries(
    archive: &mut zip::ZipArchive<std::fs::File>,
    instance_dir: &Path,
) -> Result<Vec<(usize, PathBuf)>, MrpackError> {
    let mut common = Vec::new();
    let mut client = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        install_path(instance_dir, entry.name())?;
        if entry.is_dir() {
            continue;
        }
        let name = Path::new(entry.name());
        if let Ok(relative) = name.strip_prefix("overrides") {
            common.push((i, install_path(instance_dir, &relative.to_string_lossy())?));
        } else if let Ok(relative) = name.strip_prefix("client-overrides") {
            client.push((i, install_path(instance_dir, &relative.to_string_lossy())?));
        }
    }
    // Client overrides always win, regardless of ZIP entry order.
    common.extend(client);
    Ok(common)
}

async fn extract_overrides(
    archive: &mut zip::ZipArchive<std::fs::File>,
    entries: Vec<(usize, PathBuf)>,
) -> Result<(), MrpackError> {
    for (i, dest) in entries {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tracing::info!("Extracting override #{} -> {:?}", i, dest);

        let mut buffer = Vec::new();
        archive.by_index(i)?.read_to_end(&mut buffer)?;
        tokio::fs::write(&dest, &buffer).await?;
    }
    Ok(())
}

async fn extract_icon(
    archive: &mut zip::ZipArchive<std::fs::File>,
    instance_dir: &Path,
) -> Result<(), MrpackError> {
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
#[path = "../tests/mrpack/installer.rs"]
mod tests;
