//! Portable untrusted paths and handle-relative filesystem publication.
use cap_primitives::fs::FollowSymlinks;
use cap_std::fs::{Dir, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// Symlinks are not the only reparse points Windows can put in a path.
/// Junctions, mount points and cloud placeholders are reparse points too, so
/// checking the file attribute is stricter than `is_symlink`.
fn is_link_or_reparse(meta: &cap_std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use cap_std::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        meta.is_symlink() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        meta.is_symlink()
    }
}

/// Ask cap-std to resolve the final component without following links.
/// This is the hook the upstream `cap-fs-ext` crate uses; it maps to
/// `O_NOFOLLOW` on Unix and `OBJ_DONT_REPARSE` on Windows.
fn nofollow(options: &mut OpenOptions) {
    options._cap_fs_ext_follow(FollowSymlinks::No);
}

/// Open a child directory relative to an already-open directory, refusing to
/// traverse a symlink or reparse point. The returned handle remains valid even
/// if the path is later replaced.
fn open_dir_nofollow(dir: &Dir, name: &str) -> io::Result<Dir> {
    let start = dir.try_clone()?.into_std_file();
    let file = cap_primitives::fs::open_dir_nofollow(&start, Path::new(name))?;
    Ok(Dir::from_std_file(file))
}

fn create_dir_private(dir: &Dir, name: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        use cap_std::fs::DirBuilderExt;
        let mut builder = cap_std::fs::DirBuilder::new();
        builder.mode(0o700);
        dir.create_dir_with(name, &builder)
    }
    #[cfg(not(unix))]
    {
        dir.create_dir(name)
    }
}

