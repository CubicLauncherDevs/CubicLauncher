mod copy;
mod discovery;

use crate::core::{AppEvent, PathManager, emit};
use crate::services::instance_manager::data::{RamOverrides, validate_instance_name};
use crate::services::{DownloadQueue, InstOverrides, InstanceManager};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::ipc::Channel;
use zellkern::GameVersion;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Official,
    Multimc,
}

#[derive(Clone, Serialize)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub suggested_name: String,
    pub source: PathBuf,
    pub root: PathBuf,
    pub version: Option<GameVersion>,
    pub error: Option<String>,
    pub shared_directory: bool,
    pub reinstalls_components: bool,
    #[serde(skip)]
    icon: Option<PathBuf>,
    #[serde(skip)]
    memory: Option<(u32, u32)>,
}

#[derive(Serialize)]
pub struct Scan {
    token: String,
    candidates: Vec<Candidate>,
    issues: Vec<String>,
}

struct Session {
    candidates: Vec<Candidate>,
    created: Instant,
    cancelled: Arc<AtomicBool>,
    running: AtomicBool,
}

static SESSIONS: LazyLock<parking_lot::Mutex<HashMap<String, Arc<Session>>>> =
    LazyLock::new(Default::default);

#[derive(Deserialize)]
pub struct Selection {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Serialize)]
pub struct Progress {
    id: String,
    index: usize,
    count: usize,
    copied: u64,
    total: u64,
    file: String,
    phase: &'static str,
}

#[derive(Debug, Serialize)]
pub struct MigrationResult {
    id: String,
    name: String,
    status: &'static str,
    error: Option<String>,
}

