use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use super::batch::{DownloadBatch, DownloadItemSpec};
use crate::AquaError;
use crate::jre::{JrePackage, ZuluApi};
use crate::progress::ProgressSender;

pub struct JreBatch {
    pub version: u8,
    pkg: JrePackage,
    dest_dir: PathBuf,
}

impl JreBatch {
    pub fn new(version: u8, pkg: JrePackage, dest_dir: PathBuf) -> Self {
        Self {
            version,
            pkg,
            dest_dir,
        }
    }
}

impl DownloadBatch for JreBatch {
    fn name(&self) -> String {
        format!("jre-{}", self.version)
    }

    fn items(&self) -> &[DownloadItemSpec] {
        &[]
    }

    fn finalize(
        &self,
        progress_tx: Option<ProgressSender>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        let pkg = self.pkg.clone();
        let dest_dir = self.dest_dir.clone();
        let version = self.version;

        Box::pin(async move {
            let label = format!("Java {}", version);
            ZuluApi::download_and_extract(&pkg, &dest_dir, progress_tx, label).await?;
            Ok(())
        })
    }
}
