use crate::core::path_manager::PathManager;
use crate::core::{AppEvent, emit};
use aqua::{DownloadManager, DownloadProgress, DownloadProgressType};
use dashmap::DashMap;
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc;
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
            DownloadStatus::Pending | DownloadStatus::Downloading
        )
    }
}

pub struct DownloadQueue {
    sender: mpsc::Sender<Arc<str>>,
    active: DashMap<Arc<str>, DownloadState>,
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

    async fn worker(mut rx: mpsc::Receiver<Arc<str>>, queue: Arc<DownloadQueue>) {
        while let Some(version) = rx.recv().await {
            let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
            let manager = DownloadManager::new(shared_dir);

            if let Some(mut state) = queue.active.get_mut(&version) {
                state.status = DownloadStatus::Downloading;
            } else {
                error!("State no encontrado para {}, saltando", version);
                continue;
            }

            let download_handle = match manager.prepare(&version).await {
                Ok(h) => h,
                Err(_) if version.starts_with("fabric-loader-") => {
                    let gv = version.split('-').next_back().unwrap_or("");
                    match manager.prepare(gv).await {
                        Ok(h) => h,
                        Err(_) => {
                            let msg = format!("No se pudo resolver base {} para Fabric", gv);
                            error!("{}", msg);
                            if let Some(mut state) = queue.active.get_mut(&version) {
                                state.status = DownloadStatus::Error(msg);
                            }
                            continue;
                        }
                    }
                }
                Err(_) => {
                    let msg = format!("La versión solicitada no existe: {}", version);
                    error!("{}", msg);
                    if let Some(mut state) = queue.active.get_mut(&version) {
                        state.status = DownloadStatus::Error(msg);
                    }
                    continue;
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
                }
                Err(e) => {
                    let msg = format!("No se pudo descargar {}: {:?}", version, e);
                    error!("{}", msg);
                    if let Some(mut state) = queue.active.get_mut(&version) {
                        state.status = DownloadStatus::Error(msg);
                    }
                }
            }

            queue.active.retain(|_, s| s.is_active());
        }

        error!("Worker de descargas terminó inesperadamente — el channel fue cerrado");
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
