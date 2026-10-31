//! Completion is published only after the full batch (including processors and
//! native extraction) succeeds. Metadata alone is never an installation receipt.
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

const PENDING: &str = ".cubic-installing";
const COMPLETE: &str = ".cubic-complete";

#[derive(PartialEq, Eq)]
struct VersionStamp {
    name: String,
    directory: Option<SystemTime>,
    manifest: Option<SystemTime>,
    receipt: Option<SystemTime>,
}

#[derive(PartialEq, Eq)]
struct InstalledVersionsCache {
    shared: PathBuf,
    libraries: Option<SystemTime>,
    assets: Option<SystemTime>,
    versions: Vec<VersionStamp>,
    installed: Vec<String>,
}

static INSTALLED_VERSIONS_CACHE: LazyLock<Mutex<Option<InstalledVersionsCache>>> =
    LazyLock::new(|| Mutex::new(None));
static INSTALLED_VERSIONS_GENERATION: AtomicU64 = AtomicU64::new(0);

fn version_dir(shared: &Path, version: &str) -> PathBuf {
    shared.join("versions").join(version)
}

pub(crate) async fn begin(shared: &Path, version: &str) -> io::Result<()> {
    zellkern::path_security::validate_version(version)?;
    invalidate_installed_versions_cache();
    let shared = shared.to_path_buf();
    let relative = Path::new("versions").join(version).join(PENDING);
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&shared)?;
        zellkern::path_security::ConfinedDir::open(&shared)?.write(&relative, b"1")
    })
    .await
    .map_err(io::Error::other)??;
    invalidate_installed_versions_cache();
    Ok(())
}

pub(crate) async fn finish(shared: &Path, version: &str) -> io::Result<()> {
    zellkern::path_security::validate_version(version)?;
    invalidate_installed_versions_cache();
    let shared = shared.to_path_buf();
    let version = version.to_string();
    tokio::task::spawn_blocking(move || {
        let root = zellkern::path_security::ConfinedDir::open(&shared)?;
        let dir = Path::new("versions").join(&version);
        let manifest = root.read(&dir.join(format!("{version}.json")))?;
        zellkern::VersionManifest::from_bytes(&manifest).map_err(io::Error::other)?;
        root.write(&dir.join(COMPLETE), &manifest)?;
        root.remove_file(&dir.join(PENDING))
    })
    .await
    .map_err(io::Error::other)??;
    invalidate_installed_versions_cache();
    Ok(())
}

fn file_present(path: &Path, size: Option<u64>) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|meta| meta.is_file() && size.map_or(meta.len() > 0, |size| meta.len() == size))
}

pub(crate) fn is_complete(shared: &Path, version: &str) -> bool {
    if zellkern::path_security::validate_version(version).is_err() {
        return false;
    }
    let dir = version_dir(shared, version);
    if dir.join(PENDING).exists() {
        return false;
    }
    let Ok(bytes) = std::fs::read(dir.join(format!("{version}.json"))) else {
        return false;
    };
    let Ok(manifest) = zellkern::VersionManifest::from_bytes(&bytes) else {
        return false;
    };
    if manifest.inherits_from.is_none()
        && !file_present(
            &dir.join(format!("{version}.jar")),
            manifest
                .downloads
                .as_ref()
                .map(|downloads| downloads.client.size),
        )
    {
        return false;
    }
    if std::fs::read(dir.join(COMPLETE)).is_ok_and(|receipt| receipt == bytes) {
        return true;
    }
    // Keep existing offline installations usable. Older releases have no
    // receipt, so inspect their runtime files instead of trusting just the JSON.
    for library in manifest.libraries.iter().flatten() {
        if !library.should_include() || !library.is_correct_arch() || library.is_native() {
            continue;
        }
        let size = library
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.size);
        if !file_present(&shared.join("libraries").join(library.get_path()), size) {
            return false;
        }
    }
    if let Some(index) = manifest.asset_index {
        let path = shared
            .join("assets/indexes")
            .join(format!("{}.json", index.id));
        let Ok(bytes) = std::fs::read(path) else {
            return false;
        };
        let Ok(index) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return false;
        };
        let Some(objects) = index["objects"].as_object() else {
            return false;
        };
        for object in objects.values() {
            let Some(hash) = object["hash"]
                .as_str()
                .filter(|hash| hash.len() == 40 && hash.bytes().all(|c| c.is_ascii_hexdigit()))
            else {
                return false;
            };
            if !file_present(
                &shared.join("assets/objects").join(&hash[..2]).join(hash),
                object["size"].as_u64(),
            ) {
                return false;
            }
        }
    }
    true
}

pub(crate) fn missing_dependencies(shared: &Path, version: &str) -> Vec<String> {
    zellkern::resolve_dependencies(version)
        .into_iter()
        .filter(|dependency| !is_complete(shared, dependency))
        .collect()
}

/// Lists complete versions. The scan is cached because it walks manifests,
/// libraries and asset indexes, and the main window is recreated after every
/// game session. Installs and repairs invalidate the cache explicitly.
pub(crate) fn installed_versions(shared: &Path) -> Vec<String> {
    let versions_dir = shared.join("versions");
    let mut stamps = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            stamps.push(VersionStamp {
                directory: modified(&path),
                manifest: modified(&path.join(format!("{name}.json"))),
                receipt: modified(&path.join(COMPLETE)),
                name,
            });
        }
    }
    stamps.sort_by(|a, b| a.name.cmp(&b.name));

    let libraries = modified(&shared.join("libraries"));
    let assets = modified(&shared.join("assets"));
    {
        let cache = INSTALLED_VERSIONS_CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = cache.as_ref()
            && cached.shared == shared
            && cached.libraries == libraries
            && cached.assets == assets
            && cached.versions == stamps
        {
            return cached.installed.clone();
        }
    }

    let generation = INSTALLED_VERSIONS_GENERATION.load(Ordering::Acquire);
    let mut installed: Vec<String> = stamps
        .iter()
        .map(|stamp| stamp.name.clone())
        .filter(|version| missing_dependencies(shared, version).is_empty())
        .collect();
    installed.sort_by(|a, b| b.cmp(a));

    // Do not publish a snapshot that raced with an install or repair.
    if INSTALLED_VERSIONS_GENERATION.load(Ordering::Acquire) == generation {
        *INSTALLED_VERSIONS_CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(InstalledVersionsCache {
            shared: shared.to_path_buf(),
            libraries,
            assets,
            versions: stamps,
            installed: installed.clone(),
        });
    }
    installed
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

pub(crate) fn invalidate_installed_versions_cache() {
    INSTALLED_VERSIONS_GENERATION.fetch_add(1, Ordering::AcqRel);
    *INSTALLED_VERSIONS_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
}

#[cfg(test)]
#[path = "../tests/services/version_installation.rs"]
mod tests;
