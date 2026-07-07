use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

pub type ProgressSender = tokio::sync::watch::Sender<DownloadProgress>;

/// Logical stage of a download/installation.
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStage {
    /// Resolving version/profile metadata.
    Resolving,
    /// Downloading a library.
    Library,
    /// Downloading an asset object.
    Asset,
    /// Downloading a native library.
    Native,
    /// Downloading the client/server jar.
    Client,
    /// Verifying an existing file.
    Verifying,
    /// Extracting an archive (e.g. natives, JRE).
    Extracting,
    /// Running a post-processor.
    Processing,
    /// Downloading/installing a Java runtime.
    Jre,
    /// Fallback/unknown stage.
    Generic,
}

impl DownloadStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            DownloadStage::Resolving => "resolving",
            DownloadStage::Library => "library",
            DownloadStage::Asset => "asset",
            DownloadStage::Native => "native",
            DownloadStage::Client => "client",
            DownloadStage::Verifying => "verifying",
            DownloadStage::Extracting => "extracting",
            DownloadStage::Processing => "processing",
            DownloadStage::Jre => "jre",
            DownloadStage::Generic => "generic",
        }
    }
}

/// Snapshot of progress for a download task.
#[derive(Debug, Clone, PartialEq)]
pub struct DownloadProgress {
    pub stage: DownloadStage,
    pub item_current: usize,
    pub item_total: usize,
    pub bytes_current: u64,
    pub bytes_total: u64,
    pub current_item: Option<String>,
    pub current_item_bytes: u64,
    pub current_item_total: Option<u64>,
}

impl DownloadProgress {
    pub fn empty(item_total: usize) -> Self {
        Self {
            stage: DownloadStage::Generic,
            item_current: 0,
            item_total,
            bytes_current: 0,
            bytes_total: 0,
            current_item: None,
            current_item_bytes: 0,
            current_item_total: None,
        }
    }
}

/// Shared progress state for a whole batch.
#[derive(Debug)]
pub struct ProgressState {
    pub item_total: usize,
    pub bytes_total: u64,
    pub item_current: AtomicUsize,
    pub bytes_current: AtomicU64,
    pub current_item: Mutex<Option<String>>,
    pub current_stage: Mutex<DownloadStage>,
    pub current_item_bytes: AtomicU64,
    pub current_item_total: AtomicU64,
}

impl ProgressState {
    pub fn new(item_total: usize, bytes_total: u64) -> Arc<Self> {
        Arc::new(Self {
            item_total,
            bytes_total,
            item_current: AtomicUsize::new(0),
            bytes_current: AtomicU64::new(0),
            current_item: Mutex::new(None),
            current_stage: Mutex::new(DownloadStage::Generic),
            current_item_bytes: AtomicU64::new(0),
            current_item_total: AtomicU64::new(0),
        })
    }

    pub async fn set_current_item(
        &self,
        name: Option<String>,
        total: Option<u64>,
        stage: DownloadStage,
    ) {
        *self.current_item.lock().await = name;
        *self.current_stage.lock().await = stage;
        self.current_item_bytes.store(0, Ordering::Relaxed);
        self.current_item_total
            .store(total.unwrap_or(0), Ordering::Relaxed);
    }

    pub async fn clear_current_item(&self) {
        *self.current_item.lock().await = None;
        *self.current_stage.lock().await = DownloadStage::Generic;
        self.current_item_bytes.store(0, Ordering::Relaxed);
        self.current_item_total.store(0, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> DownloadProgress {
        let current_item_total = self.current_item_total.load(Ordering::Relaxed);
        let stage = self
            .current_stage
            .try_lock()
            .map(|g| g.clone())
            .unwrap_or(DownloadStage::Generic);
        DownloadProgress {
            stage,
            item_current: self.item_current.load(Ordering::Relaxed),
            item_total: self.item_total,
            bytes_current: self.bytes_current.load(Ordering::Relaxed),
            bytes_total: self.bytes_total,
            current_item: self
                .current_item
                .try_lock()
                .ok()
                .and_then(|g| g.clone()),
            current_item_bytes: self.current_item_bytes.load(Ordering::Relaxed),
            current_item_total: if current_item_total == 0 {
                None
            } else {
                Some(current_item_total)
            },
        }
    }
}

/// Per-item reporter attached to a shared `ProgressState`.
#[derive(Clone, Debug)]
pub struct DownloadReporter {
    state: Arc<ProgressState>,
    item_bytes: Arc<AtomicU64>,
}

impl DownloadReporter {
    pub fn new(state: Arc<ProgressState>) -> Self {
        Self {
            state,
            item_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Call at the start of each attempt so a previous failed attempt's
    /// partial bytes are removed from the global counter.
    pub fn reset_attempt(&self) {
        let prev = self.item_bytes.swap(0, Ordering::Relaxed);
        if prev == 0 {
            return;
        }
        let mut current = self.state.bytes_current.load(Ordering::Relaxed);
        loop {
            let new = current.saturating_sub(prev);
            match self.state.bytes_current.compare_exchange_weak(
                current,
                new,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    /// Report `delta` bytes successfully written for the current attempt.
    pub fn report_delta(&self, delta: u64) {
        if delta == 0 {
            return;
        }
        self.item_bytes.fetch_add(delta, Ordering::Relaxed);
        self.state
            .current_item_bytes
            .fetch_add(delta, Ordering::Relaxed);
        self.state.bytes_current.fetch_add(delta, Ordering::Relaxed);
    }

    /// Use when the file already exists and is valid: all `size` bytes are done.
    pub fn commit_known_size(&self, size: u64) {
        let current = self.item_bytes.swap(size, Ordering::Relaxed);
        let delta = size.saturating_sub(current);
        if delta > 0 {
            self.state
                .current_item_bytes
                .fetch_add(delta, Ordering::Relaxed);
            self.state.bytes_current.fetch_add(delta, Ordering::Relaxed);
        }
    }
}
