use crate::core::PathManager;
use crate::core::{AppEvent, InstanceError, emit};
use crate::services::{DownloadQueue, InstanceManager};
use serde::Serialize;
use std::borrow::Cow;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Serialize)]
pub struct MrpackInfo {
    pub name: String,
    pub version_id: String,
    pub summary: Option<String>,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub file_count: usize,
    pub version_id_for_instance: Option<String>,
    pub icon: Option<String>,
}

#[tauri::command]
pub async fn parse_mrpack(path: String) -> Result<MrpackInfo, String> {
    info!("Parsing mrpack: {}", path);

    let metadata = cubrinth::mrpack::parse_mrpack(std::path::Path::new(&path))
        .map_err(|e| format!("Failed to parse mrpack: {}", e))?;

    let version_id_for_instance = metadata.game_version.as_ref().map(|gv| gv.to_version_id());

    Ok(MrpackInfo {
        name: metadata.name,
        version_id: metadata.version_id,
        summary: metadata.summary,
        minecraft_version: metadata
            .game_version
            .as_ref()
            .map(|gv| gv.mc_version.clone()),
        loader: metadata.game_version.as_ref().and_then(|gv| {
            if gv.loader.is_vanilla() {
                None
            } else {
                Some(gv.loader.name().to_string())
            }
        }),
        loader_version: metadata
            .game_version
            .as_ref()
            .and_then(|gv| gv.loader.version().map(|s| s.to_string())),
        file_count: metadata.file_count,
        version_id_for_instance,
        icon: None,
    })
}

#[tauri::command]
pub async fn install_mrpack(
    path: String,
    instance_name: String,
    project_id: Option<String>,
    modrinth_version_id: Option<String>,
    icon_url: Option<String>,
) -> Result<MrpackInfo, String> {
    info!(
        "Installing mrpack '{}' as instance '{}' (project={:?}, version={:?}, icon={:?})",
        path, instance_name, project_id, modrinth_version_id, icon_url
    );

    let metadata = cubrinth::mrpack::parse_mrpack(std::path::Path::new(&path))
        .map_err(|e| format!("Failed to parse mrpack: {}", e))?;

    let game_version = metadata
        .game_version
        .clone()
        .ok_or_else(|| "No Minecraft version specified in pack dependencies".to_string())?;

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

    // Save upstream metadata if provided
    if let (Some(pid), Some(vid)) = (&project_id, &modrinth_version_id) {
        let upstream = serde_json::json!({
            "type": "modrinth-modpack",
            "projectId": pid,
            "versionId": vid,
        });
        let upstream_path = instance_dir.join("upstream.json");
        let upstream_content = serde_json::to_string_pretty(&upstream)
            .map_err(|e| format!("Failed to serialize upstream: {}", e))?;
        let _ = tokio::fs::write(&upstream_path, &upstream_content).await;
        info!("Saved upstream metadata to {:?}", upstream_path);
    }

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
    let install_result = cubrinth::mrpack::install_mrpack(
        std::path::Path::new(&path),
        &instance_dir,
        &shared_dir,
        Some(progress_tx),
    )
    .await;

    let _ = progress_task.await;

    DownloadQueue::get().finish_work("mods").await;

    install_result.map_err(|e| format!("Failed to install mrpack: {}", e))?;

    // Download icon from Modrinth if available and set as instance icon
    let icon = if let Some(url) = icon_url {
        match reqwest::get(&url).await {
            Ok(response) => {
                match response.bytes().await {
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
                }
            }
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

    Ok(MrpackInfo {
        name: metadata.name,
        version_id: metadata.version_id,
        summary: metadata.summary,
        minecraft_version: Some(game_version.mc_version),
        loader: if game_version.loader.is_vanilla() {
            None
        } else {
            Some(game_version.loader.name().to_string())
        },
        loader_version: game_version.loader.version().map(|s| s.to_string()),
        file_count: metadata.file_count,
        version_id_for_instance: Some(version_id),
        icon,
    })
}
