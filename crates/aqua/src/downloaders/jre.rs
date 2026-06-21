use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use super::batch::{DownloadBatch, DownloadItemSpec};
use crate::jre::{JrePackage, ZuluApi};
use crate::types::{DownloadProgress, DownloadProgressInfo, DownloadProgressType};
use crate::AquaError;

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
        progress_tx: Option<Sender<DownloadProgress>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        let pkg = self.pkg.clone();
        let dest_dir = self.dest_dir.clone();
        let version = self.version;

        Box::pin(async move {
            let (bytes_tx, mut bytes_rx) = tokio::sync::mpsc::channel::<(u64, u64)>(32);
            let name_clone: Arc<String> = Arc::new(format!("jre-{}", version));

            let progress_fwd = tokio::spawn(async move {
                while let Some((current, total)) = bytes_rx.recv().await {
                    if let Some(ref tx) = progress_tx {
                        let _ = tx
                            .send(DownloadProgress {
                                current: current as usize,
                                total: total as usize,
                                info: DownloadProgressInfo {
                                    name: format!("Java {}", version),
                                    version: name_clone.clone(),
                                },
                                download_type: DownloadProgressType::Jre,
                            })
                            .await;
                    }
                }
            });

            ZuluApi::download_and_extract(&pkg, &dest_dir, Some(bytes_tx)).await?;
            let _ = progress_fwd.await;
            Ok(())
        })
    }
}
