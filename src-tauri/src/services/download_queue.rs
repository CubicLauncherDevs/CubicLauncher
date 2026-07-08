use crate::core::path_manager::PathManager;
use crate::core::{AppEvent, emit};
use crate::services::java_manager::JavaManager;
use aqua::{DownloadBatch, DownloadManager, DownloadProgress, JreBatch};
use compact_str::CompactString;
use dashmap::DashMap;
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::{mpsc, watch};
use tracing::{error, info};

static DOWNLOAD_QUEUE: OnceLock<Arc<DownloadQueue>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Done,
    Error(String),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadState {
    pub version: Arc<str>,
    pub status: DownloadStatus,
    pub stage: Cow<'static, str>,
    pub item_current: u64,
    pub item_total: u64,
    pub bytes_current: u64,
    pub bytes_total: u64,
    pub current_item: Option<String>,
}

impl DownloadState {
    fn new(version: Arc<str>) -> Self {
        Self {
            version,
            status: DownloadStatus::Pending,
            stage: Cow::Borrowed("pending"),
            item_current: 0,
            item_total: 0,
            bytes_current: 0,
            bytes_total: 0,
            current_item: None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            DownloadStatus::Pending | DownloadStatus::Downloading
        )
    }
}

pub struct DownloadQueue {
    sender: mpsc::Sender<Arc<str>>,
    active: DashMap<Arc<str>, DownloadState>,
    pending_batches: DashMap<Arc<str>, Box<dyn DownloadBatch + 'static>>,
}

