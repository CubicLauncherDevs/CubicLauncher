use std::io::Read;
use std::path::Path;
use tokio::io::AsyncWriteExt;

use super::pack_format::{MrpackMetadata, PackFormat};

const USER_AGENT: &str = "CubicLauncher/27.0.1";

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
    progress: Option<tokio::sync::mpsc::Sender<(usize, usize)>>,
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

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| MrpackError::Download(e.to_string()))?;

    let total = pack.files.len();
    let mut current = 0usize;

    for file_entry in &pack.files {
        if let Some(env) = &file_entry.env
            && env.get("client").map(|s| s.as_str()) == Some("unsupported")
        {
            current += 1;
            if let Some(ref tx) = progress {
                let _ = tx.send((current, total)).await;
            }
            continue;
        }

        let dest = instance_dir.join(&file_entry.path);

        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let url = match file_entry.downloads.first() {
            Some(u) => u,
            None => {
                tracing::warn!("No download URL for {}", file_entry.path);
                current += 1;
                if let Some(ref tx) = progress {
                    let _ = tx.send((current, total)).await;
                }
                continue;
            }
        };

        tracing::info!("Downloading {} -> {:?}", url, dest);

        download_file(&client, url, &dest).await?;

        current += 1;
        if let Some(ref tx) = progress {
            let _ = tx.send((current, total)).await;
        }
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

async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<(), MrpackError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| MrpackError::Download(format!("Request failed for {}: {}", url, e)))?;

    if !response.status().is_success() {
        return Err(MrpackError::Download(format!(
            "HTTP {} for {}",
            response.status(),
            url
        )));
    }

    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = response.bytes_stream();

    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| MrpackError::Download(format!("Stream error: {}", e)))?;
        file.write_all(&chunk).await?;
    }

    file.flush().await?;
    Ok(())
}
