use crate::commands::market::get_curseforge_file_download_url;
use crate::core::{AppEvent, InstanceError, PathManager, emit};
use crate::services::{DownloadQueue, InstanceManager};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Serialize)]
pub struct CfpackInfo {
    pub name: String,
    pub version_id: String,
    pub summary: Option<String>,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub file_count: usize,
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeManifest {
    manifest_type: String,
    name: String,
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    files: Vec<CfManifestFile>,
    overrides: Option<String>,
    minecraft: CfMinecraftSection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMinecraftSection {
    version: String,
    #[serde(default)]
    mod_loaders: Vec<CfModLoader>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfModLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifestFile {
    project_id: i64,
    file_id: i64,
    #[serde(default = "default_required")]
    required: bool,
}

fn default_required() -> bool {
    true
}

pub fn parse_curseforge_modpack(path: &Path) -> Result<CfpackInfo, String> {
    info!("Parsing CurseForge modpack: {}", path.display());

    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("ZIP error: {}", e))?;

    let manifest_idx = archive
        .file_names()
        .position(|name| name == "manifest.json")
        .ok_or_else(|| "No manifest.json found in archive".to_string())?;

    let mut content = String::new();
    archive
        .by_index(manifest_idx)
        .map_err(|e| format!("Failed to read manifest: {}", e))?
        .read_to_string(&mut content)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    let manifest: CurseForgeManifest =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse manifest: {}", e))?;

    if manifest.manifest_type != "minecraftModpack" {
        return Err(format!(
            "Unsupported manifest type: {}",
            manifest.manifest_type
        ));
    }

    let primary_loader = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first());

    let (loader_name, loader_version) = primary_loader
        .map(|l| {
            let parts: Vec<&str> = l.id.split('-').collect();
            if parts.len() >= 2 {
                (parts[0].to_string(), parts[1..].join("-"))
            } else {
                (l.id.clone(), String::new())
            }
        })
        .unwrap_or_else(|| ("vanilla".to_string(), String::new()));

    Ok(CfpackInfo {
        name: manifest.name,
        version_id: manifest.version,
        summary: if manifest.description.is_empty() {
            None
        } else {
            Some(manifest.description)
        },
        minecraft_version: Some(manifest.minecraft.version.clone()),
        loader: if loader_name == "vanilla" {
            None
        } else {
            Some(loader_name)
        },
        loader_version: if loader_version.is_empty() {
            None
        } else {
            Some(loader_version)
        },
        file_count: manifest.files.len(),
        icon: None,
    })
}

#[tauri::command]
pub async fn parse_curseforge_modpack_cmd(path: String) -> Result<CfpackInfo, String> {
    parse_curseforge_modpack(Path::new(&path))
}

#[tauri::command]
pub async fn download_curseforge_file(mod_id: u32, file_id: u32) -> Result<String, String> {
    info!("Downloading CurseForge file {}:{}", mod_id, file_id);

    let download_url = get_curseforge_file_download_url(mod_id, file_id).await?;
    if download_url.is_empty() {
        return Err("CurseForge returned empty download URL".to_string());
    }

    let cache_dir = PathManager::get().get_shared_dir().join("curseforge-cache");
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;

    let filename = format!("{}-{}.zip", mod_id, file_id);
    let dest = cache_dir.join(&filename);

    if dest.exists() {
        return Ok(dest.to_string_lossy().to_string());
    }

    let response = crate::core::http_client::HTTP
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download bytes: {}", e))?;

    tokio::fs::write(&dest, &bytes)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn install_curseforge_modpack(
    path: String,
    instance_name: String,
    project_id: Option<String>,
    file_id: Option<String>,
    icon_url: Option<String>,
) -> Result<CfpackInfo, String> {
    info!(
        "Installing CurseForge modpack '{}' as instance '{}' (project={:?}, file={:?}, icon={:?})",
        path, instance_name, project_id, file_id, icon_url
    );

    let path = PathBuf::from(path);
    let info = parse_curseforge_modpack(&path)?;

    let file = std::fs::File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("ZIP error: {}", e))?;

    let manifest_idx = archive
        .file_names()
        .position(|name| name == "manifest.json")
        .ok_or_else(|| "No manifest.json found in archive".to_string())?;

    let mut content = String::new();
    archive
        .by_index(manifest_idx)
        .map_err(|e| format!("Failed to read manifest: {}", e))?
        .read_to_string(&mut content)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    let manifest: CurseForgeManifest =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse manifest: {}", e))?;

    let game_version = build_game_version(&manifest)?;
    let version_id = game_version.to_version_id();
    let mc_version_only = game_version.mc_version.clone();

    let manager = InstanceManager::get();

    let handle = manager
        .create_instance(instance_name, version_id.clone(), None)
        .await
        .map_err(|e| match e {
            InstanceError::AlreadyExists => "An instance with that name already exists".to_string(),
            other => format!("Failed to create instance: {}", other),
        })?;

    let instance_dir = handle.get_instance_dir().await;

    if let (Some(pid), Some(fid)) = (&project_id, &file_id) {
        let upstream = serde_json::json!({
            "type": "curseforge-modpack",
            "projectId": pid,
            "fileId": fid,
        });
        let upstream_path = instance_dir.join("upstream.json");
        let upstream_content = serde_json::to_string_pretty(&upstream)
            .map_err(|e| format!("Failed to serialize upstream: {}", e))?;
        let _ = tokio::fs::write(&upstream_path, &upstream_content).await;
        info!("Saved upstream metadata to {:?}", upstream_path);
    }

    let required_files: Vec<&CfManifestFile> =
        manifest.files.iter().filter(|f| f.required).collect();

    if !required_files.is_empty() {
        DownloadQueue::get().enqueue_work("mods").await;

        let (progress_tx, mut progress_rx) =
            tokio::sync::watch::channel(aqua::progress::DownloadProgress::empty(0));
        let mods_label: Arc<str> = "mods".into();
        let progress_task = tokio::spawn(async move {
            loop {
                if progress_rx.changed().await.is_err() {
                    break;
                }
                let p = progress_rx.borrow_and_update().clone();
                emit(AppEvent::DProgress {
                    version: mods_label.clone(),
                    stage: Cow::Owned(p.stage.as_str().to_string()),
                    item_current: p.item_current as u64,
                    item_total: p.item_total as u64,
                    bytes_current: p.bytes_current,
                    bytes_total: p.bytes_total,
                    current_item: p.current_item,
                });
            }
        });

        let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
        let download_result = download_modpack_files(
            &required_files,
            &instance_dir,
            &shared_dir,
            Some(progress_tx),
        )
        .await;

        let _ = progress_task.await;
        DownloadQueue::get().finish_work("mods").await;

        download_result.map_err(|e| format!("Failed to download modpack files: {}", e))?;
    }

    extract_overrides(
        &mut archive,
        &instance_dir,
        manifest.overrides.as_deref().unwrap_or("overrides"),
    )
    .await
    .map_err(|e| format!("Failed to extract overrides: {}", e))?;

    let icon = if let Some(url) = icon_url {
        match crate::core::http_client::HTTP.get(&url).send().await {
            Ok(response) => match response.bytes().await {
                Ok(bytes) => {
                    let icon_path = instance_dir.join("icon.png");
                    if let Err(e) = tokio::fs::write(&icon_path, &bytes).await {
                        tracing::error!("Failed to write icon: {}", e);
                        None
                    } else if let Some(icon_str) = icon_path.to_str() {
                        handle.set_icon(Some(icon_str.to_string())).await;
                        let _ = handle.save_if_dirty().await;
                        Some(icon_str.to_string())
                    } else {
                        None
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read icon bytes: {}", e);
                    None
                }
            },
            Err(e) => {
                tracing::error!("Failed to download icon: {}", e);
                None
            }
        }
    } else {
        None
    };

    match &game_version.loader {
        zellkern::Loader::Fabric(_)
        | zellkern::Loader::Quilt(_)
        | zellkern::Loader::Forge(_)
        | zellkern::Loader::NeoForge(_) => {
            DownloadQueue::get().enqueue(version_id.clone()).await;
        }
        _ => {
            DownloadQueue::get().enqueue(mc_version_only.clone()).await;
        }
    }

    emit(AppEvent::InstanceCreated {
        id: handle.uuid.to_string().into(),
        dto: handle.to_dto().await,
    });

    Ok(CfpackInfo {
        name: info.name,
        version_id: info.version_id,
        summary: info.summary,
        minecraft_version: info.minecraft_version,
        loader: info.loader,
        loader_version: info.loader_version,
        file_count: info.file_count,
        icon,
    })
}

fn build_game_version(manifest: &CurseForgeManifest) -> Result<zellkern::GameVersion, String> {
    let mc_version = manifest.minecraft.version.clone();
    if mc_version.is_empty() {
        return Err("No Minecraft version specified in manifest".to_string());
    }

    let primary_loader = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first());

    let loader = if let Some(loader) = primary_loader {
        let parts: Vec<&str> = loader.id.split('-').collect();
        if parts.is_empty() {
            zellkern::Loader::Vanilla
        } else {
            let loader_type = parts[0].to_lowercase();
            let version = parts[1..].join("-");
            match loader_type.as_str() {
                "fabric" => zellkern::Loader::Fabric(version),
                "forge" => zellkern::Loader::Forge(version),
                "neoforge" => zellkern::Loader::NeoForge(version),
                "quilt" => zellkern::Loader::Quilt(version),
                _ => zellkern::Loader::Vanilla,
            }
        }
    } else {
        zellkern::Loader::Vanilla
    };

    Ok(zellkern::GameVersion { mc_version, loader })
}

async fn download_modpack_files(
    files: &[&CfManifestFile],
    instance_dir: &Path,
    shared_dir: &Path,
    progress: Option<aqua::progress::ProgressSender>,
) -> Result<(), String> {
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(8));
    let mut handles = Vec::new();

