use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

fn prepare(path: &Path, contents: &[u8]) -> io::Result<NamedTempFile> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing parent directory"))?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(contents)?;
    temporary.as_file().sync_all()?;
    Ok(temporary)
}

fn publish(temporary: NamedTempFile, path: &Path) -> io::Result<()> {
    temporary.persist(path).map_err(|err| err.error)?;
    // Some platforms do not support opening/syncing directories.
    if let Some(parent) = path.parent()
        && let Ok(directory) = File::open(parent)
    {
        let _ = directory.sync_all();
    }
    Ok(())
}

pub(crate) fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    publish(prepare(path, contents)?, path)
}

pub(crate) async fn write_async(path: PathBuf, contents: Vec<u8>) -> io::Result<()> {
    let destination = path.clone();
    let temporary = tokio::task::spawn_blocking(move || prepare(&destination, &contents))
        .await
        .map_err(io::Error::other)??;
    // Publish while the caller still holds its data/filesystem locks. A cancelled
    // preparation only drops its temporary; it cannot publish an obsolete snapshot.
    publish(temporary, &path)
}

#[cfg(test)]
#[path = "../tests/core/atomic_file.rs"]
mod tests;
