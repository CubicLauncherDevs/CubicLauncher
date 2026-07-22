use crate::core::errors::InstanceError;
use crate::core::event_bus;
use crate::services::{
    AddonManager, AddonMetaNoIcon, InstanceManager, ModSource, PackCacheEntry, PackFullCacheEntry,
    compute_file_sha1, file_fingerprint, read_all_full_pack_cache,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

use super::launch::validate_uuid;

#[derive(serde::Serialize)]
pub struct ModDto {
    pub name: String,
    pub filename: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
    pub icon: Option<String>,
    pub enabled: bool,
    pub sha1: String,
    pub file_size: u64,
    pub source: String,
    pub project_id: Option<String>,
    pub slug: Option<String>,
}

/// Per-file entry in ablage, keyed by filename
/// NOTA: Sin icon — se busca aparte via AddonManager::get_mod_icon()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerFileCacheEntry {
    pub sha1: String,
    pub metadata: Option<AddonMetaNoIcon>,
    pub source: ModSource,
}

#[derive(Debug, Deserialize)]
struct ModrinthVersionEntry {
    id: String,
    project_id: String,
}

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

pub(crate) fn repo_path(mods_dir: &Path) -> PathBuf {
    mods_dir.join(".mod_cache.crep")
}

fn is_hidden_or_crep(filename: &str) -> bool {
    filename.starts_with('.') || filename.ends_with(".crep")
}

/// Preserve a non-Local source that was saved during a market install
/// so the frontend can still match the file with a remote project.
fn preserve_pack_source(entry: &mut PackFullCacheEntry, repo: &ablage::Repo, filename: &str) {
    if !matches!(entry.source, ModSource::Local) {
        return;
    }
    let Some(data) = repo.get(filename).map(|e| e.data.clone()) else {
        return;
    };
    if let Ok(prev) = postcard::from_bytes::<PackFullCacheEntry>(&data) {
        entry.source = prev.source;
    } else if let Ok(prev) = postcard::from_bytes::<PackCacheEntry>(&data) {
        entry.source = prev.source;
    }
}

