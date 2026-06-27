use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use log::{debug, info, warn};

use crate::{Error, MCVersion, VersionManifest};

/// Determine the native subdirectory based on Minecraft version.
/// Versions >= 26.2 use "java" subdirectory.
pub fn natives_subdir(version: &MCVersion) -> &'static str {
    if version.major > 26 || (version.major == 26 && version.minor >= 2) {
        "java"
    } else {
        ""
    }
}

pub fn extract_natives(
    manifest: &VersionManifest,
    lib_dir: &Path,
    natives_dir: &Path,
) -> Result<(), Error> {
    fs::create_dir_all(natives_dir)?;

    let Some(libraries) = &manifest.libraries else {
        return Ok(());
    };

    for lib in libraries {
        if !lib.should_include() {
            continue;
        }

        let native_artifact = match lib.native_artifact() {
            Some(art) => art,
            None => {
                if lib.is_native() {
                    let jar_path = lib_dir.join(lib.get_path());
                    if jar_path.exists() {
                        extract_jar(&jar_path, natives_dir)?;
                    } else {
                        warn!("Legacy native JAR not found: {}", jar_path.display());
                    }
                }
                continue;
            }
        };

        let jar_path = lib_dir.join(&native_artifact.path);
        if !jar_path.exists() {
            warn!("Native JAR not found, skipping: {}", jar_path.display());
            continue;
        }

        extract_jar(&jar_path, natives_dir)?;
    }
    Ok(())
}

pub fn extract_jar(jar_path: &Path, dest_dir: &Path) -> Result<(), Error> {
    let file = fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to open JAR {}: {e}", jar_path.display()),
        )
    })?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let name = entry.name().to_string();

        if !is_native_file(&name) {
            continue;
        }

        let file_name = Path::new(&name)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if file_name.is_empty() {
            continue;
        }

        let out_path = dest_dir.join(&file_name);

        if out_path.exists() && out_path.metadata().map(|m| m.len()).unwrap_or(0) == entry.size() {
            debug!("Native already extracted: {}", out_path.display());
            continue;
        }

        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        fs::write(&out_path, &buf)?;
        info!("Extracted native: {} -> {}", file_name, dest_dir.display());
    }

    Ok(())
}

pub fn is_native_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.starts_with("meta-inf") || lower.ends_with('/') {
        return false;
    }
    lower.ends_with(".so")
        || lower.ends_with(".dll")
        || lower.ends_with(".dylib")
        || lower.ends_with(".jnilib")
        || lower.contains(".so.")
}

pub fn list_native_jars(manifest: &VersionManifest, lib_dir: &Path) -> Vec<PathBuf> {
    let Some(libraries) = &manifest.libraries else {
        return vec![];
    };
    libraries
        .iter()
        .filter(|l| l.should_include() && l.is_native())
        .map(|l| lib_dir.join(l.get_path()))
        .collect()
}
