use crate::core::path_manager::PathManager;
use crate::core::{AppEvent, emit};
use crate::services::java_manager::JavaManager;
use aqua::{DownloadBatch, DownloadManager, DownloadProgress, DownloadProgressType};
use compact_str::CompactString;
use dashmap::DashMap;
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

const MAX_RETRIES: u8 = 3;
const RETRY_DELAYS: [u64; 3] = [5, 15, 45];

static DOWNLOAD_QUEUE: OnceLock<Arc<DownloadQueue>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Done,
    Error(String),
    Retrying(u8, u8),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadState {
    pub version: Arc<str>,
    pub status: DownloadStatus,
    pub current: u64,
    pub total: u64,
}

impl DownloadState {
    fn new(version: Arc<str>) -> Self {
        Self {
            version,
            status: DownloadStatus::Pending,
            current: 0,
            total: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            DownloadStatus::Pending | DownloadStatus::Downloading | DownloadStatus::Retrying(..)
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
                current: 0,
                total: 0,
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
            Self::process_with_retry(&queue, version).await;
        }

        error!("Worker de descargas terminó inesperadamente — el channel fue cerrado");
    }

    async fn process_with_retry(queue: &Arc<DownloadQueue>, version: Arc<str>) {
        for attempt in 0..=MAX_RETRIES {
            let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
            let manager = DownloadManager::new(shared_dir.clone());

            if attempt > 0 {
                warn!(
                    "Reintentando {} (intento {}/{})",
                    version, attempt, MAX_RETRIES
                );
                if let Some(mut state) = queue.active.get_mut(&version) {
                    state.status = DownloadStatus::Retrying(attempt, MAX_RETRIES);
                }
                emit(AppEvent::DRetry {
                    version: version.clone(),
                    attempt,
                    max: MAX_RETRIES,
                });
                let delay = RETRY_DELAYS[(attempt - 1) as usize];
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }

            if let Some(mut state) = queue.active.get_mut(&version) {
                state.status = DownloadStatus::Downloading;
            } else {
                error!("State no encontrado para {}, saltando", version);
                break;
            }

            // Check if there's a pending batch for this version (e.g. JRE)
            if let Some(batch) = queue.pending_batches.remove(&version) {
                let (_, batch) = batch;
                let (tx, progress_rx) = mpsc::channel::<DownloadProgress>(100);
                let monitor =
                    monitor_download_progress(version.clone(), progress_rx, queue.clone());

                let (dl_result, ()) =
                    tokio::join!(async { batch.finalize(Some(tx)).await }, monitor);

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
                        break;
                    }
                    Err(e) => {
                        if attempt < MAX_RETRIES {
                            continue;
                        }
                        emit_and_set_error(
                            queue,
                            &version,
                            format!("No se pudo completar el batch {}: {:?}", version, e),
                        );
                        break;
                    }
                }
            }

