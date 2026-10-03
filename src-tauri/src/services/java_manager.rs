use crate::core::{AppError, FsError, PathManager};
use aqua::{JrePackage, JreProviderChain, JreStatus, JreVendor};
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
        let _shared_guard = super::shared_storage::acquire().await;
        let installed = Self::is_installed(version);
        let (java_version, vendor) = if installed {
            Self::detect_runtime(version).await
        } else {
            (None, None)
        };

        Ok(JreStatus {
            version,
            installed,
            java_version,
            vendor,
        })
    }

    pub async fn get_latest_package(
        version: u8,
        vendor: Option<JreVendor>,
    ) -> Result<JrePackage, AppError> {
        JreProviderChain::get_package(version, vendor)
            .await
            .map_err(|e| AppError::CoreError(crate::core::CoreError::Other(e.to_string())))
    }

    /// Proveedores con un build descargable para la versión indicada.
    pub async fn get_available_vendors(version: u8) -> Result<Vec<JreVendor>, AppError> {
        JreProviderChain::available_vendors(version)
            .await
            .map_err(|e| AppError::CoreError(crate::core::CoreError::Other(e.to_string())))
    }

    pub async fn uninstall(version: u8) -> Result<(), AppError> {
        let exclusive =
            super::shared_storage::try_exclusive().map_err(crate::core::CoreError::Other)?;
        tokio::spawn(async move {
            let _exclusive = exclusive;
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
        })
        .await
        .map_err(|e| crate::core::CoreError::Other(e.to_string()))?
    }

    /// Devuelve la versión de Java y el identificador del proveedor detectado
    /// en el runtime administrado.
    async fn detect_runtime(version: u8) -> (Option<String>, Option<String>) {
        let Some(output) = Self::fetch_version_output(version).await else {
            return (None, None);
        };

        let parsed_version = parse_java_version(&output);
        let vendor = JreVendor::detect(&output).map(|vendor| vendor.id().to_string());

        info!(
            "Detected Java {} version: {:?} (vendor: {:?})",
            version, parsed_version, vendor
        );

        (parsed_version, vendor)
    }

    async fn fetch_version_output(version: u8) -> Option<String> {
        let java_bin = Self::get_java_binary(version);
        if !java_bin.exists() {
            return None;
        }

        let output = tokio::process::Command::new(&java_bin)
            .arg("-version")
            .output()
            .await
            .ok()?;

        Some(
            String::from_utf8_lossy(if output.stderr.is_empty() {
                &output.stdout
            } else {
                &output.stderr
            })
            .to_string(),
        )
    }
}

/// Extrae la versión de un texto como `openjdk version "21.0.11" 2025-...`.
fn parse_java_version(version_output: &str) -> Option<String> {
    let version_line = version_output.lines().next()?;
    version_line
        .split('"')
        .nth(1)
        .or_else(|| {
            version_line
                .split_whitespace()
                .find(|s| s.chars().next().is_some_and(|c| c.is_ascii_digit()))
        })
        .map(|s| s.to_string())
}

#[cfg(test)]
#[path = "../tests/services/java_manager.rs"]
mod tests;
