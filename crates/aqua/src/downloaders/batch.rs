use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use crate::AquaError;
use crate::progress::{DownloadStage, ProgressSender};

#[derive(Debug, Clone)]
pub struct DownloadItemSpec {
    pub url: String,
    pub destination: PathBuf,
    pub fallback_url: Option<String>,
    pub expected_hash: String,
    pub label: String,
    pub required: bool,
    pub stage: DownloadStage,
    pub size: Option<u64>,
}

impl DownloadItemSpec {
    pub fn new(url: impl Into<String>, destination: PathBuf, label: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            fallback_url: None,
            destination,
            expected_hash: String::new(),
            label: label.into(),
            required: true,
            stage: DownloadStage::Generic,
            size: None,
        }
    }

    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.expected_hash = hash.into();
        self
    }

    pub fn with_fallback_url(mut self, fallback_url: impl Into<String>) -> Self {
        self.fallback_url = Some(fallback_url.into());
        self
    }

    pub fn non_required(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_stage(mut self, stage: DownloadStage) -> Self {
        self.stage = stage;
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }
}

pub trait DownloadBatch: Send + Sync {
    fn name(&self) -> String;

    fn items(&self) -> &[DownloadItemSpec];

    fn prepare(&self) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn finalize(
        &self,
        _progress_tx: Option<ProgressSender>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

// ─── GenericBatch ─────────────────────────────────────────────────────────────

pub struct GenericBatch {
    name: String,
    items: Vec<DownloadItemSpec>,
}

impl GenericBatch {
    pub fn new(name: impl Into<String>, items: Vec<DownloadItemSpec>) -> Self {
        Self {
            name: name.into(),
            items,
        }
    }
}

impl DownloadBatch for GenericBatch {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn items(&self) -> &[DownloadItemSpec] {
        &self.items
    }
}
