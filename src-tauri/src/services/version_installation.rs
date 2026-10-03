//! Completion is published only after the full batch (including processors and
//! native extraction) succeeds. Metadata alone is never an installation receipt.
use std::io;
use std::path::{Path, PathBuf};

const PENDING: &str = ".cubic-installing";
const COMPLETE: &str = ".cubic-complete";

fn version_dir(shared: &Path, version: &str) -> PathBuf {
    shared.join("versions").join(version)
}

pub(crate) async fn begin(shared: &Path, version: &str) -> io::Result<()> {
    zellkern::path_security::validate_version(version)?;
    let shared = shared.to_path_buf();
    let relative = Path::new("versions").join(version).join(PENDING);
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&shared)?;
        zellkern::path_security::ConfinedDir::open(&shared)?.write(&relative, b"1")
    })
    .await
    .map_err(io::Error::other)?
}

pub(crate) async fn finish(shared: &Path, version: &str) -> io::Result<()> {
    zellkern::path_security::validate_version(version)?;
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
    .map_err(io::Error::other)?
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

#[cfg(test)]
#[path = "../tests/services/version_installation.rs"]
mod tests;