            // Detect Forge versions: "{mc}-forge-{forge}"
            let download_handle = match version.contains("-forge-") {
                true => {
                    let parts: Vec<&str> = version.split("-forge-").collect();
                    if parts.len() == 2 {
                        let gv = parts[0];
                        let fv = parts[1];

                        let mc_jar = shared_dir
                            .join("versions")
                            .join(gv)
                            .join(format!("{gv}.jar"));
                        if !mc_jar.exists() {
                            info!("Base MC {gv} jar not found, downloading before Forge...");
                            if let Ok(base_handle) = manager.prepare(gv).await {
                                let _ = base_handle.download_all(None).await;
                            }
                        }

                        let java_path = [21u8, 17, 8]
                            .into_iter()
                            .find(|v| JavaManager::is_installed(*v))
                            .map(|v| JavaManager::get_java_binary(v));

                        let installer_url = aqua::ForgeBatch::resolve_installer_url(gv, fv);
                        match aqua::ForgeBatch::new(&shared_dir, gv, fv, &installer_url, java_path).await {
                            Ok(batch) => match manager.prepare_batch(Box::new(batch)).await {
                                Ok(h) => h,
                                Err(e) => {
                                    if attempt < MAX_RETRIES { continue; }
                                    emit_and_set_error(queue, &version, format!("No se pudo preparar Forge: {:?}", e));
                                    break;
                                }
                            },
                            Err(e) => {
                                if attempt < MAX_RETRIES { continue; }
                                emit_and_set_error(queue, &version, format!("No se pudo crear Forge batch: {:?}", e));
                                break;
                            }
                        }
                    } else {
                        emit_and_set_error(queue, &version, format!("Forge version format invalid: {}", version));
                        break;
                    }
                }
                false => {
                    match manager.prepare(&version).await {
                        Ok(h) => h,
                        Err(_) => {
                            let deps = zellkern::resolve_dependencies(&version);
                            let mc_version = deps.first().filter(|v| *v != version.as_ref()).cloned();

                            if let Some(gv) = mc_version {
                                info!("Loader {} falló, descargando base MC {} primero...", version, gv);
                                if let Ok(base_handle) = manager.prepare(&gv).await {
                                    let _ = base_handle.download_all(None).await;
                                }
                                match manager.prepare(&version).await {
                                    Ok(h) => h,
                                    Err(e) => {
                                        if attempt < MAX_RETRIES { continue; }
                                        emit_and_set_error(queue, &version, format!("No se pudo descargar loader después de restaurar base: {:?}", e));
                                        break;
                                    }
                                }
                            } else {
                                emit_and_set_error(queue, &version, format!("La versión solicitada no existe: {}", version));
                                break;
                            }
                        }
                    }
                }
            };

            let (tx, progress_rx) = mpsc::channel::<DownloadProgress>(100);
            let monitor = monitor_download_progress(version.clone(), progress_rx, queue.clone());
            let (dl_result, ()) = tokio::join!(download_handle.download_all(Some(tx)), monitor);

            match dl_result {
                Ok(_) => {
                    info!("Versión {} descargada correctamente", version);
                    if let Some(mut state) = queue.active.get_mut(&version) {
                        state.status = DownloadStatus::Done;
                    }
                    emit(AppEvent::DFinish { version });
                    break;
                }
                Err(e) => {
                    if attempt < MAX_RETRIES {
                        continue;
                    }
                    emit_and_set_error(queue, &version, format!("No se pudo descargar {}: {:?}", version, e));
                    break;
                }
            }
        }

        queue.active.retain(|_, s| s.is_active());
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

fn d_type_str(t: &DownloadProgressType) -> &'static str {
    match t {
        DownloadProgressType::Library => "Library",
        DownloadProgressType::Asset => "Asset",
        DownloadProgressType::Native => "Native",
        DownloadProgressType::Client => "Client",
        DownloadProgressType::Verifying => "Verifying",
        DownloadProgressType::Generic => "Generic",
        DownloadProgressType::Processing => "Processing",
        DownloadProgressType::Jre => "Jre",
    }
}

async fn monitor_download_progress(
    version: Arc<str>,
    mut progress_rx: mpsc::Receiver<DownloadProgress>,
    queue: Arc<DownloadQueue>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(75));
    interval.tick().await;
    let mut latest: Option<DownloadProgress> = None;

    loop {
        tokio::select! {
            biased;
            _ = interval.tick() => {
                if let Some(ref p) = latest {
                    emit(AppEvent::DProgress {
                        version: version.clone(),
                        current: p.current as u32,
                        total: p.total as u32,
                        d_type: Cow::Borrowed(d_type_str(&p.download_type)),
                    });
                }
            }
            maybe = progress_rx.recv() => {
                match maybe {
                    Some(progress) => {
                        if let Some(mut state) = queue.active.get_mut(&version) {
                            state.current = progress.current as u64;
                            state.total = progress.total as u64;
                        }
                        latest = Some(progress);
                    }
                    None => break,
                }
            }
        }
    }

    if let Some(p) = latest {
        emit(AppEvent::DProgress {
            version: version.clone(),
            current: p.current as u32,
            total: p.total as u32,
            d_type: Cow::Borrowed(d_type_str(&p.download_type)),
        });
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
