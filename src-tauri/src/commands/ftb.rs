use crate::core::{AppEvent, InstanceError, emit};
use crate::services::{DownloadQueue, InstanceManager};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tracing::info;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseManifestMinecraft {
    version: String,
    mod_loaders: Vec<CurseManifestLoader>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseManifestLoader {
    id: String,
    primary: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseManifestFile {
    project_id: u32,
    file_id: u32,
    required: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseManifest {
    minecraft: CurseManifestMinecraft,
    #[allow(dead_code)]
    manifest_type: Option<String>,
    #[allow(dead_code)]
    manifest_version: Option<i32>,
    name: String,
    version: String,
    author: Option<String>,
    files: Vec<CurseManifestFile>,
    #[allow(dead_code)]
    overrides: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CurseManifestInfo {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub minecraft_version: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub file_count: usize,
    pub manifest_files: Vec<ManifestFileEntry>,
}

#[derive(Debug, Serialize)]
pub struct ManifestFileEntry {
    pub project_id: u32,
    pub file_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurseFileUrl {
    pub project_id: u32,
    pub file_id: u32,
    pub url: String,
    pub filename: String,
}

#[tauri::command]
pub async fn parse_curse_manifest(path: String) -> Result<CurseManifestInfo, String> {
    info!("Parsing CurseForge/FTB manifest from: {}", path);

    let manifest = read_curse_manifest(Path::new(&path))?;

    let (loader, loader_version) = parse_loader(&manifest);

    Ok(CurseManifestInfo {
        name: manifest.name,
        version: manifest.version,
        author: manifest.author,
        minecraft_version: manifest.minecraft.version,
        loader,
        loader_version,
        file_count: manifest.files.len(),
        manifest_files: manifest
            .files
            .into_iter()
            .filter(|f| f.required)
            .map(|f| ManifestFileEntry {
                project_id: f.project_id,
                file_id: f.file_id,
            })
            .collect(),
    })
}

#[derive(Debug, Serialize)]
pub struct InstallResult {
    pub name: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub version_id_for_instance: String,
}

#[tauri::command]
pub async fn install_curse_manifest(
    path: String,
    instance_name: String,
    file_urls: Vec<CurseFileUrl>,
) -> Result<InstallResult, String> {
    info!(
        "Installing CurseForge/FTB modpack from '{}' as '{}'",
        path, instance_name
    );

    let manifest = read_curse_manifest(Path::new(&path))?;
    let mc_version = manifest.minecraft.version.clone();

    let (loader, loader_version) = parse_loader(&manifest);

    let version_id = if let Some(ref lv) = loader_version {
        match loader.as_deref() {
            Some("fabric") => format!("fabric-loader-{}-{}", lv, mc_version),
            Some("forge") => mc_version.to_string(),
            Some("neoforge") => mc_version.to_string(),
            _ => mc_version.clone(),
        }
    } else {
        mc_version.clone()
    };

    let manager = InstanceManager::get();
    let handle = manager
        .create_instance(instance_name, version_id.clone(), None)
        .await
        .map_err(|e| match e {
            InstanceError::AlreadyExists => "An instance with that name already exists".to_string(),
            other => format!("Failed to create instance: {}", other),
        })?;

    let instance_dir = handle.get_instance_dir().await;

    let client = reqwest::Client::builder()
        .user_agent("CubicLauncher/27.0.1")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mods_dir = instance_dir.join("mods");
    tokio::fs::create_dir_all(&mods_dir)
        .await
        .map_err(|e| format!("Failed to create mods directory: {}", e))?;

    for file_entry in &file_urls {
        let dest = mods_dir.join(&file_entry.filename);
        info!("Downloading {} -> {:?}", file_entry.url, dest);

        let response = client
            .get(&file_entry.url)
            .send()
            .await
            .map_err(|e| format!("Download failed for {}: {}", file_entry.filename, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP {} downloading {}",
                response.status(),
                file_entry.filename
            ));
        }

        let mut file = tokio::fs::File::create(&dest)
            .await
            .map_err(|e| format!("Failed to create file {:?}: {}", dest, e))?;
        let mut stream = response.bytes_stream();
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write error: {}", e))?;
        }
        file.flush()
            .await
            .map_err(|e| format!("Flush error: {}", e))?;
    }

    extract_overrides(Path::new(&path), &instance_dir).await?;

    DownloadQueue::get().enqueue(mc_version.clone()).await;

    emit(AppEvent::InstanceCreated {
        id: handle.uuid.to_string().into(),
        dto: handle.to_dto().await,
    });

    Ok(InstallResult {
        name: manifest.name,
        version: manifest.version,
        minecraft_version: mc_version,
        loader,
        loader_version,
        version_id_for_instance: version_id,
    })
}

fn read_curse_manifest(path: &Path) -> Result<CurseManifest, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;

    let manifest_idx = archive
        .file_names()
        .position(|name| name == "manifest.json")
        .ok_or_else(|| {
            "No manifest.json found in the zip archive (not a CurseForge/FTB modpack)".to_string()
        })?;

    let mut content = String::new();
    archive
        .by_index(manifest_idx)
        .map_err(|e| format!("Failed to read manifest.json: {}", e))?
        .read_to_string(&mut content)
        .map_err(|e| format!("Failed to read manifest.json content: {}", e))?;

    let manifest: CurseManifest =
        serde_json::from_str(&content).map_err(|e| format!("Invalid manifest.json: {}", e))?;

    Ok(manifest)
}

fn parse_loader(manifest: &CurseManifest) -> (Option<String>, Option<String>) {
    for ml in &manifest.minecraft.mod_loaders {
        let parts: Vec<&str> = ml.id.split('-').collect();
        if parts.len() >= 2 {
            let loader_name = parts[0].to_string();
            let loader_ver = parts[1..].join("-");
            if ml.primary {
                return (Some(loader_name), Some(loader_ver));
            }
        }
    }

    if let Some(ml) = manifest.minecraft.mod_loaders.first() {
        let parts: Vec<&str> = ml.id.split('-').collect();
        if parts.len() >= 2 {
            return (Some(parts[0].to_string()), Some(parts[1..].join("-")));
        }
    }

    (None, None)
}

#[tauri::command]
pub async fn install_ftb_modpack(
    file_urls: Vec<CurseFileUrl>,
    instance_name: String,
    name: String,
    version: String,
    minecraft_version: String,
    loader: Option<String>,
    loader_version: Option<String>,
) -> Result<InstallResult, String> {
    info!(
        "Installing FTB modpack '{}' v{} as '{}'",
        name, version, instance_name
    );

    let version_id = if let Some(ref lv) = loader_version {
        match loader.as_deref() {
            Some("fabric") => format!("fabric-loader-{}-{}", lv, minecraft_version),
            Some("forge") => minecraft_version.to_string(),
            Some("neoforge") => minecraft_version.to_string(),
            _ => minecraft_version.clone(),
        }
    } else {
        minecraft_version.clone()
    };

    let manager = InstanceManager::get();
    let handle = manager
        .create_instance(instance_name, version_id.clone(), None)
        .await
        .map_err(|e| match e {
            InstanceError::AlreadyExists => "An instance with that name already exists".to_string(),
            other => format!("Failed to create instance: {}", other),
        })?;

    let instance_dir = handle.get_instance_dir().await;

    let client = reqwest::Client::builder()
        .user_agent("CubicLauncher/27.0.1")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mods_dir = instance_dir.join("mods");
    tokio::fs::create_dir_all(&mods_dir)
        .await
        .map_err(|e| format!("Failed to create mods directory: {}", e))?;

    for file_entry in &file_urls {
        let dest = mods_dir.join(&file_entry.filename);
        info!("Downloading {} -> {:?}", file_entry.url, dest);

        let response = client
            .get(&file_entry.url)
            .send()
            .await
            .map_err(|e| format!("Download failed for {}: {}", file_entry.filename, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP {} downloading {}",
                response.status(),
                file_entry.filename
            ));
        }

        let mut file = tokio::fs::File::create(&dest)
            .await
            .map_err(|e| format!("Failed to create file {:?}: {}", dest, e))?;
        let mut stream = response.bytes_stream();
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write error: {}", e))?;
        }
        file.flush()
            .await
            .map_err(|e| format!("Flush error: {}", e))?;
    }

    DownloadQueue::get().enqueue(version_id.clone()).await;

    emit(AppEvent::InstanceCreated {
        id: handle.uuid.to_string().into(),
        dto: handle.to_dto().await,
    });

    Ok(InstallResult {
        name,
        version,
        minecraft_version,
        loader,
        loader_version,
        version_id_for_instance: version_id,
    })
}

async fn extract_overrides(zip_path: &Path, instance_dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;

    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("Zip error: {}", e))?;
        let entry_name = entry.name().to_string();
        let is_dir = entry.is_dir();
        drop(entry);

        if is_dir {
            continue;
        }

        let relative_path = if let Some(stripped) = entry_name.strip_prefix("overrides/") {
            stripped.to_string()
        } else {
            continue;
        };

        let dest = instance_dir.join(&relative_path);

        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directory {:?}: {}", parent, e))?;
        }

        let mut buffer = Vec::new();
        archive
            .by_index(i)
            .map_err(|e| format!("Zip read error: {}", e))?
            .read_to_end(&mut buffer)
            .map_err(|e| format!("Read error: {}", e))?;

        tokio::fs::write(&dest, &buffer)
            .await
            .map_err(|e| format!("Write error on {:?}: {}", dest, e))?;
    }

    Ok(())
}
