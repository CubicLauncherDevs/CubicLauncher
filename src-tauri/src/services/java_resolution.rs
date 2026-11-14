//! Resolución de la versión de Java que necesita una instancia.

use super::SettingsSnapshot;
use super::java_manager::JavaManager;
use launchwerk::models::{JavaVersion, VersionManifest};
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use zellkern::{GameVersion, Loader};

#[derive(Clone, Debug)]
pub struct JavaResolution {
    /// Antes del override de la instancia.
    pub requested: u8,
    pub major: u8,
    pub path: PathBuf,
}

impl JavaResolution {
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

pub fn manifest_path(shared: &Path, version_id: &str) -> PathBuf {
    shared
        .join("versions")
        .join(version_id)
        .join(format!("{version_id}.json"))
}

/// Java que declara el manifiesto de la versión, incluyendo herencia.
pub fn declared_java(shared: &Path, manifest: &VersionManifest) -> Option<u8> {
    if let Some(java) = manifest.java_version.as_ref() {
        return Some(java.major_version);
    }
    let parent_id = manifest.inherits_from.as_ref()?;
    VersionManifest::from_file(manifest_path(shared, parent_id))
        .ok()
        .and_then(|parent| parent.java_version)
        .map(|java| java.major_version)
}

/// Java inferido del identificador de la versión, con el bump de Forge.
pub fn infer_java_for(version_id: &str) -> u8 {
    let inferred = aqua::infer_java_version(&GameVersion::from_version_id(version_id).mc_version);
    bump_forge_java(version_id, inferred)
}

/// Sube a Java 17 solo en Forge >= 36.2.26: las anteriores traen ModLauncher
/// 8.0.9 con el constructor de `ManifestEntryVerifier` roto.
pub fn bump_forge_java(version_id: &str, java: u8) -> u8 {
    let loader = Loader::from_version_id(version_id);
    if !matches!(loader, Loader::Forge(_) | Loader::NeoForge(_)) || java >= 16 {
        return java;
    }
    let should_bump = match &loader {
        Loader::Forge(id) => is_forge_version_safe(id),
        Loader::NeoForge(_) => true,
        _ => false,
    };
    if should_bump {
        info!("Forge/NeoForge detectado (Java {java} solicitado), subiendo a Java 17");
        17
    } else {
        debug!("Forge antiguo detectado, se mantiene Java {java}");
        java
    }
}

/// True si Forge es >= 36.2.26 (ModLauncher 8.1.3+, `ManifestEntryVerifier`
/// arreglado).
pub fn is_forge_version_safe(forge_ver: &str) -> bool {
    let parts: Vec<u32> = forge_ver
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    let safe = match parts.as_slice() {
        [major, minor, patch, ..] => (*major, *minor, *patch) >= (36, 2, 26),
        _ => false,
    };
    debug!("Forge version safety check: forge_ver={forge_ver}, parts={parts:?}, safe={safe}");
    safe
}

pub fn resolve_java_path(
    settings: &SettingsSnapshot,
    java_version: Option<&JavaVersion>,
) -> (u8, PathBuf) {
    let version = match java_version {
        Some(v) => v.major_version,
        None => 25,
    };
    match version {
        8 if settings.jre8_managed && JavaManager::is_installed(8) => {
            (8, JavaManager::get_java_binary(8))
        }
        17 if settings.jre17_managed && JavaManager::is_installed(17) => {
            (17, JavaManager::get_java_binary(17))
        }
        21 if settings.jre21_managed && JavaManager::is_installed(21) => {
            (21, JavaManager::get_java_binary(21))
        }
        25 if settings.jre25_managed && JavaManager::is_installed(25) => {
            (25, JavaManager::get_java_binary(25))
        }
        8 => (8, settings.get_jre8_path().to_path_buf()),
        17 => (17, settings.get_jre17_path().to_path_buf()),
        21 => (21, settings.get_jre21_path().to_path_buf()),
        25 => (25, settings.get_jre25_path().to_path_buf()),
        _ => {
            if settings.jre21_managed && JavaManager::is_installed(21) {
                (21, JavaManager::get_java_binary(21))
            } else {
                (21, settings.get_jre21_path().to_path_buf())
            }
        }
    }
}

/// A diferencia del lanzamiento, un manifiesto ilegible no es un error: se
/// infiere el Java a partir del identificador de la versión.
pub fn resolve(
    settings: &SettingsSnapshot,
    shared: &Path,
    version_id: &str,
    override_major: Option<u8>,
) -> JavaResolution {
    let (requested, inferred) = match VersionManifest::from_file(manifest_path(shared, version_id))
    {
        Ok(manifest) => match declared_java(shared, &manifest) {
            Some(java) => (java, false),
            None => {
                let inferred = aqua::infer_java_version(
                    &GameVersion::from_version_id(&manifest.id_raw).mc_version,
                );
                info!(
                    "No explicit java_version for {}; inferred Java {}",
                    manifest.id_raw, inferred
                );
                (inferred, true)
            }
        },
        Err(_) => (infer_java_for(version_id), true),
    };

    let requested = if inferred {
        infer_java_for(version_id)
    } else {
        bump_forge_java(version_id, requested)
    };

    let (major, path) = resolve_java_path(
        settings,
        Some(&JavaVersion {
            component: String::new(),
            major_version: override_major.unwrap_or(requested),
        }),
    );

    JavaResolution {
        requested,
        major,
        path,
    }
}