pub async fn scan(provider: Provider, root: Option<String>) -> Result<Scan, String> {
    // Cached Mojang metadata resolves latest-release/snapshot and uninstalled vanilla profiles.
    let latest = if matches!(provider, Provider::Official) {
        tokio::time::timeout(
            Duration::from_secs(10),
            crate::commands::download::get_available_versions(),
        )
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default()
        .into_iter()
        .map(|v| (v.id, v.version_type))
        .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let (candidates, issues) = tokio::task::spawn_blocking(move || {
        let roots = root
            .map(|p| vec![PathBuf::from(p)])
            .unwrap_or_else(|| discovery::default_roots(provider));
        let mut candidates = Vec::new();
        let mut issues = Vec::new();
        let mut seen = HashSet::new();
        for root in roots {
            let canonical = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
            if !seen.insert(canonical) {
                continue;
            }
            match discovery::scan_root(provider, &root, &latest) {
                Ok(mut found) => candidates.append(&mut found),
                Err(e) => issues.push(format!("{}: {e}", root.display())),
            }
        }
        (candidates, issues)
    })
    .await
    .map_err(|e| e.to_string())?;
    let token = uuid::Uuid::new_v4().to_string();
    let mut sessions = SESSIONS.lock();
    sessions.retain(|_, s| {
        s.running.load(Ordering::Relaxed) || s.created.elapsed() < Duration::from_secs(900)
    });
    if sessions.len() >= 16 {
        return Err("Too many migration previews; close an existing preview".into());
    }
    sessions.insert(
        token.clone(),
        Arc::new(Session {
            candidates: candidates.clone(),
            created: Instant::now(),
            cancelled: Arc::new(AtomicBool::new(false)),
            running: AtomicBool::new(false),
        }),
    );
    Ok(Scan {
        token,
        candidates,
        issues,
    })
}

pub fn cancel(token: &str) {
    let mut sessions = SESSIONS.lock();
    if let Some(session) = sessions.get(token) {
        session.cancelled.store(true, Ordering::Relaxed);
        if !session.running.load(Ordering::Relaxed) {
            sessions.remove(token);
        }
    }
}

struct SessionGuard(String);
impl Drop for SessionGuard {
    fn drop(&mut self) {
        SESSIONS.lock().remove(&self.0);
    }
}

pub async fn migrate(
    token: String,
    selections: Vec<Selection>,
    channel: Channel<Progress>,
) -> Result<Vec<MigrationResult>, String> {
    let session = {
        let sessions = SESSIONS.lock();
        let session = sessions
            .get(&token)
            .ok_or("Migration preview expired; scan again")?
            .clone();
        if session.created.elapsed() >= Duration::from_secs(900)
            || session.cancelled.load(Ordering::Relaxed)
        {
            return Err("Migration preview expired; scan again".into());
        }
        let mut names = HashSet::new();
        let mut ids = HashSet::new();
        if selections.is_empty() {
            return Err("No instances selected".into());
        }
        for selection in &selections {
            validate_instance_name(&selection.name)?;
            if !names.insert(&selection.name) || !ids.insert(&selection.id) {
                return Err("Duplicate instance selection or name".into());
            }
            if !session
                .candidates
                .iter()
                .any(|c| c.id == selection.id && c.error.is_none() && c.version.is_some())
            {
                return Err("Invalid or unsupported instance selection".into());
            }
        }
        if session.running.swap(true, Ordering::Relaxed) {
            return Err("Migration already running".into());
        }
        session
    };
    let _guard = SessionGuard(token);
    let mut results = Vec::new();
    for (index, selection) in selections.iter().enumerate() {
        let candidate = session
            .candidates
            .iter()
            .find(|c| c.id == selection.id)
            .unwrap()
            .clone();
        let progress = Progress {
            id: candidate.id.clone(),
            index: index + 1,
            count: selections.len(),
            copied: 0,
            total: 0,
            file: String::new(),
            phase: "preparing",
        };
        let result = migrate_one(
            candidate,
            selection.name.clone(),
            session.cancelled.clone(),
            channel.clone(),
            progress,
        )
        .await;
        let (status, error) = match result {
            Ok(()) => ("success", None),
            Err(e) if e == "Migration cancelled" => ("cancelled", Some(e)),
            Err(e) => ("error", Some(e)),
        };
        results.push(MigrationResult {
            id: selection.id.clone(),
            name: selection.name.clone(),
            status,
            error,
        });
    }
    Ok(results)
}

async fn migrate_one(
    candidate: Candidate,
    name: String,
    cancelled: Arc<AtomicBool>,
    channel: Channel<Progress>,
    mut progress: Progress,
) -> Result<(), String> {
    copy::check_cancel(&cancelled)?;
    let _ = channel.send(progress.clone());
    let version = candidate
        .version
        .as_ref()
        .ok_or("Missing Minecraft version")?
        .to_version_id();
    for handle in InstanceManager::get().get_all_handles().await {
        if handle.get_name().await.as_ref() == name {
            return Err("An instance with that name already exists".into());
        }
    }
    let cancel_copy = cancelled.clone();
    let copy_channel = channel.clone();
    let mut copy_progress = progress.clone();
    let source = candidate.source.clone();
    let icon = candidate.icon.clone();
    let stage = tokio::task::spawn_blocking(move || {
        let stage = tempfile::Builder::new()
            .prefix(".migration-")
            .tempdir_in(PathManager::get().get_instance_dir())
            .map_err(|e| e.to_string())?;
        copy::copy_game(
            &source,
            stage.path(),
            icon.as_deref(),
            &cancel_copy,
            |copied, total, file| {
                copy_progress.phase = "copying";
                copy_progress.copied = copied;
                copy_progress.total = total;
                copy_progress.file = file.to_string();
                let _ = copy_channel.send(copy_progress.clone());
            },
        )?;
        Ok::<_, String>(stage)
    })
    .await
    .map_err(|e| e.to_string())??;
    copy::check_cancel(&cancelled)?;
    progress.phase = "installing";
    let _ = channel.send(progress);
    let manager = InstanceManager::get();
    let handle = manager
        .create_instance(name, version.clone(), None)
        .await
        .map_err(|e| e.to_string())?;
    let result = async {
        let guard = handle.try_lock_files()?;
        let target = handle.get_instance_dir().await;
        let mut entries = tokio::fs::read_dir(stage.path())
            .await
            .map_err(|e| e.to_string())?;
        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            copy::check_cancel(&cancelled)?;
            tokio::fs::rename(entry.path(), target.join(entry.file_name()))
                .await
                .map_err(|e| e.to_string())?;
        }
        if candidate.icon.is_some() {
            handle
                .set_icon(Some(target.join("icon.png").to_string_lossy().to_string()))
                .await;
        }
        if let Some((min_mem, max_mem)) = candidate.memory {
            handle
                .set_overrides(Some(InstOverrides {
                    java_version: None,
                    memory: Some(RamOverrides { min_mem, max_mem }),
                }))
                .await;
        }
        copy::check_cancel(&cancelled)?;
        guard.save_if_dirty().await.map_err(|e| e.to_string())?;
        Ok::<_, String>(())
    }
    .await;
    if let Err(error) = result {
        return match manager.delete_instance(&handle.uuid).await {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!("{error}; cleanup failed: {cleanup}")),
        };
    }
    emit(AppEvent::InstanceCreated {
        id: handle.uuid.to_string().into(),
        dto: handle.to_dto().await,
    });
    DownloadQueue::get().enqueue(version).await;
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/services/instance_import/migration_sessions.rs"]
mod tests;