    for file in files {
        let project_id = file.project_id as u32;
        let file_id = file.file_id as u32;
        let permit = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = permit.acquire().await.map_err(|e| e.to_string())?;
            let url = get_curseforge_file_download_url(project_id, file_id).await?;
            if url.is_empty() {
                return Err(format!("Empty download URL for {}:{}", project_id, file_id));
            }
            let filename = format!("{}-{}.jar", project_id, file_id);
            Ok::<(u32, u32, String, String), String>((project_id, file_id, url, filename))
        }));
    }

    let mut items = Vec::new();
    let mods_dir = instance_dir.join("mods");
    tokio::fs::create_dir_all(&mods_dir)
        .await
        .map_err(|e| format!("Failed to create mods dir: {}", e))?;

    for handle in handles {
        let (project_id, file_id, url, filename) = handle
            .await
            .map_err(|e| format!("URL resolution task panicked: {}", e))??;
        let dest = mods_dir.join(&filename);
        let label = format!("{}-{}", project_id, file_id);
        items.push(aqua::DownloadItemSpec::new(url, dest, &label).with_size(0));
    }

    if !items.is_empty() {
        let batch = aqua::GenericBatch::new("curseforge-modpack".to_string(), items);
        let dm = aqua::DownloadManager::new(shared_dir.to_path_buf());
        let handle = dm
            .prepare_batch(Box::new(batch))
            .await
            .map_err(|e| format!("Failed to prepare batch: {}", e))?;
        handle
            .download_all(progress)
            .await
            .map_err(|e| format!("Failed to download modpack files: {}", e))?;
    }

    Ok(())
}

async fn extract_overrides(
    archive: &mut zip::ZipArchive<std::fs::File>,
    instance_dir: &Path,
    overrides_dir: &str,
) -> Result<(), std::io::Error> {
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();
        let is_dir = entry.is_dir();
        drop(entry);

        if is_dir {
            continue;
        }

        let prefix = format!("{}/", overrides_dir);
        let relative_path = if let Some(stripped) = entry_name.strip_prefix(&prefix) {
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
