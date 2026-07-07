use crate::core::{AppError, FsError, PathManager};
use aqua::{JrePackage, JreStatus, ZuluApi};
use std::path::PathBuf;
use tokio::fs;
use tracing::info;

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

}
