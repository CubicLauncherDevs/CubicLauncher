//! Portable untrusted paths and handle-relative filesystem publication.
use cap_std::fs::{Dir, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// A single portable filename. Reject Windows aliases even on Unix so an
/// archive cannot acquire a different meaning when imported on another OS.
pub fn validate_component(name: &str) -> io::Result<()> {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    let device = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|n| {
            matches!(
                n,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "/\\<>:\"|?*".contains(c))
        || device
    {
        return Err(invalid(format!("Unsafe path component: {name:?}")));
    }
    Ok(())
}

/// Versions are identifiers, never paths (including on a different platform).
pub fn validate_version(version: &str) -> io::Result<()> {
    validate_component(version)?;
    if version.len() > 200
        || !version
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b" ._-+".contains(&c))
    {
        return Err(invalid(format!("Invalid version identifier: {version:?}")));
    }
    Ok(())
}

/// Validate the original ZIP name, before a ZIP library can discard roots or
/// normalize away parent references. Backslashes are accepted as separators.
pub fn archive_path(name: &str) -> io::Result<PathBuf> {
    let normalized = name.replace('\\', "/");
    if normalized.starts_with('/') {
        return Err(invalid(format!("Absolute archive path: {name:?}")));
    }
    let mut path = PathBuf::new();
    for part in normalized.trim_end_matches('/').split('/') {
        validate_component(part)?;
        path.push(part);
    }
    Ok(path)
}

/// Directory capability: every lookup stays under an opened directory, rather
/// than checking a string/canonical path and later reopening it by ambient path.
pub struct ConfinedDir(Dir);

impl ConfinedDir {
    /// The caller chooses the trusted root. Its configured ancestors may be
    /// symlinks (e.g. storage on another disk); descendants may not be links.
    pub fn open(root: &Path) -> io::Result<Self> {
        let root = root.canonicalize()?;
        Ok(Self(Dir::open_ambient_dir(
            root,
            cap_std::ambient_authority(),
        )?))
    }

    fn parent(&self, relative: &Path, create: bool) -> io::Result<(Dir, String)> {
        let text = relative.to_str().ok_or_else(|| invalid("Non UTF-8 path"))?;
        let path = archive_path(text)?;
        let mut parts = path.iter().peekable();
        let mut dir = self.0.try_clone()?;
        while let Some(part) = parts.next() {
            let name = part
                .to_str()
                .ok_or_else(|| invalid("Non UTF-8 component"))?;
            if parts.peek().is_none() {
                return Ok((dir, name.to_owned()));
            }
            if create {
                match dir.create_dir(name) {
                    Ok(()) => {}
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(e),
                }
            }
            let meta = dir.symlink_metadata(name)?;
            if !meta.is_dir() || meta.is_symlink() {
                return Err(invalid("Destination ancestor is not a regular directory"));
            }
            dir = dir.open_dir(name)?;
        }
        Err(invalid("Empty destination path"))
    }

    pub fn create_dir_all(&self, relative: &Path) -> io::Result<()> {
        let (dir, _) = self.parent(&relative.join(".cubic-directory-probe"), true)?;
        drop(dir);
        Ok(())
    }

    pub fn open_dir(&self, relative: &Path) -> io::Result<Self> {
        let (dir, _) = self.parent(&relative.join(".cubic-directory-probe"), false)?;
        Ok(Self(dir))
    }

    pub fn read(&self, relative: &Path) -> io::Result<Vec<u8>> {
        let (dir, name) = self.parent(relative, false)?;
        let meta = dir.symlink_metadata(&name)?;
        if !meta.is_file() || meta.is_symlink() {
            return Err(invalid("Source is not a regular file"));
        }
        dir.read(name)
    }

    pub fn file_exists(&self, relative: &Path) -> io::Result<bool> {
        let (dir, name) = match self.parent(relative, false) {
            Ok(parent) => parent,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_file() && !meta.is_symlink() => Ok(true),
            Ok(_) => Err(invalid("Destination is not a regular file")),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Write to a new inode and publish by handle-relative rename. This avoids
    /// truncating symlink/hardlink targets, including an existing final file.
    pub fn copy_from(&self, relative: &Path, source: &mut impl Read) -> io::Result<()> {
        let (dir, name) = self.parent(relative, true)?;
        match dir.symlink_metadata(&name) {
            Ok(meta) if !meta.is_file() || meta.is_symlink() => {
                return Err(invalid("Destination is not a regular file"));
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let temporary = format!(".cubic-write-{}", uuid::Uuid::new_v4());
        let result = (|| {
            let mut file =
                dir.open_with(&temporary, OpenOptions::new().write(true).create_new(true))?;
            io::copy(source, &mut file)?;
            file.flush()?;
            file.sync_all()?;
            drop(file);
            dir.rename(&temporary, &dir, &name)?;
            // Directory syncing is not supported on every target.
            let _ = dir.try_clone()?.into_std_file().sync_all();
            Ok(())
        })();
        if result.is_err() {
            let _ = dir.remove_file(&temporary);
        }
        result
    }

    pub fn write(&self, relative: &Path, contents: &[u8]) -> io::Result<()> {
        self.copy_from(relative, &mut &contents[..])
    }

    pub fn remove_file(&self, relative: &Path) -> io::Result<()> {
        let (dir, name) = self.parent(relative, false)?;
        dir.remove_file(name)
    }

    pub fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let (source, from) = self.parent(from, false)?;
        let (destination, to) = self.parent(to, false)?;
        source.rename(from, &destination, to)
    }

    pub fn entries(&self) -> io::Result<Vec<std::ffi::OsString>> {
        self.0
            .entries()?
            .map(|entry| entry.map(|e| e.file_name()))
            .collect()
    }

    pub fn remove_dir_all(&self, relative: &Path) -> io::Result<()> {
        let (dir, name) = self.parent(relative, false)?;
        dir.remove_dir_all(name)
    }
}

/// Reject special entries instead of publishing symlinks into a tree that
/// later import/copy code might follow. Validate all names before writing.
pub fn extract_zip<R: Read + io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    root: &Path,
) -> io::Result<()> {
    let mut paths = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        if entry.is_symlink() {
            return Err(invalid("Archive symlinks are not allowed"));
        }
        paths.push(archive_path(entry.name())?);
    }
    let destination = ConfinedDir::open(root)?;
    for (i, path) in paths.iter().enumerate() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            destination.create_dir_all(path)?;
        } else {
            destination.copy_from(path, &mut entry)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/path_security.rs"]
mod tests;
