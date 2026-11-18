//! Resolución de la versión de Java que necesita una instancia.

use super::SettingsSnapshot;
use super::java_manager::{JavaManager, parse_java_version};
use launchwerk::models::{JavaVersion, VersionManifest};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex, PoisonError};
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

/// Primera versión mayor que el launcher gestiona en `runtimes/jre{N}`.
const FIRST_MANAGED_JAVA: u8 = 8;

/// Serie mayor y build de una versión de `java -version`, comparables dentro
/// de la misma serie: `1.8.0_412` y `8u412` → (8, 412); `21.0.14+12` → (21, 14).
pub fn java_major_and_update(java_version: &str) -> Option<(u8, u32)> {
    let digits = java_version
        .trim()
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '_' || c == 'u'))
        .find(|segment| !segment.is_empty())?;
    let (major, rest) = digits.split_once(['.', '_', 'u'])?;
    let major: u32 = major.parse().ok()?;

    if major == 1 {
        // `1.8.0_412`: el build va tras `_`; `1.8.0` sin build es 0.
        let update = digits.split('_').nth(1).unwrap_or("0");
        return update.parse().ok().map(|update| (8, update));
    }

    // `17.0.21` / `21.0.14+12`: la actualización es la última cifra punteada.
    let update = rest
        .split(['.', 'u'])
        .next_back()
        .unwrap_or("0")
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap_or("0");
    ((FIRST_MANAGED_JAVA as u32..=99).contains(&major))
        .then_some((major as u8, update.parse().ok()?))
}

/// Indica si `java -version` corresponde a una versión anterior al arreglo de
/// CVE-2026-114938, cuyo advisory lista 8u512, 11.0.33, 17.0.21, 21.0.13,
/// 25.0.5 y 27.0.1 como últimos builds afectados de cada serie.
pub fn is_java_known_vulnerable(java_version: &str) -> bool {
    let Some((major, update)) = java_major_and_update(java_version) else {
        return false;
    };
    match major {
        8 => update <= 512,
        11 => update <= 33,
        17 => update <= 21,
        21 => update <= 13,
        25 => update <= 5,
        27 => update <= 1,
        // 22-24 quedaron fuera de soporte antes del arreglo; el resto no se
        // usa en el launcher.
        22..=24 => true,
        _ => false,
    }
}

/// True si el runtime administrado para la serie `major` está seguro.
fn managed_runtime_is_secure(major: u8) -> bool {
    match managed_java_version(major) {
        Some(version) => !is_java_known_vulnerable(&version),
        // Sin `java -version` legible no se puede afirmar que esté al día.
        None => false,
    }
}

/// Caché por serie mayor de la versión que imprime el `java -version` del
/// runtime administrado. Se vacía tras instalar o desinstalar.
static MANAGED_JAVA_VERSIONS: LazyLock<Mutex<HashMap<u8, Option<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Olvida la versión detectada de un runtime administrado (instalación o
/// desinstalación).
pub fn invalidate_managed_java(version: u8) {
    MANAGED_JAVA_VERSIONS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&version);
}

fn managed_java_version(major: u8) -> Option<String> {
    if let Some(cached) = MANAGED_JAVA_VERSIONS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&major)
    {
        return cached.clone();
    }

    let binary = JavaManager::get_java_binary(major);
    let output = binary.exists().then(|| {
        Command::new(&binary)
            .arg("-version")
            .stdin(Stdio::null())
            .output()
            .ok()
    });
    let detected = output.flatten().and_then(|output| {
        let raw = if output.stderr.is_empty() {
            &output.stdout
        } else {
            &output.stderr
        };
        parse_java_version(&String::from_utf8_lossy(raw))
    });

    MANAGED_JAVA_VERSIONS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(major, detected.clone());
    detected
}

pub fn resolve_java_path(
    settings: &SettingsSnapshot,
    java_version: Option<&JavaVersion>,
) -> (u8, PathBuf) {
    let version = match java_version {
        Some(v) => v.major_version,
        None => 25,
    };
    // Un runtime administrado de una serie anterior al arreglo de
    // CVE-2026-114938 no se usa: se trata como si no estuviera instalado para
    // que el lanzamiento pida reinstalarlo desde la cadena de proveedores.
    let managed = |major: u8, enabled: bool| {
        enabled && JavaManager::is_installed(major) && managed_runtime_is_secure(major)
    };
    match version {
        8 if managed(8, settings.jre8_managed) => (8, JavaManager::get_java_binary(8)),
        17 if managed(17, settings.jre17_managed) => (17, JavaManager::get_java_binary(17)),
        21 if managed(21, settings.jre21_managed) => (21, JavaManager::get_java_binary(21)),
        25 if managed(25, settings.jre25_managed) => (25, JavaManager::get_java_binary(25)),
        8 => (8, settings.get_jre8_path().to_path_buf()),
        17 => (17, settings.get_jre17_path().to_path_buf()),
        21 => (21, settings.get_jre21_path().to_path_buf()),
        25 => (25, settings.get_jre25_path().to_path_buf()),
        _ => {
            if managed(21, settings.jre21_managed) {
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
