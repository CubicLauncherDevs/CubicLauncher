use std::io::Read;
use std::path::Path;

use super::pack_format::{MrpackMetadata, PackFormat};

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

    if pack.game != "minecraft" {
        return Err(MrpackError::Invalid(format!(
            "Pack is for '{}', not 'minecraft'",
            pack.game
        )));
    }

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

    if pack.game != "minecraft" {
        return Err(MrpackError::Invalid(format!(
            "Pack is for '{}', not 'minecraft'",
            pack.game
        )));
    }

    let metadata = pack.extract_metadata();

    // Build DownloadItemSpecs for files that need downloading
    let items: Vec<aqua::DownloadItemSpec> = pack
        .files
        .iter()
        .filter(|f| {
            !f.env
                .as_ref()
                .and_then(|env| env.get("client"))
                .is_some_and(|v| v == "unsupported")
        })
        .filter_map(|f| {
            let url = f.downloads.first()?;
            let dest = instance_dir.join(&f.path);
            let hash = f.hashes.get("sha1").map(|s| s.as_str()).unwrap_or("");
            Some(
                aqua::DownloadItemSpec::new(url.clone(), dest, &f.path)
                    .with_hash(hash)
                    .with_size(f.file_size as u64),
            )
        })
        .collect();

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

    extract_overrides(&mut archive, instance_dir).await?;
    extract_icon(&mut archive, instance_dir).await?;

    Ok(metadata)
}

async fn extract_overrides(
    archive: &mut zip::ZipArchive<std::fs::File>,
    instance_dir: &Path,
) -> Result<(), MrpackError> {
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();
        let is_dir = entry.is_dir();
        drop(entry);

        if is_dir {
            continue;
        }

        let relative_path = if let Some(stripped) = entry_name.strip_prefix("overrides/") {
            stripped.to_string()
        } else if let Some(stripped) = entry_name.strip_prefix("client-overrides/") {
            stripped.to_string()
        } else {
            continue;
        };

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