impl DownloadQueue {
    pub fn get() -> &'static Arc<DownloadQueue> {
        DOWNLOAD_QUEUE
            .get()
            .expect("BUG: DownloadQueue usado antes de inicializar")
    }

    pub async fn init(_app_handle: Option<tauri::AppHandle>) -> Arc<Self> {
        let (tx, rx) = mpsc::channel::<Arc<str>>(64);

        let queue = Arc::new(Self {
            sender: tx,
            active: DashMap::new(),
            pending_batches: DashMap::new(),
        });

        let queue_clone = queue.clone();
        tokio::spawn(async move {
            Self::worker(rx, queue_clone).await;
        });

        let _ = DOWNLOAD_QUEUE.set(queue.clone());
        queue
    }

    pub async fn enqueue(&self, version: impl Into<Arc<str>>) {
        let version: Arc<str> = version.into();

        if let Some(state) = self.active.get(&version)
            && state.is_active()
        {
            return;
        }

        info!("{} encolada", &*version);

        self.active
            .insert(version.clone(), DownloadState::new(version.clone()));

        emit(AppEvent::DEnqueue {
            version: version.clone(),
        });

        if let Err(e) = self.sender.send(version).await {
            error!("Error al encolar descarga: {}", e);
        }
    }

    pub async fn get_active_downloads(&self) -> Vec<DownloadState> {
        self.active
            .iter()
            .filter(|r| r.value().is_active())
            .map(|r| r.value().clone())
            .collect()
    }

    pub async fn enqueue_work(&self, label: impl Into<Arc<str>>) {
        let label: Arc<str> = label.into();
        self.active.insert(
            label.clone(),
            DownloadState {
                version: label.clone(),
                status: DownloadStatus::Downloading,
                stage: Cow::Borrowed("downloading"),
                item_current: 0,
                item_total: 0,
                bytes_current: 0,
                bytes_total: 0,
                current_item: None,
            },
        );
        emit(AppEvent::DEnqueue {
            version: label.clone(),
        });
    }

    pub async fn enqueue_batch(
        &self,
        version: impl Into<Arc<str>>,
        batch: Box<dyn DownloadBatch + 'static>,
    ) {
        let version: Arc<str> = version.into();

        if let Some(state) = self.active.get(&version)
            && state.is_active()
        {
            return;
        }

        info!("Batch {} encolada", &*version);

        self.pending_batches.insert(version.clone(), batch);
        self.active
            .insert(version.clone(), DownloadState::new(version.clone()));

        emit(AppEvent::DEnqueue {
            version: version.clone(),
        });

        if let Err(e) = self.sender.send(version).await {
            error!("Error al encolar batch: {}", e);
        }
    }

    pub async fn finish_work(&self, label: &str) {
        let label: Arc<str> = label.into();
        if let Some(mut state) = self.active.get_mut(&label) {
            state.status = DownloadStatus::Done;
        }
        emit(AppEvent::DFinish {
            version: label.clone(),
        });
        self.active.retain(|_, s| s.is_active());
    }

    async fn worker(mut rx: mpsc::Receiver<Arc<str>>, queue: Arc<DownloadQueue>) {
        while let Some(version) = rx.recv().await {
            Self::process_version(&queue, version).await;
        }

        error!("Worker de descargas terminó inesperadamente — el channel fue cerrado");
    }

    async fn process_version(queue: &Arc<DownloadQueue>, version: Arc<str>) {
        let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
        let manager = DownloadManager::new(shared_dir.clone());

        if let Some(mut state) = queue.active.get_mut(&version) {
            state.status = DownloadStatus::Downloading;
        } else {
            error!("State no encontrado para {}, saltando", version);
            queue.active.retain(|_, s| s.is_active());
            return;
        }

        // JRE batch (pending_batches)
        if let Some(batch) = queue.pending_batches.remove(&version) {
            let (_, batch) = batch;
            let (tx, progress_rx) = watch::channel(DownloadProgress::empty(0));
            let monitor = monitor_download_progress(version.clone(), progress_rx, queue.clone());

            let (dl_result, ()) = tokio::join!(async { batch.finalize(Some(tx)).await }, monitor);

            match dl_result {
                Ok(_) => {
                    info!("Batch {} completado correctamente", version);
                    if let Some(mut state) = queue.active.get_mut(&version) {
                        state.status = DownloadStatus::Done;
                    }
                    emit(AppEvent::DFinish {
                        version: version.clone(),
                    });
                    emit(AppEvent::JREChanged);
                }
                Err(e) => {
                    emit_and_set_error(
                        queue,
                        &version,
                        format!("No se pudo completar el batch {}: {:?}", version, e),
                    );
                }
            }

            queue.active.retain(|_, s| s.is_active());
            return;
        }

        let result = if version.contains("-neoforge-") {
            Self::process_neoforge_version(shared_dir, &manager, queue, version.clone()).await
        } else if version.contains("-forge-") {
            Self::process_forge_version(shared_dir, &manager, queue, version.clone()).await
        } else if let Some((game_version, loader_version)) = parse_fabric_version(&version) {
            Self::process_fabric_version(
                shared_dir,
                &manager,
                queue,
                version.clone(),
                game_version,
                loader_version,
            )
            .await
        } else if let Some((game_version, loader_version)) = parse_quilt_version(&version) {
            Self::process_quilt_version(
                shared_dir,
                &manager,
                queue,
                version.clone(),
                game_version,
                loader_version,
            )
            .await
        } else {
            // Vanilla (or invalid version)
            match manager.prepare(&version).await {
                Ok(handle) => download_with_progress(version.clone(), handle, queue.clone()).await,
                Err(_) => Err(aqua::AquaError::Other(format!(
                    "La versión solicitada no existe: {}",
                    version
                ))),
            }
        };

        match result {
            Ok(_) => {
                info!("Versión {} descargada correctamente", version);
                if let Some(mut state) = queue.active.get_mut(&version) {
                    state.status = DownloadStatus::Done;
                }
                emit(AppEvent::DFinish { version });
            }
            Err(e) => {
                emit_and_set_error(
                    queue,
                    &version,
                    format!("No se pudo descargar {}: {:?}", version, e),
                );
            }
        }

        queue.active.retain(|_, s| s.is_active());
    }

    async fn process_fabric_version(
        shared_dir: PathBuf,
        manager: &DownloadManager,
        queue: &Arc<DownloadQueue>,
        version: Arc<str>,
        game_version: String,
        loader_version: String,
    ) -> Result<(), aqua::AquaError> {
        emit_stage(
            &version,
            "resolving",
            Some(format!("Fabric loader {}", loader_version)),
        );

        let batch = aqua::FabricBatch::new(&shared_dir, &game_version, &loader_version).await?;
        let handle = manager.prepare_batch(Box::new(batch)).await?;
        download_with_progress(version.clone(), handle, queue.clone()).await?;

        download_base_mc(version, &game_version, manager, queue).await
    }

    async fn process_quilt_version(
        shared_dir: PathBuf,
        manager: &DownloadManager,
        queue: &Arc<DownloadQueue>,
        version: Arc<str>,
        game_version: String,
        loader_version: String,
    ) -> Result<(), aqua::AquaError> {
        emit_stage(
            &version,
            "resolving",
            Some(format!("Quilt loader {}", loader_version)),
        );

        let batch = aqua::QuiltBatch::new(&shared_dir, &game_version, &loader_version).await?;
        let handle = manager.prepare_batch(Box::new(batch)).await?;
        download_with_progress(version.clone(), handle, queue.clone()).await?;

        download_base_mc(version, &game_version, manager, queue).await
    }

    async fn process_forge_version(
        shared_dir: PathBuf,
        manager: &DownloadManager,
        queue: &Arc<DownloadQueue>,
        version: Arc<str>,
    ) -> Result<(), aqua::AquaError> {
        let parts: Vec<&str> = version.split("-forge-").collect();
        let (gv, fv) = match parts.as_slice() {
            [gv, fv] => (*gv, *fv),
            _ => {
                return Err(aqua::AquaError::Other(format!(
                    "Forge version format invalid: {}",
                    version
                )));
            }
        };

        // Determine Java preference based on MC version
        // MC 1.21+ → Java 21; MC 1.17-1.20.4 → Java 17; < 1.17 → Java 8
        let java_pref: &[u8] = match parse_mc_major_minor(gv) {
            Some((1, n)) if n >= 21 => &[21, 17, 8],
            Some((1, n)) if n >= 17 => &[17, 21, 8],
            _ => &[8, 17, 21],
        };

        // Ensure a suitable Java runtime is installed, making the download visible.
        if !java_pref.iter().any(|v| JavaManager::is_installed(*v)) {
            let java_version = java_pref[0];
            emit_stage(
                &version,
                "jre",
                Some(format!("Java {} para Forge {}", java_version, version)),
            );
            let pkg = JavaManager::get_latest_package(java_version)
                .await
                .map_err(|e| aqua::AquaError::Other(e.to_string()))?;
            let dest_dir = JavaManager::get_jre_dir(java_version);
            let jre_batch = JreBatch::new(java_version, pkg, dest_dir);
            let jre_handle = manager.prepare_batch(Box::new(jre_batch)).await?;
            download_with_progress(version.clone(), jre_handle, queue.clone()).await?;
        }

        let java_path = java_pref
            .iter()
            .copied()
            .find(|v| JavaManager::is_installed(*v))
            .map(JavaManager::get_java_binary);

        let Some(java_path) = java_path else {
            return Err(aqua::AquaError::Other(
                "No se pudo instalar un runtime Java compatible".into(),
            ));
        };

        // Always download/verify base MC files before Forge.
        download_base_mc(version.clone(), gv, manager, queue).await?;

        let installer_url = aqua::ForgeBatch::resolve_installer_url(gv, fv);
        let batch =
            aqua::ForgeBatch::new(&shared_dir, gv, fv, &installer_url, Some(java_path)).await?;
        let handle = manager.prepare_batch(Box::new(batch)).await?;
        download_with_progress(version, handle, queue.clone()).await
    }

    async fn process_neoforge_version(
        shared_dir: PathBuf,
        manager: &DownloadManager,
        queue: &Arc<DownloadQueue>,
        version: Arc<str>,
    ) -> Result<(), aqua::AquaError> {
        let parts: Vec<&str> = version.split("-neoforge-").collect();
        let (gv, nv) = match parts.as_slice() {
            [gv, nv] => (*gv, *nv),
            _ => {
                return Err(aqua::AquaError::Other(format!(
                    "NeoForge version format invalid: {}",
                    version
                )));
            }
        };

        // NeoForge only exists for MC 1.20.2+ / future year-based versions.
        // MC 1.21+ -> Java 21; MC 1.20.x -> Java 17.
        let java_pref: &[u8] = match parse_mc_major_minor(gv) {
            Some((1, n)) if n >= 21 => &[21, 17, 8],
            _ => &[17, 21, 8],
        };

        if !java_pref.iter().any(|v| JavaManager::is_installed(*v)) {
            let java_version = java_pref[0];
            emit_stage(
                &version,
                "jre",
                Some(format!("Java {} para NeoForge {}", java_version, version)),
            );
            let pkg = JavaManager::get_latest_package(java_version)
                .await
                .map_err(|e| aqua::AquaError::Other(e.to_string()))?;
            let dest_dir = JavaManager::get_jre_dir(java_version);
            let jre_batch = JreBatch::new(java_version, pkg, dest_dir);
            let jre_handle = manager.prepare_batch(Box::new(jre_batch)).await?;
            download_with_progress(version.clone(), jre_handle, queue.clone()).await?;
        }

        let java_path = java_pref
            .iter()
            .copied()
            .find(|v| JavaManager::is_installed(*v))
            .map(JavaManager::get_java_binary);

        let Some(java_path) = java_path else {
            return Err(aqua::AquaError::Other(
                "No se pudo instalar un runtime Java compatible".into(),
            ));
        };

        // Always download/verify base MC files before NeoForge.
        download_base_mc(version.clone(), gv, manager, queue).await?;

        let installer_url = aqua::NeoForgeBatch::resolve_installer_url(nv);
        let batch =
            aqua::NeoForgeBatch::new(&shared_dir, gv, nv, &installer_url, Some(java_path)).await?;
        let handle = manager.prepare_batch(Box::new(batch)).await?;
        download_with_progress(version, handle, queue.clone()).await
    }
}

