//! Lightweight mod catalogs. ZIP parsing and hashing happen off the async
//! runtime, once per instance, with a global bound on disk workers.
use super::launch::validate_uuid;
use super::mods::{ModDto, PerFileCacheEntry, repo_path, resolve_modrinth_hashes};
use crate::core::errors::InstanceError;
use crate::core::{event_bus, validate_filename};
use crate::services::{
    AddonManager, InstanceManager, ModSource, compute_file_sha1, file_fingerprint,
};
use futures::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;
use tokio::sync::Semaphore;
use tracing::warn;

static DISK_WORKERS: Semaphore = Semaphore::const_new(4);
static ICON_WORKERS: Semaphore = Semaphore::const_new(2);
static ENRICHING: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

struct EnrichmentGuard(PathBuf);
impl EnrichmentGuard {
    fn acquire(path: &Path) -> Option<Self> {
        ENRICHING
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(path.to_path_buf())
            .then(|| Self(path.to_path_buf()))
    }
}
impl Drop for EnrichmentGuard {
    fn drop(&mut self) {
        ENRICHING
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.0);
    }
}

#[derive(Clone)]
struct ModFile {
    path: PathBuf,
    filename: String,
    enabled: bool,
    size: u64,
    modified: SystemTime,
    fingerprint: u64,
}

fn mod_file(path: PathBuf) -> Option<ModFile> {
    let filename = path.file_name()?.to_str()?.to_string();
    let lower = filename.to_lowercase();
    let enabled = !lower.ends_with(".disabled");
    let base = lower.strip_suffix(".disabled").unwrap_or(&lower);
    if !base.ends_with(".jar") && !base.ends_with(".zip") {
        return None;
    }
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let modified = metadata.modified().ok()?;
    let size = metadata.len();
    let fingerprint = file_fingerprint(&filename, &modified, size);
    Some(ModFile {
        path,
        filename,
        enabled,
        size,
        modified,
        fingerprint,
    })
}

fn cached_entry(
    repo: &ablage::Repo,
    filename: &str,
    fingerprint: u64,
) -> Option<PerFileCacheEntry> {
    let entry = repo.get(filename)?;
    if entry.fingerprint != fingerprint {
        return None;
    }
    postcard::from_bytes(&entry.data).ok()
}

struct Work {
    file: ModFile,
    reusable: Option<PerFileCacheEntry>,
}
struct Snapshot {
    items: Vec<ModDto>,
    missing: Vec<Work>,
}

fn snapshot(dir: &Path, include_icons: bool) -> Snapshot {
    let mut result = Snapshot {
        items: Vec::new(),
        missing: Vec::new(),
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return result;
    };
    let repo = ablage::Repo::open(repo_path(dir));
    for file in entries.flatten().filter_map(|entry| mod_file(entry.path())) {
        let exact = cached_entry(&repo, &file.filename, file.fingerprint);
        let renamed = if exact.is_none() {
            let other = if file.enabled {
                format!("{}.disabled", file.filename)
            } else {
                file.filename[..file.filename.len() - ".disabled".len()].to_string()
            };
            cached_entry(
                &repo,
                &other,
                file_fingerprint(&other, &file.modified, file.size),
            )
        } else {
            None
        };
        let cached = exact.as_ref().or(renamed.as_ref());
        let meta = cached.and_then(|entry| entry.metadata.as_ref());
        let source = cached
            .map(|entry| &entry.source)
            .unwrap_or(&ModSource::Local);
        result.items.push(ModDto {
            icon_revision: Some(file.fingerprint.to_string()),
            name: meta
                .map(|meta| meta.name.clone())
                .unwrap_or_else(|| file.filename.trim_end_matches(".disabled").to_string()),
            filename: file.filename.clone(),
            version: meta.and_then(|meta| meta.version.clone()),
            description: meta.and_then(|meta| meta.description.clone()),
            authors: meta.and_then(|meta| meta.authors.clone()),
            icon: if include_icons {
                AddonManager::get_mod_icon(&file.path).map(|icon| (*icon).clone())
            } else {
                None
            },
            enabled: file.enabled,
            sha1: cached.map(|entry| entry.sha1.clone()).unwrap_or_default(),
            file_size: file.size,
            source: source.source_str().to_string(),
            project_id: source.project_id().map(str::to_string),
            slug: source.slug().map(str::to_string),
        });
        if exact.is_none() {
            result.missing.push(Work {
                file,
                reusable: renamed,
            });
        }
    }
    result
}

fn put(repo: &mut ablage::Repo, file: &ModFile, value: &PerFileCacheEntry) {
    if let Ok(data) = postcard::to_stdvec(value) {
        repo.put(
            file.filename.clone(),
            ablage::Entry {
                version: 1,
                fingerprint: file.fingerprint,
                data,
            },
        );
    }
}

fn unchanged(file: &ModFile) -> bool {
    mod_file(file.path.clone()).is_some_and(|current| current.fingerprint == file.fingerprint)
}

