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
    })
}

#[tauri::command]
pub async fn install_mrpack(path: String, instance_name: String) -> Result<MrpackInfo, String> {
    info!(
        "Installing mrpack '{}' as instance '{}'",
        path, instance_name
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

    DownloadQueue::get().enqueue_work("mods").await;

    let (progress_tx, progress_rx) = tokio::sync::mpsc::channel::<(usize, usize)>(32);
    let mods_label: Arc<str> = "mods".into();
    let progress_task = tokio::spawn(async move {
        let mut rx = progress_rx;
        while let Some((current, total)) = rx.recv().await {
            emit(AppEvent::DProgress {
                version: mods_label.clone(),
                stage: Cow::Borrowed("generic"),
                item_current: current as u64,
                item_total: total as u64,
                bytes_current: 0,
                bytes_total: 0,
                current_item: None,
            });
        }
    });

    let install_result = cubrinth::mrpack::install_mrpack(
        std::path::Path::new(&path),
        &instance_dir,
        Some(progress_tx),
    )
    .await;

    let _ = progress_task.await;

    DownloadQueue::get().finish_work("mods").await;

    install_result.map_err(|e| format!("Failed to install mrpack: {}", e))?;

    match &game_version.loader {
        zellkern::Loader::Fabric(_) | zellkern::Loader::Quilt(_) => {
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
    })
}


