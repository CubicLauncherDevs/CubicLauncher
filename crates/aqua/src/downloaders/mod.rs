mod batch;
mod fabric;
mod forge;
mod jre;
mod minecraft;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub use batch::{DownloadBatch, DownloadItemSpec, GenericBatch};
pub use fabric::FabricBatch;
pub use forge::{ForgeBatch, ForgeVersionInfo};
use futures::TryStreamExt;
use futures::stream::{self, StreamExt};
pub use jre::JreBatch;
use log::warn;
pub use minecraft::MinecraftBatch;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;

use crate::AquaError;
use crate::types::{
    DownloadProgress, DownloadProgressInfo, DownloadProgressType, NormalizedVersion,
};
use crate::utilities::download_file;

const DEFAULT_MAX_HANDLES: usize = 2;
const DEFAULT_DOWNLOADS_PER_HANDLE: usize = 128;

// ─── DownloadManager ──────────────────────────────────────────────────────────

pub struct DownloadManager {
    game_path: PathBuf,
    handle_semaphore: Arc<Semaphore>,
    downloads_per_handle: usize,
}

impl DownloadManager {
    pub fn new(game_path: PathBuf) -> Self {
        Self {
            game_path,
            handle_semaphore: Arc::new(Semaphore::new(DEFAULT_MAX_HANDLES)),
            downloads_per_handle: DEFAULT_DOWNLOADS_PER_HANDLE,
        }
    }

    pub fn with_max_handles(mut self, max: usize) -> Self {
        self.handle_semaphore = Arc::new(Semaphore::new(max));
        self
    }

    pub fn with_max_downloads(mut self, max: usize) -> Self {
        self.downloads_per_handle = max;
        self
    }

    /// Minecraft-specific: resolve version from Mojang manifest and download everything.
    pub async fn prepare(&self, version_id: &str) -> Result<DownloadHandle, AquaError> {
        let batch = MinecraftBatch::new(&self.game_path, version_id).await?;
        let name = batch.name();
        let total = batch.items().len();
        let version = Some(batch.version().clone());
        let version_id = batch.version().id.clone();

        Ok(DownloadHandle {
            inner: Arc::new(DownloadInner {
                name,
                _version_id: version_id,
                version,
                batch: Box::new(batch),
                handle_sem: Arc::clone(&self.handle_semaphore),
                max_downloads: self.downloads_per_handle,
                cancel_flag: AtomicBool::new(false),
                join_handle: Mutex::new(None),
                completed_items: Arc::new(AtomicUsize::new(0)),
                total_items: Arc::new(AtomicUsize::new(total)),
            }),
        })
    }

    /// Generic: accept any `DownloadBatch` implementation.
    pub async fn prepare_batch(
        &self,
        batch: Box<dyn DownloadBatch + 'static>,
    ) -> Result<DownloadHandle, AquaError> {
        let name = batch.name();
        let total = batch.items().len();

        Ok(DownloadHandle {
            inner: Arc::new(DownloadInner {
                name,
                _version_id: String::new(),
                version: None,
                batch,
                handle_sem: Arc::clone(&self.handle_semaphore),
                max_downloads: self.downloads_per_handle,
                cancel_flag: AtomicBool::new(false),
                join_handle: Mutex::new(None),
                completed_items: Arc::new(AtomicUsize::new(0)),
                total_items: Arc::new(AtomicUsize::new(total)),
            }),
        })
    }
}

// ─── DownloadHandle ───────────────────────────────────────────────────────────

struct DownloadInner {
    name: String,
    _version_id: String,
    version: Option<NormalizedVersion>,
    batch: Box<dyn DownloadBatch>,
    handle_sem: Arc<Semaphore>,
    max_downloads: usize,
    cancel_flag: AtomicBool,
    join_handle: Mutex<Option<JoinHandle<Result<(), AquaError>>>>,
    completed_items: Arc<AtomicUsize>,
    total_items: Arc<AtomicUsize>,
}

pub struct DownloadHandle {
    inner: Arc<DownloadInner>,
}

impl DownloadHandle {
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Minecraft version info, `None` for non-Minecraft batches.
    pub fn version(&self) -> Option<&NormalizedVersion> {
        self.inner.version.as_ref()
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancel_flag.load(Ordering::Relaxed)
    }

    pub fn progress(&self) -> (usize, usize) {
        let c = self.inner.completed_items.load(Ordering::Relaxed);
        let t = self.inner.total_items.load(Ordering::Relaxed);
        (c, t)
    }