#[tauri::command]
pub async fn get_instance_mods(id: String) -> Vec<ModDto> {
    if let Err(e) = validate_uuid(&id) {
        warn!("{}", e);
        return Vec::new();
    }
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        warn!("Instancia {} no encontrada para listar mods", id);
        return Vec::new();
    };

    let mods_dir = handle.get_instance_dir().await.join("mods");
    info!("Listando mods de instancia {} en {:?}", id, mods_dir);

    // --- Phase 1: List files + compute fingerprints ---
    struct FileEntry {
        path: PathBuf,
        filename: String,
        display_name: String,
        enabled: bool,
        size: u64,
        fingerprint: u64,
    }

    let entries = tokio::task::spawn_blocking(move || -> Vec<FileEntry> {
        let dir = match std::fs::read_dir(&mods_dir) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        dir.flatten()
            .filter_map(|e| {
                let path = e.path();
                if !path.is_file() {
                    return None;
                }
                let file_name = path.file_name()?.to_string_lossy().to_string();
                let file_name_lower = file_name.to_lowercase();
                let ext = path.extension()?.to_string_lossy().to_lowercase();

                let (is_mod, enabled) = if ext == "jar" || ext == "zip" {
                    (true, true)
                } else if ext == "disabled"
                    && (file_name_lower.ends_with(".jar.disabled")
                        || file_name_lower.ends_with(".zip.disabled"))
                {
                    (true, false)
                } else {
                    (false, false)
                };

                if !is_mod {
                    return None;
                }

                let display_name = file_name
                    .strip_suffix(".disabled")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| file_name.clone());

                let meta = std::fs::metadata(&path).ok()?;
                let mtime = meta.modified().ok()?;
                let size = meta.len();
                let fingerprint = file_fingerprint(&file_name, &mtime, size);

                Some(FileEntry {
                    path,
                    filename: file_name,
                    display_name,
                    enabled,
                    size,
                    fingerprint,
                })
            })
            .collect()
    })
    .await
    .unwrap_or_default();

    if entries.is_empty() {
        return Vec::new();
    }

    let mods_dir2 = entries[0].path.parent().unwrap().to_path_buf();
    let repo_path = repo_path(&mods_dir2);

    // --- Phase 2: Fast-path via global fingerprint (sin cargar el HashMap completo) ---
    let global_fp: u64 = entries.iter().fold(0, |acc, e| acc ^ e.fingerprint);

    if ablage::Repo::check_global_fingerprint(&repo_path, global_fp) {
        // Cache hit: abrimos el repo solo para deserializar entries
        let repo = ablage::Repo::open(&repo_path);
        let mods: Vec<ModDto> = entries
            .into_iter()
            .map(|e| {
                let entry: Option<PerFileCacheEntry> = repo
                    .get(&e.filename)
                    .and_then(|entry| postcard::from_bytes(&entry.data).ok());
                match entry {
                    Some(cached) => {
                        let icon = AddonManager::get_mod_icon(&e.path).map(|s| (*s).clone());
                        let md = cached.metadata.as_ref();
                        ModDto {
                            name: md.map(|m| m.name.clone()).unwrap_or(e.display_name),
                            filename: e.filename,
                            version: md.and_then(|m| m.version.clone()),
                            description: md.and_then(|m| m.description.clone()),
                            authors: md.map(|m| m.authors.clone().unwrap_or_default()),
                            icon,
                            enabled: e.enabled,
                            sha1: cached.sha1,
                            file_size: e.size,
                            source: cached.source.source_str().to_string(),
                            project_id: cached.source.project_id().map(|s| s.to_string()),
                            slug: cached.source.slug().map(|s| s.to_string()),
                        }
                    }
                    None => ModDto {
                        name: e.display_name,
                        filename: e.filename,
                        version: None,
                        description: None,
                        authors: None,
                        icon: None,
                        enabled: e.enabled,
                        sha1: String::new(),
                        file_size: e.size,
                        source: "local".to_string(),
                        project_id: None,
                        slug: None,
                    },
                }
            })
            .collect();
        info!(
            "{} mods cargados desde cache en instancia {}",
            mods.len(),
            id
        );
        return mods;
    }

    // --- Cache miss: build minimal ModDtos, enrich in background ---
    let minimal: Vec<ModDto> = entries
        .iter()
        .map(|e| ModDto {
            name: e.display_name.clone(),
            filename: e.filename.clone(),
            version: None,
            description: None,
            authors: None,
            icon: None,
            enabled: e.enabled,
            sha1: String::new(),
            file_size: e.size,
            source: "local".to_string(),
            project_id: None,
            slug: None,
        })
        .collect();

    info!(
        "{} mods listados (minimal) en instancia {} — enriqueciendo en background",
        minimal.len(),
        id
    );

    let repo_path2 = repo_path.clone();
    let id2 = id.clone();
    tokio::spawn(async move {
        let mut repo = ablage::Repo::open(&repo_path2);

        struct RawResult {
            sha1: String,
            metadata: Option<AddonMetaNoIcon>,
        }

        let mut to_resolve: Vec<String> = Vec::new();
        let mut cached: Vec<(FileEntry, PerFileCacheEntry)> = Vec::with_capacity(entries.len());

        let handles: Vec<_> = entries
            .iter()
            .map(|e| {
                let path = e.path.clone();
                let filename = e.filename.clone();
                let fingerprint = e.fingerprint;

                let cached_entry = repo.get(&filename).and_then(|entry| {
                    if entry.fingerprint == fingerprint {
                        postcard::from_bytes::<PerFileCacheEntry>(&entry.data).ok()
                    } else {
                        None
                    }
                });

                if let Some(cached) = cached_entry {
                    tokio::task::spawn_blocking(move || -> RawResult {
                        RawResult {
                            sha1: cached.sha1,
                            metadata: cached.metadata,
                        }
                    })
                } else {
                    tokio::task::spawn_blocking(move || -> RawResult {
                        let sha1 = compute_file_sha1(&path).unwrap_or_default();
                        let parsed = AddonManager::get_mod_info(&path);
                        RawResult {
                            sha1,
                            metadata: parsed,
                        }
                    })
                }
            })
            .collect();

        let raw_results: Vec<RawResult> = futures::future::join_all(handles)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        for (entry, raw) in entries.into_iter().zip(raw_results) {
            let existing = repo.get(&entry.filename).and_then(|e| {
                if e.fingerprint == entry.fingerprint {
                    postcard::from_bytes::<PerFileCacheEntry>(&e.data).ok()
                } else {
                    None
                }
            });

            match existing {
                Some(per_file) => {
                    cached.push((entry, per_file));
                }
                None => {
                    let per_file = PerFileCacheEntry {
                        sha1: raw.sha1.clone(),
                        metadata: raw.metadata.clone(),
                        source: ModSource::Local,
                    };
                    if let Ok(data) = postcard::to_stdvec(&per_file) {
                        repo.put(
                            entry.filename.clone(),
                            ablage::Entry {
                                version: 1,
                                fingerprint: entry.fingerprint,
                                data,
                            },
                        );
                    }
                    if !raw.sha1.is_empty() {
                        to_resolve.push(raw.sha1.clone());
                    }
                    cached.push((entry, per_file));
                }
            }
        }

        if !to_resolve.is_empty() {
            let pending: Vec<String> = cached
                .iter()
                .filter(|(_, entry)| matches!(entry.source, ModSource::Local))
                .filter(|(_, entry)| !entry.sha1.is_empty())
                .map(|(_, entry)| entry.sha1.clone())
                .collect();

            if !pending.is_empty() {
                info!(
                    "Resolviendo {} mods via Modrinth hash lookup en instancia {}",
                    pending.len(),
                    id2
                );
                match resolve_modrinth_hashes(&pending).await {
                    Ok(api_results) => {
                        for (sha1, version) in api_results {
                            let source = ModSource::Modrinth {
                                project_id: version.project_id,
                                version_id: version.id,
                                slug: None,
                            };
                            for (file_entry, entry) in cached.iter_mut() {
                                if entry.sha1 == sha1 {
                                    entry.source = source.clone();
                                    let updated = PerFileCacheEntry {
                                        sha1: entry.sha1.clone(),
                                        metadata: entry.metadata.clone(),
                                        source: source.clone(),
                                    };
                                    if let Ok(data) = postcard::to_stdvec(&updated) {
                                        repo.put(
                                            file_entry.filename.clone(),
                                            ablage::Entry {
                                                version: 1,
                                                fingerprint: 0,
                                                data,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error resolviendo hashes via Modrinth: {}", e);
                    }
                }
            }
        }

        repo.put(
            "__global",
            ablage::Entry {
                version: 1,
                fingerprint: global_fp,
                data: global_fp.to_le_bytes().to_vec(),
            },
        );

        let _ = repo.flush();

        event_bus::emit(event_bus::AppEvent::ModsEnriched { id: id2.into() });
    });

    minimal
}

async fn resolve_modrinth_hashes(
    hashes: &[String],
) -> Result<HashMap<String, ModrinthVersionEntry>, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("CubicLauncher/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let body = serde_json::json!({
        "hashes": hashes,
        "algorithm": "sha1",
    });

    let resp = client
        .post(format!("{}/version_files/update", MODRINTH_API))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Modrinth API request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Modrinth API returned {}", resp.status()));
    }

    let result: HashMap<String, ModrinthVersionEntry> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Modrinth response: {}", e))?;

    Ok(result)
}

#[tauri::command]
pub async fn toggle_instance_mod(id: String, filename: String, enable: bool) -> Result<(), String> {
    validate_uuid(&id)?;
    info!(
        "Cambiando estado del mod '{}' en instancia {}: enable={}",
        filename, id, enable
    );
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        error!("Instancia {} no encontrada para toggle mod", id);
        return Err(InstanceError::NotFound.to_string());
    };

    if handle.is_busy() {
        error!("Intento de toggle mod en instancia ocupada {}", id);
        return Err(InstanceError::Busy.to_string());
    }

    let mods_dir = handle.get_instance_dir().await.join("mods");
    let file_path = mods_dir.join(&filename);

    if !file_path.exists() {
        error!("Mod '{}' no encontrado en instancia {}", filename, id);
        return Err(InstanceError::ModNotFound.to_string());
    }

    let is_currently_disabled = filename.ends_with(".disabled");

    if enable && is_currently_disabled {
        let new_filename = filename
            .strip_suffix(".disabled")
            .ok_or_else(|| InstanceError::FilenameParse.to_string())?;
        let new_path = mods_dir.join(new_filename);
        tokio::fs::rename(file_path, new_path)
            .await
            .map_err(|e| e.to_string())?;
        info!("Mod '{}' habilitado en instancia {}", new_filename, id);
    } else if !enable && !is_currently_disabled {
        let new_filename = format!("{}.disabled", filename);
        let new_path = mods_dir.join(new_filename);
        tokio::fs::rename(file_path, new_path)
            .await
            .map_err(|e| e.to_string())?;
        info!("Mod '{}' deshabilitado en instancia {}", filename, id);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_instance_resourcepacks(id: String) -> Vec<ModDto> {
    if let Err(e) = validate_uuid(&id) {
        warn!("{}", e);
        return Vec::new();
    }
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        warn!("Instancia {} no encontrada para listar resourcepacks", id);
        return Vec::new();
    };

    let resourcepacks_dir = handle.get_instance_dir().await.join("resourcepacks");

    struct FileEntry {
        path: PathBuf,
        filename: String,
        size: u64,
        fingerprint: u64,
    }

    let entries = tokio::task::spawn_blocking({
        let d = resourcepacks_dir.clone();
        move || -> Vec<FileEntry> {
            let dir = match std::fs::read_dir(&d) {
                Ok(d) => d,
                Err(_) => return Vec::new(),
            };
            dir.flatten()
                .filter_map(|e| {
                    let path = e.path();
                    if !path.is_file() {
                        return None;
                    }
                    let filename = path.file_name()?.to_string_lossy().to_string();
                    if is_hidden_or_crep(&filename) {
                        return None;
                    }
                    let meta = std::fs::metadata(&path).ok()?;
                    let mtime = meta.modified().ok()?;
                    let size = meta.len();
                    let fingerprint = file_fingerprint(&filename, &mtime, size);
                    Some(FileEntry {
                        path,
                        filename,
                        size,
                        fingerprint,
                    })
                })
                .collect()
        }
    })
    .await
    .unwrap_or_default();

    if entries.is_empty() {
        return Vec::new();
    }

    let dir_clone = entries[0].path.parent().unwrap().to_path_buf();
    let cache_path = crate::services::pack_cache_path_custom(&dir_clone);

    let global_fp: u64 = entries.iter().fold(0, |acc, e| acc ^ e.fingerprint);

    if ablage::Repo::check_global_fingerprint(&cache_path, global_fp) {
        let cache = read_all_full_pack_cache(&dir_clone);
        let mut resourcepacks: Vec<ModDto> = entries
            .into_iter()
            .map(|e| {
                let cached = cache.get(&e.filename);
                match cached {
                    Some(entry) => {
                        let (md_name, md_desc) = entry
                            .metadata
                            .as_ref()
                            .map(|m| (m.name.clone(), m.description.clone()))
                            .unwrap_or_else(|| (e.filename.clone(), None));
                        ModDto {
                            name: md_name,
                            filename: e.filename,
                            version: entry.metadata.as_ref().and_then(|m| m.version.clone()),
                            description: md_desc,
                            authors: entry.metadata.as_ref().and_then(|m| m.authors.clone()),
                            icon: entry.icon.clone(),
                            enabled: true,
                            sha1: entry.sha1.clone(),
                            file_size: e.size,
                            source: entry.source.source_str().to_string(),
                            project_id: entry.source.project_id().map(|s| s.to_string()),
                            slug: entry.source.slug().map(|s| s.to_string()),
                        }
                    }
                    None => ModDto {
                        name: e.filename.clone(),
                        filename: e.filename,
                        version: None,
                        description: None,
                        authors: None,
                        icon: None,
                        enabled: true,
                        sha1: String::new(),
                        file_size: e.size,
                        source: "local".to_string(),
                        project_id: None,
                        slug: None,
                    },
                }
            })
            .collect();
        resourcepacks.sort_by_key(|a| a.name.to_lowercase());
        info!(
            "{} resourcepacks cargados desde cache en instancia {}",
            resourcepacks.len(),
            id
        );
        return resourcepacks;
    }

    // Cache miss: return minimal, enrich in background
    let minimal: Vec<ModDto> = entries
        .iter()
        .map(|e| ModDto {
            name: e.filename.clone(),
            filename: e.filename.clone(),
            version: None,
            description: None,
            authors: None,
            icon: None,
            enabled: true,
            sha1: String::new(),
            file_size: e.size,
            source: "local".to_string(),
            project_id: None,
            slug: None,
        })
        .collect();

    info!(
        "{} resourcepacks listados (minimal) en instancia {} — enriqueciendo en background",
        minimal.len(),
        id
    );

    let id2 = id.clone();
    let cache_path2 = cache_path.clone();
    tokio::spawn(async move {
        let handles: Vec<_> = entries
            .into_iter()
            .map(|e| {
                let path = e.path.clone();
                let filename = e.filename.clone();
                let fingerprint = e.fingerprint;
                tokio::task::spawn_blocking(move || -> (String, u64, PackFullCacheEntry) {
                    let sha1 = compute_file_sha1(&path).unwrap_or_default();
                    let (meta, icon) = AddonManager::get_resourcepack_info_full(&path);
                    let icon_str = icon.map(|s| (*s).clone());
                    let entry = PackFullCacheEntry {
                        sha1: sha1.clone(),
                        metadata: meta,
                        icon: icon_str,
                        source: ModSource::Local,
                    };
                    (filename, fingerprint, entry)
                })
            })
            .collect();

        let mut results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        let mut repo = ablage::Repo::open(&cache_path2);
        for (filename, fingerprint, entry) in &mut results {
            preserve_pack_source(entry, &repo, filename);
            if let Ok(data) = postcard::to_stdvec(entry) {
                repo.put(
                    filename.clone(),
                    ablage::Entry {
                        version: 1,
                        fingerprint: *fingerprint,
                        data,
                    },
                );
            }
        }
        repo.put(
            "__global",
            ablage::Entry {
                version: 1,
                fingerprint: global_fp,
                data: global_fp.to_le_bytes().to_vec(),
            },
        );
        let _ = repo.flush();

        event_bus::emit(event_bus::AppEvent::ResourcepacksEnriched { id: id2.into() });
    });

    minimal
}

#[tauri::command]
pub async fn get_instance_shaderpacks(id: String) -> Vec<ModDto> {
    if let Err(e) = validate_uuid(&id) {
        warn!("{}", e);
        return Vec::new();
    }
    let manager = InstanceManager::get();
    let Some(handle) = manager.get_handle(&id).await else {
        warn!("Instancia {} no encontrada para listar shaderpacks", id);
        return Vec::new();
    };

    let shaderpacks_dir = handle.get_instance_dir().await.join("shaderpacks");

    struct FileEntry {
        path: PathBuf,
        filename: String,
        size: u64,
        fingerprint: u64,
    }

    let entries = tokio::task::spawn_blocking({
        let d = shaderpacks_dir.clone();
        move || -> Vec<FileEntry> {
            let dir = match std::fs::read_dir(&d) {
                Ok(d) => d,
                Err(_) => return Vec::new(),
            };
            dir.flatten()
                .filter_map(|e| {
                    let path = e.path();
                    if !path.is_file() {
                        return None;
                    }
                    let filename = path.file_name()?.to_string_lossy().to_string();
                    if is_hidden_or_crep(&filename) {
                        return None;
                    }
                    let meta = std::fs::metadata(&path).ok()?;
                    let mtime = meta.modified().ok()?;
                    let size = meta.len();
                    let fingerprint = file_fingerprint(&filename, &mtime, size);
                    Some(FileEntry {
                        path,
                        filename,
                        size,
                        fingerprint,
                    })
                })
                .collect()
        }
    })
    .await
    .unwrap_or_default();

    if entries.is_empty() {
        return Vec::new();
    }

    let dir_clone = entries[0].path.parent().unwrap().to_path_buf();
    let cache_path = crate::services::pack_cache_path_custom(&dir_clone);

    let global_fp: u64 = entries.iter().fold(0, |acc, e| acc ^ e.fingerprint);

    if ablage::Repo::check_global_fingerprint(&cache_path, global_fp) {
        let cache = read_all_full_pack_cache(&dir_clone);
        let mut shaderpacks: Vec<ModDto> = entries
            .into_iter()
            .map(|e| {
                let cached = cache.get(&e.filename);
                match cached {
                    Some(entry) => {
                        let (md_name, md_desc) = entry
                            .metadata
                            .as_ref()
                            .map(|m| (m.name.clone(), m.description.clone()))
                            .unwrap_or_else(|| (e.filename.clone(), None));
                        ModDto {
                            name: md_name,
                            filename: e.filename,
                            version: entry.metadata.as_ref().and_then(|m| m.version.clone()),
                            description: md_desc,
                            authors: entry.metadata.as_ref().and_then(|m| m.authors.clone()),
                            icon: entry.icon.clone(),
                            enabled: true,
                            sha1: entry.sha1.clone(),
                            file_size: e.size,
                            source: entry.source.source_str().to_string(),
                            project_id: entry.source.project_id().map(|s| s.to_string()),
                            slug: entry.source.slug().map(|s| s.to_string()),
                        }
                    }
                    None => ModDto {
                        name: e.filename.clone(),
                        filename: e.filename,
                        version: None,
                        description: None,
                        authors: None,
                        icon: None,
                        enabled: true,
                        sha1: String::new(),
                        file_size: e.size,
                        source: "local".to_string(),
                        project_id: None,
                        slug: None,
                    },
                }
            })
            .collect();
        shaderpacks.sort_by_key(|a| a.name.to_lowercase());
        info!(
            "{} shaderpacks cargados desde cache en instancia {}",
            shaderpacks.len(),
            id
        );
        return shaderpacks;
    }

    // Cache miss: return minimal, enrich in background
    let minimal: Vec<ModDto> = entries
        .iter()
        .map(|e| ModDto {
            name: e.filename.clone(),
            filename: e.filename.clone(),
            version: None,
            description: None,
            authors: None,
            icon: None,
            enabled: true,
            sha1: String::new(),
            file_size: e.size,
            source: "local".to_string(),
            project_id: None,
            slug: None,
        })
        .collect();

    info!(
        "{} shaderpacks listados (minimal) en instancia {} — enriqueciendo en background",
        minimal.len(),
        id
    );

    let id2 = id.clone();
    let cache_path2 = cache_path.clone();
    tokio::spawn(async move {
        let handles: Vec<_> = entries
            .into_iter()
            .map(|e| {
                let path = e.path.clone();
                let filename = e.filename.clone();
                let fingerprint = e.fingerprint;
                tokio::task::spawn_blocking(move || -> (String, u64, PackFullCacheEntry) {
                    let sha1 = compute_file_sha1(&path).unwrap_or_default();
                    let (meta, icon) = AddonManager::get_shaderpack_info_full(&path);
                    let icon_str = icon.map(|s| (*s).clone());
                    let entry = PackFullCacheEntry {
                        sha1: sha1.clone(),
                        metadata: meta,
                        icon: icon_str,
                        source: ModSource::Local,
                    };
                    (filename, fingerprint, entry)
                })
            })
            .collect();

        let mut results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        let mut repo = ablage::Repo::open(&cache_path2);
        for (filename, fingerprint, entry) in &mut results {
            preserve_pack_source(entry, &repo, filename);
            if let Ok(data) = postcard::to_stdvec(entry) {
                repo.put(
                    filename.clone(),
                    ablage::Entry {
                        version: 1,
                        fingerprint: *fingerprint,
                        data,
                    },
                );
            }
        }
        repo.put(
            "__global",
            ablage::Entry {
                version: 1,
                fingerprint: global_fp,
                data: global_fp.to_le_bytes().to_vec(),
            },
        );
        let _ = repo.flush();

        event_bus::emit(event_bus::AppEvent::ShaderpacksEnriched { id: id2.into() });
    });

    minimal
}
