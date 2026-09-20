use super::super::launchers::multimc::migrate::FOLDERS_TO_MIGRATE;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub fn check_cancel(cancelled: &AtomicBool) -> Result<(), String> {
    if cancelled.load(Ordering::Relaxed) {
        Err("Migration cancelled".into())
    } else {
        Ok(())
    }
}

struct Entry {
    source: PathBuf,
    relative: PathBuf,
    directory: bool,
}

fn collect(
    source: &Path,
    relative: &Path,
    entries: &mut Vec<Entry>,
    total: &mut u64,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    check_cancel(cancelled)?;
    let path = source.join(relative);
    let meta = fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.file_type().is_symlink() || (!meta.is_dir() && !meta.is_file()) {
        return Err(format!(
            "Cannot migrate a symbolic link or special file: {}",
            path.display()
        ));
    }
    entries.push(Entry {
        source: path.clone(),
        relative: relative.to_path_buf(),
        directory: meta.is_dir(),
    });
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            collect(
                source,
                &relative.join(entry.file_name()),
                entries,
                total,
                cancelled,
            )?;
        }
    } else {
        *total = total.saturating_add(meta.len());
    }
    Ok(())
}

/// Only game content is copied: credentials, launcher metadata and shared runtimes stay out.
pub fn copy_game(
    source: &Path,
    target: &Path,
    icon: Option<&Path>,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(u64, u64, &str),
) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "Game directory no longer exists: {}",
            source.display()
        ));
    }
    let mut entries = Vec::new();
    let mut total = 0;
    for name in FOLDERS_TO_MIGRATE {
        match fs::symlink_metadata(source.join(name)) {
            Ok(_) => collect(source, Path::new(name), &mut entries, &mut total, cancelled)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    if let Some(icon) = icon {
        let meta = fs::symlink_metadata(icon).map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("Invalid instance icon".into());
        }
        total = total.saturating_add(meta.len());
        entries.push(Entry {
            source: icon.to_path_buf(),
            relative: "icon.png".into(),
            directory: false,
        });
    }
    let mut copied = 0u64;
    let mut buffer = vec![0u8; 256 * 1024];
    let mut last = Instant::now();
    progress(0, total, "");
    for entry in entries {
        check_cancel(cancelled)?;
        // Recheck in case files were changed while the inventory was being built.
        let meta = fs::symlink_metadata(&entry.source).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() || meta.is_dir() != entry.directory {
            return Err("Source changed during migration".into());
        }
        let destination = target.join(&entry.relative);
        if entry.directory {
            fs::create_dir_all(destination).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut input = fs::File::open(&entry.source).map_err(|e| e.to_string())?;
        let mut output = fs::File::create(destination).map_err(|e| e.to_string())?;
        loop {
            check_cancel(cancelled)?;
            let read = input.read(&mut buffer).map_err(|e| e.to_string())?;
            if read == 0 {
                break;
            }
            output
                .write_all(&buffer[..read])
                .map_err(|e| e.to_string())?;
            copied = copied.saturating_add(read as u64);
            if last.elapsed() >= Duration::from_millis(100) {
                progress(copied, total.max(copied), &entry.relative.to_string_lossy());
                last = Instant::now();
            }
        }
        output.flush().map_err(|e| e.to_string())?;
    }
    check_cancel(cancelled)?;
    progress(copied, total.max(copied), "");
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/services/instance_import/migration_copy.rs"]
mod tests;