async fn enrich_local(dir: PathBuf, missing: Vec<Work>) -> Vec<(ModFile, PerFileCacheEntry)> {
    let parsed: Vec<_> = stream::iter(missing)
        .map(|work| async move {
            let permit = DISK_WORKERS.acquire().await.ok()?;
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let entry = work.reusable.unwrap_or_else(|| PerFileCacheEntry {
                    sha1: compute_file_sha1(&work.file.path).unwrap_or_default(),
                    metadata: AddonManager::get_mod_metadata_only(&work.file.path),
                    source: ModSource::Local,
                });
                (work.file, entry)
            })
            .await
            .ok()
        })
        .buffer_unordered(4)
        .filter_map(|value| async { value })
        .collect()
        .await;
    tokio::task::spawn_blocking(move || {
        let mut repo = ablage::Repo::open(repo_path(&dir));
        let mut saved = Vec::new();
        for (file, mut value) in parsed {
            if !unchanged(&file) {
                continue;
            }
            // Market downloads may have saved their source while parsing ran.
            if let Some(previous) = repo
                .get(&file.filename)
                .and_then(|entry| postcard::from_bytes::<PerFileCacheEntry>(&entry.data).ok())
                && (previous.sha1.is_empty() || previous.sha1 == value.sha1)
                && !matches!(previous.source, ModSource::Local)
            {
                value.source = previous.source;
            }
            put(&mut repo, &file, &value);
            saved.push((file, value));
        }
        if let Err(error) = repo.flush() {
            warn!("Mod metadata cache: {error}");
        }
        saved
    })
    .await
    .unwrap_or_default()
}

fn notify(id: &str) {
    event_bus::emit(event_bus::AppEvent::ModsEnriched {
        id: id.to_string().into(),
    });
}

pub(super) async fn list(id: String, dir: PathBuf, include_icons: bool) -> Vec<ModDto> {
    let scan_dir = dir.clone();
    let scan = tokio::task::spawn_blocking(move || snapshot(&scan_dir, include_icons)).await;
    let Ok(Snapshot { items, missing }) = scan else {
        return Vec::new();
    };
    if missing.is_empty() {
        return items;
    }
    if let Some(guard) = EnrichmentGuard::acquire(&dir) {
        tokio::spawn(async move {
            let saved = enrich_local(dir.clone(), missing).await;
            // Names and versions are ready before any network request finishes.
            notify(&id);
            let pending: Vec<_> = saved
                .iter()
                .filter(|(_, value)| {
                    matches!(value.source, ModSource::Local) && !value.sha1.is_empty()
                })
                .map(|(_, value)| value.sha1.clone())
                .collect();
            if !pending.is_empty() {
                match resolve_modrinth_hashes(&pending).await {
                    Ok(resolved) => {
                        let _ = tokio::task::spawn_blocking(move || {
                            let mut repo = ablage::Repo::open(repo_path(&dir));
                            for (file, value) in saved {
                                let Some(remote) = resolved.get(&value.sha1) else {
                                    continue;
                                };
                                let Some(mut current) =
                                    cached_entry(&repo, &file.filename, file.fingerprint)
                                else {
                                    continue;
                                };
                                if current.sha1 != value.sha1
                                    || !unchanged(&file)
                                    || !matches!(current.source, ModSource::Local)
                                {
                                    continue;
                                }
                                current.source = ModSource::Modrinth {
                                    project_id: remote.project_id.clone(),
                                    version_id: remote.id.clone(),
                                    slug: None,
                                };
                                put(&mut repo, &file, &current);
                            }
                            if let Err(error) = repo.flush() {
                                warn!("Mod source cache: {error}");
                            }
                        })
                        .await;
                    }
                    Err(error) => warn!("Mod source lookup: {error}"),
                }
            }
            drop(guard);
            // A file changed while this scan was active: the next refresh can
            // now schedule its own enrichment rather than duplicate this one.
            notify(&id);
        });
    }
    items
}

#[derive(Deserialize, Clone)]
pub struct IconRequest {
    filename: String,
    revision: String,
}
#[derive(Serialize)]
pub struct ModIcon {
    filename: String,
    revision: String,
    icon: Option<String>,
}

fn icons(dir: &Path, files: Vec<IconRequest>) -> Vec<ModIcon> {
    files
        .into_iter()
        .filter_map(|request| {
            let file = mod_file(dir.join(&request.filename))?;
            if file.fingerprint.to_string() != request.revision {
                return None;
            }
            let icon = AddonManager::get_mod_icon(&file.path).map(|icon| (*icon).clone());
            if !unchanged(&file) {
                return None;
            }
            Some(ModIcon {
                filename: request.filename,
                revision: request.revision,
                icon,
            })
        })
        .collect()
}

#[tauri::command]
pub async fn get_instance_mod_icons(
    id: String,
    files: Vec<IconRequest>,
) -> Result<Vec<ModIcon>, String> {
    validate_uuid(&id)?;
    if files.len() > 24 {
        return Err(icon_error("Too many mod icons requested"));
    }
    for file in &files {
        validate_filename(&file.filename)?;
    }
    let handle = InstanceManager::get()
        .get_handle(&id)
        .await
        .ok_or_else(|| String::from(InstanceError::NotFound))?;
    let dir = handle.get_instance_dir().await.join("mods");
    let permit = ICON_WORKERS.acquire().await.map_err(icon_error)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        icons(&dir, files)
    })
    .await
    .map_err(icon_error)
}

fn icon_error(error: impl std::fmt::Display) -> String {
    serde_json::json!({ "code": "INST_MOD_ICONS", "params": { "error": error.to_string() } })
        .to_string()
}

#[cfg(test)]
#[path = "../../tests/commands/instance/mod_catalog.rs"]
mod tests;