fn emit_and_set_error(queue: &DownloadQueue, version: &Arc<str>, msg: String) {
    error!("{}", msg);
    emit(AppEvent::DError {
        version: version.clone(),
        message: CompactString::from(&*msg),
    });
    if let Some(mut state) = queue.active.get_mut(version) {
        state.status = DownloadStatus::Error(msg);
    }
}

fn emit_stage(version: &Arc<str>, stage: &'static str, info: Option<String>) {
    emit(AppEvent::DStage {
        version: version.clone(),
        stage: Cow::Borrowed(stage),
        info,
    });
}

async fn download_with_progress(
    version: Arc<str>,
    handle: aqua::DownloadHandle,
    queue: Arc<DownloadQueue>,
) -> Result<(), aqua::AquaError> {
    let (tx, rx) = watch::channel(DownloadProgress::empty(handle.progress().1));
    let monitor = monitor_download_progress(version, rx, queue);
    let (result, ()) = tokio::join!(handle.download_all(Some(tx)), monitor);
    result
}

async fn download_base_mc(
    version: Arc<str>,
    game_version: &str,
    manager: &DownloadManager,
    queue: &Arc<DownloadQueue>,
) -> Result<(), aqua::AquaError> {
    emit_stage(&version, "mc", Some(format!("Minecraft {}", game_version)));
    let base_handle = manager.prepare(game_version).await?;
    download_with_progress(version, base_handle, queue.clone()).await
}

