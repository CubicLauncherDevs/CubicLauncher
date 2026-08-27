use crate::errors::AquaError;
use crate::jre::types::{ArchiveFormat, JrePackage};
use crate::path_security::safe_join;
use crate::progress::{DownloadProgress, DownloadStage, ProgressSender};
use crate::utilities::{ProgressReporter, download_file};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Downloads a JRE package and extracts it into the requested destination
/// directory.
///
/// The extraction is robust against archive layout changes: instead of
/// assuming the archive top-level directory name matches the filename, the
/// function searches the extracted tree for the Java runtime root (the
/// directory that contains `bin/java` or `bin/javaw.exe`) and moves it to
/// `dest_dir`.
pub async fn download_and_extract(
    pkg: &JrePackage,
    dest_dir: &Path,
    progress_tx: Option<ProgressSender>,
    item_label: String,
) -> Result<(), AquaError> {
    let extract_dir = dest_dir.parent().unwrap_or(dest_dir);
    tokio::fs::create_dir_all(extract_dir).await?;

    let ext = match pkg.archive_format {
        ArchiveFormat::TarGz => "tar.gz",
        ArchiveFormat::Zip => "zip",
    };
    let archive_path = extract_dir.join(format!("jre{}_tmp.{}", pkg.major_version, ext));
    let staging_dir = extract_dir.join(format!("jre{}_stage", pkg.major_version));

    let cleanup = Cleanup {
        archive: archive_path.clone(),
        staging: staging_dir.clone(),
    };

    let total = pkg.size.unwrap_or(0);
    let reporter = progress_tx.as_ref().map(|tx| {
        Box::new(WatchReporter::new(
            tx.clone(),
            DownloadStage::Jre,
            item_label.clone(),
            total,
        )) as Box<dyn ProgressReporter>
    });

    send_progress(
        progress_tx.as_ref(),
        DownloadStage::Jre,
        0,
        1,
        0,
        total,
        Some(item_label.clone()),
    );

    let expected_hash = pkg.sha256_hash.as_deref().unwrap_or("");
    download_file(
        &pkg.download_url,
        &archive_path,
        expected_hash,
        Some(total),
        reporter.as_ref().map(|r| r.as_ref()),
    )
    .await?;

    send_progress(
        progress_tx.as_ref(),
        DownloadStage::Extracting,
        1,
        1,
        total,
        total,
        Some(item_label.clone()),
    );

    // Clean and recreate staging dir before extraction.
    let _ = tokio::fs::remove_dir_all(&staging_dir).await;
    tokio::fs::create_dir_all(&staging_dir).await?;

    match pkg.archive_format {
        ArchiveFormat::TarGz => extract_tar_gz(&archive_path, &staging_dir).await?,
        ArchiveFormat::Zip => extract_zip(&archive_path, &staging_dir).await?,
    }

    send_progress(
        progress_tx.as_ref(),
        DownloadStage::Verifying,
        1,
        1,
        total,
        total,
        Some(item_label.clone()),
    );

    let runtime_root =
        find_runtime_root(&staging_dir, pkg.java_binary_name()).ok_or_else(|| {
            AquaError::Other(format!(
                "No se encontró el binario {} dentro del JRE extraído",
                pkg.java_binary_name()
            ))
        })?;

    // Ensure the destination is clean before installing.
    if dest_dir.exists() {
        tokio::fs::remove_dir_all(dest_dir).await?;
    }
    tokio::fs::rename(&runtime_root, dest_dir).await?;

    let final_binary = dest_dir.join("bin").join(pkg.java_binary_name());
    if !final_binary.exists() {
        return Err(AquaError::Other(format!(
            "El binario Java no existe tras extraer: {}",
            final_binary.display()
        )));
    }

    send_progress(
        progress_tx.as_ref(),
        DownloadStage::Jre,
        1,
        1,
        total,
        total,
        Some(item_label),
    );

    // `cleanup` drops here and schedules best-effort removal of temp files,
    // even if we returned early with an error.
    drop(cleanup);

    Ok(())
}

/// Simple helper to delete temporary files even when extraction fails.
struct Cleanup {
    archive: PathBuf,
    staging: PathBuf,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let archive = self.archive.clone();
        let staging = self.staging.clone();
        // Spawn a detached cleanup task; filesystem cleanup is best-effort.
        tokio::spawn(async move {
            let _ = tokio::fs::remove_file(&archive).await;
            let _ = tokio::fs::remove_dir_all(&staging).await;
        });
    }
}

fn send_progress(
    tx: Option<&ProgressSender>,
    stage: DownloadStage,
    item_current: usize,
    item_total: usize,
    bytes_current: u64,
    bytes_total: u64,
    current_item: Option<String>,
) {
    if let Some(tx) = tx {
        let _ = tx.send(DownloadProgress {
            stage,
            item_current,
            item_total,
            bytes_current,
            bytes_total,
            current_item: current_item.clone(),
            current_item_bytes: bytes_current,
            current_item_total: if bytes_total == 0 {
                None
            } else {
                Some(bytes_total)
            },
        });
    }
}

/// Adapts a `watch::Sender<DownloadProgress>` to the `ProgressReporter` trait
/// so that `download_file` can stream JRE progress back to the UI.
struct WatchReporter {
    tx: ProgressSender,
    stage: DownloadStage,
    label: String,
    total: u64,
    bytes: AtomicU64,
    last_reported: AtomicU64,
}

