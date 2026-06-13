use crate::core::{AppError, FsError, PathManager};
use aqua::{JrePackage, JreStatus, ZuluApi};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{error, info, warn};

pub struct JavaManager;

impl JavaManager {
    pub fn get_runtimes_dir() -> PathBuf {
        PathManager::get().get_shared_dir().join("runtimes")
    }

    pub fn get_jre_dir(version: u8) -> PathBuf {
        Self::get_runtimes_dir().join(format!("jre{}", version))
    }

    pub fn get_java_binary(version: u8) -> PathBuf {
        let dir = Self::get_jre_dir(version);
        if cfg!(target_os = "windows") {
            dir.join("bin").join("javaw.exe")
        } else {
            dir.join("bin").join("java")
        }
    }

    pub fn is_installed(version: u8) -> bool {
        Self::get_java_binary(version).exists()
    }

    pub async fn get_status(version: u8) -> Result<JreStatus, AppError> {
        let installed = Self::is_installed(version);
        let java_version = if installed {
            Self::detect_java_version(version).await
        } else {
            None
        };

        Ok(JreStatus {
            version,
            installed,
            java_version,
        })
    }

    pub async fn get_latest_package(version: u8) -> Result<JrePackage, AppError> {
        ZuluApi::get_latest_package(version)
            .await
            .map_err(|e| AppError::CoreError(crate::core::CoreError::Other(e.to_string())))
    }

    pub async fn install(version: u8) -> Result<(), AppError> {
        let pkg = Self::get_latest_package(version).await?;
        info!("Installing JRE {}: {}", version, pkg.filename);

        let dest_dir = Self::get_jre_dir(version);

        if dest_dir.exists() {
            fs::remove_dir_all(&dest_dir).await.map_err(|e| {
                AppError::Fs(FsError::Remove {
                    path: dest_dir.to_string_lossy().to_string(),
                    source: e,
                })
            })?;
        }

        ZuluApi::download_and_extract(&pkg, &dest_dir)
            .await
            .map_err(|e| {
                error!("Failed to download/extract JRE {}: {:?}", version, e);
                AppError::CoreError(crate::core::CoreError::Other(e.to_string()))
            })?;

        if !Self::is_installed(version) {
            // Try to find java binary in subdirectories
            Self::find_and_relocate_java(version, &dest_dir, &pkg).await?;
        }

        info!("JRE {} installed successfully at {:?}", version, dest_dir);
        Ok(())
    }

    pub async fn uninstall(version: u8) -> Result<(), AppError> {
        let dir = Self::get_jre_dir(version);
        if dir.exists() {
            fs::remove_dir_all(&dir).await.map_err(|e| {
                AppError::Fs(FsError::Remove {
                    path: dir.to_string_lossy().to_string(),
                    source: e,
                })
            })?;
            info!("JRE {} uninstalled", version);
        }
        Ok(())
    }

    async fn detect_java_version(version: u8) -> Option<String> {
        let java_bin = Self::get_java_binary(version);
        if !java_bin.exists() {
            return None;
        }

        let output = tokio::process::Command::new(&java_bin)
            .arg("-version")
            .output()
            .await
            .ok()?;

        let version_str = String::from_utf8_lossy(if output.stderr.is_empty() {
            &output.stdout
        } else {
            &output.stderr
        })
        .to_string();

        // Parse version line like: openjdk version "21.0.11" 2025-...
        let version_line = version_str.lines().next()?;
        let parsed_version = version_line
            .split('"')
            .nth(1)
            .or_else(|| {
                version_line
                    .split_whitespace()
                    .find(|s| s.chars().next().is_some_and(|c| c.is_ascii_digit()))
            })
            .map(|s| s.to_string());

        info!("Detected Java {} version: {:?}", version, parsed_version);
        parsed_version
    }

    /// After extracting a tar.gz/zip, the java binary might be inside a subdirectory
    /// like `zulu21.50.19-ca-jre21.0.11-linux_x64/`. We need to move it to the right place.
    async fn find_and_relocate_java(
        version: u8,
        dest_dir: &Path,
        _pkg: &JrePackage,
    ) -> Result<(), AppError> {
        let expected = Self::get_java_binary(version);
        if expected.exists() {
            return Ok(());
        }

        if !dest_dir.exists() {
            return Err(AppError::CoreError(crate::core::CoreError::Other(format!(
                "JRE {} destination directory not found after extraction",
                version
            ))));
        }

        let mut entries = fs::read_dir(dest_dir).await.map_err(|e| {
            AppError::Fs(FsError::ReadDir {
                path: dest_dir.to_string_lossy().to_string(),
                source: e,
            })
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            AppError::Fs(FsError::ReadDir {
                path: dest_dir.to_string_lossy().to_string(),
                source: e,
            })
        })? {
            if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                let subdir = entry.path();
                let java_in_subdir = if cfg!(target_os = "windows") {
                    subdir.join("bin").join("javaw.exe")
                } else {
                    subdir.join("bin").join("java")
                };

                if java_in_subdir.exists() {
                    info!(
                        "Found java binary in subdirectory {:?}, moving contents to {:?}",
                        subdir, dest_dir
                    );
                    Self::move_contents(&subdir, dest_dir)?;
                    if let Err(e) = fs::remove_dir_all(&subdir).await {
                        warn!("Error al eliminar directorio temporal {:?}: {}", subdir, e);
                    }
                    return Ok(());
                }
            }
        }

        Err(AppError::CoreError(crate::core::CoreError::Other(format!(
            "Java binary not found after extracting JRE {}",
            version
        ))))
    }

    fn move_contents(src: &Path, dest: &Path) -> Result<(), AppError> {
        let entries = std::fs::read_dir(src).map_err(|e| {
            AppError::Fs(FsError::ReadDir {
                path: src.to_string_lossy().to_string(),
                source: e,
            })
        })?;

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let src_path = entry.path();
            let dest_path = dest.join(&file_name);

            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Err(e) = std::fs::create_dir_all(&dest_path) {
                    warn!("Error al crear directorio {:?}: {}", dest_path, e);
                }
                Self::move_contents(&src_path, &dest_path)?;
                if let Err(e) = std::fs::remove_dir_all(&src_path) {
                    warn!("Error al eliminar directorio fuente {:?}: {}", src_path, e);
                }
            } else if dest_path.exists() {
                if let Err(e) = std::fs::remove_file(&dest_path) {
                    warn!("Error al eliminar archivo destino {:?}: {}", dest_path, e);
                }
                if let Err(e) = std::fs::rename(&src_path, &dest_path) {
                    warn!(
                        "Error al renombrar {:?} -> {:?}: {}",
                        src_path, dest_path, e
                    );
                }
            } else {
                std::fs::rename(&src_path, &dest_path).map_err(|e| {
                    AppError::Fs(FsError::Rename {
                        from: src_path.to_string_lossy().to_string(),
                        to: dest_path.to_string_lossy().to_string(),
                        source: e,
                    })
                })?;
            }
        }

        Ok(())
    }
}