fn parse_fabric_version(version: &str) -> Option<(String, String)> {
    let rest = version.strip_prefix("fabric-loader-")?;
    let (loader, game) = rest.rsplit_once('-')?;
    Some((game.to_string(), loader.to_string()))
}

fn parse_quilt_version(version: &str) -> Option<(String, String)> {
    let rest = version.strip_prefix("quilt-loader-")?;
    let (loader, game) = rest.rsplit_once('-')?;
    Some((game.to_string(), loader.to_string()))
}

fn parse_mc_major_minor(version: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    match parts.as_slice() {
        [major] => major.parse().ok().map(|m| (m, 0)),
        [major, minor] => {
            let m = major.parse().ok()?;
            let n = minor.parse().ok()?;
            Some((m, n))
        }
        _ => None,
    }
}

async fn monitor_download_progress(
    version: Arc<str>,
    mut progress_rx: watch::Receiver<DownloadProgress>,
    queue: Arc<DownloadQueue>,
) {
    let mut last_stage: Option<String> = None;

    loop {
        if progress_rx.changed().await.is_err() {
            break;
        }
        let progress = progress_rx.borrow().clone();

        if let Some(mut state) = queue.active.get_mut(&version) {
            state.stage = Cow::Borrowed(progress.stage.as_str());
            state.item_current = progress.item_current as u64;
            state.item_total = progress.item_total as u64;
            state.bytes_current = progress.bytes_current;
            state.bytes_total = progress.bytes_total;
            state.current_item = progress.current_item.clone();
        }

        let stage_str = progress.stage.as_str().to_string();
        let current_item = progress.current_item.clone();

        emit(AppEvent::DProgress {
            version: version.clone(),
            stage: Cow::Borrowed(progress.stage.as_str()),
            item_current: progress.item_current as u64,
            item_total: progress.item_total as u64,
            bytes_current: progress.bytes_current,
            bytes_total: progress.bytes_total,
            current_item,
        });

        if last_stage.as_ref() != Some(&stage_str) {
            emit(AppEvent::DStage {
                version: version.clone(),
                stage: Cow::Borrowed(progress.stage.as_str()),
                info: progress.current_item,
            });
            last_stage = Some(stage_str);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_state_pending() {
        let s = DownloadState::new(Arc::from("1.21"));
        assert_eq!(s.status, DownloadStatus::Pending);
        assert!(s.is_active());
    }

    #[test]
    fn test_download_state_not_active_done() {
        let mut s = DownloadState::new(Arc::from("1.21"));
        s.status = DownloadStatus::Done;
        assert!(!s.is_active());
    }

    #[test]
    fn test_download_state_not_active_error() {
        let mut s = DownloadState::new(Arc::from("1.21"));
        s.status = DownloadStatus::Error("err".into());
        assert!(!s.is_active());
    }
}