/// Split a trusted root into its (possibly symlinked) parent and a validated
/// final component. The final component must be a single portable name.
fn root_parent(root: &Path) -> io::Result<(&Path, String)> {
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid("Storage root has no file name"))?;
    validate_component(name)?;
    let parent = match root.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    Ok((parent, name.to_owned()))
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

    /// Open an existing root without following a link in its final component.
    ///
    /// Configured ancestors are trusted and may be symlinks, but the root
    /// itself must be a real directory: otherwise a link planted at the
    /// storage path would redirect every later operation outside the boundary.
    pub fn open_existing(root: &Path) -> io::Result<Self> {
        let (parent, name) = root_parent(root)?;
        let parent_dir = Dir::open_ambient_dir(parent, cap_std::ambient_authority())?;
        let meta = parent_dir.symlink_metadata(&name)?;
        if is_link_or_reparse(&meta) {
            return Err(invalid("Storage root is a symbolic link or reparse point"));
        }
        if !meta.is_dir() {
            return Err(invalid("Storage root is not a directory"));
        }
        let dir = open_dir_nofollow(&parent_dir, &name)?;
        Self::harden_root(&dir);
        Ok(Self(dir))
    }

    /// Open an existing root, or create it with owner-only permissions.
    ///
    /// Creation is handle-relative: the final component is never created or
    /// opened through a link, even if another process races the check.
    pub fn open_or_create(root: &Path) -> io::Result<Self> {
        let (parent, name) = root_parent(root)?;
        std::fs::create_dir_all(parent)?;
        let parent_dir = Dir::open_ambient_dir(parent, cap_std::ambient_authority())?;

        match parent_dir.symlink_metadata(&name) {
            Ok(meta) if is_link_or_reparse(&meta) => {
                return Err(invalid("Storage root is a symbolic link or reparse point"));
            }
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Err(invalid("Storage root exists and is not a directory")),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                match create_dir_private(&parent_dir, &name) {
                    Ok(()) => {}
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(e),
                }
            }
            Err(e) => return Err(e),
        }

        let dir = open_dir_nofollow(&parent_dir, &name)?;
        Self::harden_root(&dir);
        Ok(Self(dir))
    }

    fn harden_root(dir: &Dir) {
        #[cfg(unix)]
        {
            use cap_std::fs::PermissionsExt;
            let _ = dir.set_permissions(".", cap_std::fs::Permissions::from_mode(0o700));
        }
        #[cfg(not(unix))]
        {
            let _ = dir;
        }
    }

    /// Apply owner-only permissions to this directory on Unix. Other targets
    /// keep their inherited ACLs.
    pub fn restrict(&self) {
        Self::harden_root(&self.0);
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
                match create_dir_private(&dir, name) {
                    Ok(()) => {}
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(e),
                }
            }
            let meta = dir.symlink_metadata(name)?;
            if !meta.is_dir() || is_link_or_reparse(&meta) {
                return Err(invalid("Destination ancestor is not a regular directory"));
            }
            dir = open_dir_nofollow(&dir, name)?;
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
        let mut options = OpenOptions::new();
        options.read(true);
        nofollow(&mut options);
        let mut file = dir.open_with(&name, &options)?;
        let meta = file.metadata()?;
        if !meta.is_file() || is_link_or_reparse(&meta) {
            return Err(invalid("Source is not a regular file"));
        }
        #[cfg(unix)]
        {
            use cap_std::fs::PermissionsExt;
            let _ = file.set_permissions(cap_std::fs::Permissions::from_mode(0o600));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub fn file_exists(&self, relative: &Path) -> io::Result<bool> {
        let (dir, name) = match self.parent(relative, false) {
            Ok(parent) => parent,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        match dir.symlink_metadata(&name) {
            Ok(meta) if meta.is_file() && !is_link_or_reparse(&meta) => Ok(true),
            Ok(_) => Err(invalid("Destination is not a regular file")),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Write to a new inode and publish by handle-relative rename. This avoids
    /// truncating symlink/hardlink targets, including an existing final file.
    pub fn copy_from(&self, relative: &Path, source: &mut impl Read) -> io::Result<()> {
        self.copy_from_inner(relative, source, false)
    }

    /// Like [`Self::copy_from`], but the published file is owner-only on Unix.
    /// The temporary file is created in the confined directory too, so cleanup
    /// can only ever unlink an entry below the opened root.
    pub fn copy_from_private(&self, relative: &Path, source: &mut impl Read) -> io::Result<()> {
        self.copy_from_inner(relative, source, true)
    }

    fn copy_from_inner(
        &self,
        relative: &Path,
        source: &mut impl Read,
        private: bool,
    ) -> io::Result<()> {
        #[cfg(not(unix))]
        let _ = private;
        let (dir, name) = self.parent(relative, true)?;
        match dir.symlink_metadata(&name) {
            Ok(meta) if !meta.is_file() || is_link_or_reparse(&meta) => {
                return Err(invalid("Destination is not a regular file"));
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let temporary = format!(".cubic-write-{}", uuid::Uuid::new_v4());
        let result = (|| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            nofollow(&mut options);
            #[cfg(unix)]
            {
                if private {
                    use cap_std::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
            }
            let mut file = dir.open_with(&temporary, &options)?;
            io::copy(source, &mut file)?;
            file.flush()?;
            file.sync_all()?;
            #[cfg(unix)]
            {
                if private {
                    use cap_std::fs::PermissionsExt;
                    file.set_permissions(cap_std::fs::Permissions::from_mode(0o600))?;
                }
            }
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

    pub fn write_private(&self, relative: &Path, contents: &[u8]) -> io::Result<()> {
        self.copy_from_private(relative, &mut &contents[..])
    }

    /// Create a new file without replacing an existing entry. The kernel
    /// refuses to follow a link when `create_new` is set, so this is safe even
    /// if a link appears between the metadata check and the open.
    pub fn create_new_private(&self, relative: &Path, contents: &[u8]) -> io::Result<()> {
        let (dir, name) = self.parent(relative, true)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        nofollow(&mut options);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = dir.open_with(&name, &options)?;
        file.write_all(contents)?;
        file.flush()?;
        file.sync_all()?;
        #[cfg(unix)]
        {
            use cap_std::fs::PermissionsExt;
            file.set_permissions(cap_std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    pub fn remove_file(&self, relative: &Path) -> io::Result<()> {
        let (dir, name) = self.parent(relative, false)?;
        match dir.symlink_metadata(&name) {
            Ok(meta) if is_link_or_reparse(&meta) => {
                return Err(invalid(
                    "Refusing to remove a symbolic link or reparse point",
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        dir.remove_file(&name)
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
        let name = entry.name()?;
        paths.push(archive_path(&name)?);
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