    pub fn cancel(&self) {
        self.inner.cancel_flag.store(true, Ordering::Relaxed);
    }

    pub async fn download_all(
        &self,
        progress_tx: Option<Sender<DownloadProgress>>,
    ) -> Result<(), AquaError> {
        self.start(progress_tx).await?;
        self.wait().await
    }

    pub async fn start(
        &self,
        progress_tx: Option<Sender<DownloadProgress>>,
    ) -> Result<(), AquaError> {
        let mut slot = self.inner.join_handle.lock().await;
        if slot.is_some() {
            return Err(AquaError::Other(
                "Download already in progress or completed".into(),
            ));
        }

        let inner = Arc::clone(&self.inner);
        let handle = tokio::spawn(async move {
            let _handle_permit = Arc::clone(&inner.handle_sem).acquire_owned().await;
            run_download(inner, progress_tx).await
        });

        *slot = Some(handle);
        Ok(())
    }

    pub async fn wait(&self) -> Result<(), AquaError> {
        let handle = self.inner.join_handle.lock().await.take();
        match handle {
            Some(h) => h.await?,
            None => Err(AquaError::Other("Download not started".into())),
        }
    }
}

// ─── Generic download loop ────────────────────────────────────────────────────

async fn run_download(
    inner: Arc<DownloadInner>,
    progress_tx: Option<Sender<DownloadProgress>>,
) -> Result<(), AquaError> {
    if inner.cancel_flag.load(Ordering::Relaxed) {
        return Err(AquaError::Cancelled);
    }

    inner.batch.prepare().await?;

    let total = inner.batch.items().len();
    inner.total_items.store(total, Ordering::Relaxed);

    // Pre-create unique parent directories once
    let mut parents: Vec<&Path> = inner
        .batch
        .items()
        .iter()
        .filter_map(|item| item.destination.parent())
        .collect();
    parents.sort();
    parents.dedup();
    for parent in parents {
        tokio::fs::create_dir_all(parent).await?;
    }

    let completed = Arc::clone(&inner.completed_items);
    let batch_name = inner.batch.name();
    let version_arc = Arc::new(batch_name);
    let max_concurrent = inner.max_downloads;
    let items_vec: Vec<_> = inner.batch.items().to_vec();

    let inner_for_finalize = Arc::clone(&inner);
    let progress_tx_for_stream = progress_tx.clone();

    stream::iter(items_vec.into_iter().map(move |item| {
        let c = Arc::clone(&completed);
        let tx = progress_tx_for_stream.clone();
        let version_arc = Arc::clone(&version_arc);
        let inner = Arc::clone(&inner);

        async move {
            if inner.cancel_flag.load(Ordering::Relaxed) {
                return Err(AquaError::Cancelled);
            }
            if let Err(e) = download_file(&item.url, &item.destination, &item.expected_hash).await {
                if let Some(ref fallback) = item.fallback_url {
                    warn!("Main URL failed. Using fallback: {fallback}");
                    if let Err(_) =
                        download_file(fallback, &item.destination, &item.expected_hash).await
                    {
                        warn!("Fallback failed, using fallback with universal.");
                        download_file(
                            &fallback.replace(".jar", "-universal.jar"),
                            &item.destination,
                            &item.expected_hash,
                        )
                        .await?
                    }
                } else {
                    warn!("Main URL failed but there's no fallback. Aborting");
                    return Err(e);
                }
            }

            let count = c.fetch_add(1, Ordering::Relaxed) + 1;
            report_progress(
                &tx,
                count,
                inner.total_items.load(Ordering::Relaxed),
                DownloadProgressType::Generic,
                item.label,
                &version_arc,
            )
            .await;
            Ok::<_, AquaError>(())
        }
    }))
    .buffer_unordered(max_concurrent)
    .try_collect::<()>()
    .await?;

    inner_for_finalize.batch.finalize(progress_tx).await?;

    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

async fn report_progress(
    tx: &Option<Sender<DownloadProgress>>,
    current: usize,
    total: usize,
    dtype: DownloadProgressType,
    name: impl Into<String>,
    version: &Arc<String>,
) {
    if let Some(tx) = tx {
        let _ = tx
            .send(DownloadProgress {
                current,
                total,
                info: DownloadProgressInfo {
                    name: name.into(),
                    version: Arc::clone(version),
                },
                download_type: dtype,
            })
            .await;
    }
}