impl WatchReporter {
    fn new(tx: ProgressSender, stage: DownloadStage, label: String, total: u64) -> Self {
        Self {
            tx,
            stage,
            label,
            total,
            bytes: AtomicU64::new(0),
            last_reported: AtomicU64::new(0),
        }
    }

    fn emit(&self) {
        let current = self.bytes.load(Ordering::Relaxed);
        let _ = self.tx.send(DownloadProgress {
            stage: self.stage.clone(),
            item_current: 0,
            item_total: 1,
            bytes_current: current,
            bytes_total: self.total,
            current_item: Some(self.label.clone()),
            current_item_bytes: current,
            current_item_total: if self.total == 0 {
                None
            } else {
                Some(self.total)
            },
        });
    }
}

impl ProgressReporter for WatchReporter {
    fn reset_attempt(&self) {
        self.bytes.store(0, Ordering::Relaxed);
        self.last_reported.store(0, Ordering::Relaxed);
    }

    fn report_delta(&self, delta: u64) {
        if delta == 0 {
            return;
        }
        let new = self.bytes.fetch_add(delta, Ordering::Relaxed) + delta;
        if new.saturating_sub(self.last_reported.load(Ordering::Relaxed)) >= 256 * 1024 {
            self.last_reported.store(new, Ordering::Relaxed);
            self.emit();
        }
    }

    fn commit_known_size(&self, size: u64) {
        self.bytes.store(size, Ordering::Relaxed);
        self.last_reported.store(size, Ordering::Relaxed);
        self.emit();
    }
}

/// Extract a `.tar.gz` archive into `dest`.
async fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), AquaError> {
    let archive = archive.to_path_buf();
    let dest = dest.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive)?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&dest)?;
        Ok::<_, AquaError>(())
    })
    .await?
}

/// Extract a `.zip` archive into `dest`, rejecting entries that escape the
/// archive root.
async fn extract_zip(archive: &Path, dest: &Path) -> Result<(), AquaError> {
    let dest = dest.to_path_buf();
    let archive = archive.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| AquaError::Other(format!("Failed to open ZIP: {}", e)))?;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| AquaError::Other(format!("Failed to read ZIP entry: {}", e)))?;

            let name = entry.name().to_string();

            let Some(enclosed) = entry.enclosed_name() else {
                log::warn!("JRE ZIP entry with unsafe path ignored: {}", name);
                continue;
            };

            if entry.is_dir() {
                std::fs::create_dir_all(dest.join(enclosed))?;
                continue;
            }

            let out_path =
                safe_join(&dest, enclosed.to_string_lossy().as_ref()).map_err(AquaError::Other)?;
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }

        Ok::<_, AquaError>(())
    })
    .await?
}

/// Searches `base_dir` for a directory that contains `bin/<binary_name>` and
/// returns the shallowest match. This handles archives whose top-level
/// directory name does not match the filename.
fn find_runtime_root(base_dir: &Path, binary_name: &str) -> Option<PathBuf> {
    fn search(dir: &Path, binary_name: &str, best: &mut Option<(usize, PathBuf)>, depth: usize) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let candidate = path.join("bin").join(binary_name);
            if candidate.exists() {
                if best.as_ref().is_none_or(|(d, _)| depth < *d) {
                    *best = Some((depth, path));
                }
                // Prefer the shallowest match; do not recurse deeper once found
                // at this depth for this subtree.
                continue;
            }

            search(&path, binary_name, best, depth + 1);
        }
    }

    let mut best = None;
    search(base_dir, binary_name, &mut best, 0);
    best.map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("cubic_jre_test_{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_find_runtime_root_top_level() {
        let tmp = TempDir::new();
        let root = tmp.path().join("jre21");
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(root.join("bin").join("java"), "").unwrap();

        assert_eq!(
            find_runtime_root(tmp.path(), "java"),
            Some(root.canonicalize().unwrap())
        );
    }

    #[test]
    fn test_find_runtime_root_nested() {
        let tmp = TempDir::new();
        // Simulates archive layout where the runtime is nested one level deep.
        let root = tmp.path().join("zulu21").join("jre");
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(root.join("bin").join("java"), "").unwrap();

        assert_eq!(
            find_runtime_root(tmp.path(), "java"),
            Some(root.canonicalize().unwrap())
        );
    }

    #[test]
    fn test_find_runtime_root_shallowest_wins() {
        let tmp = TempDir::new();
        let shallow = tmp.path().join("shallow");
        let deep = tmp.path().join("deep").join("nested");
        fs::create_dir_all(shallow.join("bin")).unwrap();
        fs::create_dir_all(deep.join("bin")).unwrap();
        fs::write(shallow.join("bin").join("java"), "").unwrap();
        fs::write(deep.join("bin").join("java"), "").unwrap();

        assert_eq!(
            find_runtime_root(tmp.path(), "java"),
            Some(shallow.canonicalize().unwrap())
        );
    }

    #[test]
    fn test_find_runtime_root_missing_binary() {
        let tmp = TempDir::new();
        fs::create_dir_all(tmp.path().join("empty")).unwrap();
        assert_eq!(find_runtime_root(tmp.path(), "java"), None);
    }
}
