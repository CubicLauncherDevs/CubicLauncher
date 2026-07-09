use crate::core::errors::InstanceError;
use crate::services::{compute_file_sha1, ModSource};
use crate::services::{AddonManager, AddonMetadata, InstanceManager};
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

/// Cached in ablage, keyed by SHA1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModCacheEntry {
    pub metadata: Option<AddonMetadata>,
    pub source: ModSource,
}

#[derive(Debug, Deserialize)]
struct ModrinthVersionEntry {
    id: String,
    project_id: String,
}

const MODRINTH_API: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = concat!("CubicLauncher/", env!("CARGO_PKG_VERSION"));

pub(crate) fn repo_path(mods_dir: &Path) -> PathBuf {
    mods_dir.join(".mod_cache.crep")
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

    // --- Phase 1: List files ---
    struct FileEntry {
        path: PathBuf,
        filename: String,
        display_name: String,
        enabled: bool,
        size: u64,
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
                let size = meta.len();

                Some(FileEntry {
                    path,
                    filename: file_name,
                    display_name,
                    enabled,
                    size,
                })
            })
            .collect()
    })
    .await
    .unwrap_or_default();

    if entries.is_empty() {
        return Vec::new();
    }

    let mods_dir2 = match entries.first() {
        Some(e) => e.path.parent().unwrap().to_path_buf(),
        None => return Vec::new(),
    };
    let repo_path = repo_path(&mods_dir2);

    // --- Phase 2: Compute SHA1 + parse JARs in parallel ---
    struct RawResult {
        filename: String,
        display_name: String,
        enabled: bool,
        sha1: String,
        size: u64,
        metadata: Option<AddonMetadata>,
    }

    let handles: Vec<_> = entries
        .into_iter()
        .map(|e| {
            let path = e.path;
            let filename = e.filename;
            let display_name = e.display_name;
            let enabled = e.enabled;
            let size = e.size;

            tokio::task::spawn_blocking(move || -> RawResult {
                let sha1 = compute_file_sha1(&path).unwrap_or_default();
                let parsed = AddonManager::get_mod_info(&path);
                RawResult {
                    filename,
                    display_name,
                    enabled,
                    sha1,
                    size,
                    metadata: parsed,
                }
            })
        })
        .collect();

    let raw_results: Vec<RawResult> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // --- Phase 3: Resolve from cache + auto-enrich via Modrinth API ---
    let mut repo = ablage::Repo::open(&repo_path);
    let mut to_resolve: Vec<String> = Vec::new();
    let mut dirty = false;

    let mut cached: Vec<(RawResult, ModCacheEntry)> = Vec::with_capacity(raw_results.len());

    for r in &raw_results {
        if r.sha1.is_empty() {
            cached.push((RawResult {
                sha1: r.sha1.clone(),
                ..RawResult {
                    filename: r.filename.clone(),
                    display_name: r.display_name.clone(),
                    enabled: r.enabled,
                    sha1: String::new(),
                    size: r.size,
                    metadata: r.metadata.clone(),
                }
            }, ModCacheEntry {
                metadata: r.metadata.clone(),
                source: ModSource::Local,
            }));
            continue;
        }

        if let Some(entry) = repo.get(&r.sha1) {
            if let Ok(cached_entry) = postcard::from_bytes::<ModCacheEntry>(&entry.data) {
                cached.push((RawResult {
                    sha1: r.sha1.clone(),
                    ..RawResult {
                        filename: r.filename.clone(),
                        display_name: r.display_name.clone(),
                        enabled: r.enabled,
                        sha1: r.sha1.clone(),
                        size: r.size,
                        metadata: r.metadata.clone(),
                    }
                }, cached_entry));
                continue;
            }
        }

        // Cache miss: use parsed metadata (if any) + Local source initially
        let entry = ModCacheEntry {
            metadata: r.metadata.clone(),
            source: ModSource::Local,
        };
        if let Ok(data) = postcard::to_stdvec(&entry) {
            repo.put(
                r.sha1.clone(),
                ablage::Entry {
                    version: 1,
                    fingerprint: 0,
                    data,
                },
            );
            dirty = true;
        }
        to_resolve.push(r.sha1.clone());
        cached.push((RawResult {
            sha1: r.sha1.clone(),
            ..RawResult {
                filename: r.filename.clone(),
                display_name: r.display_name.clone(),
                enabled: r.enabled,
                sha1: r.sha1.clone(),
                size: r.size,
                metadata: r.metadata.clone(),
            }
        }, entry));
    }

    // --- Phase 4: Batch-resolve unresolved SHA1s via Modrinth ---
    if !to_resolve.is_empty() {
        // Filter to only those still marked Local (not yet enriched)
        let pending: Vec<String> = cached
            .iter()
            .filter(|(_, entry)| matches!(entry.source, ModSource::Local))
            .filter(|(r, _)| !r.sha1.is_empty())
            .map(|(r, _)| r.sha1.clone())
            .collect();

        if !pending.is_empty() {
            info!(
                "Resolviendo {} mods via Modrinth hash lookup en instancia {}",
                pending.len(),
                id
            );
            match resolve_modrinth_hashes(&pending).await {
                Ok(api_results) => {
                    for (sha1, version) in api_results {
                        let source = ModSource::Modrinth {
                            project_id: version.project_id,
                            version_id: version.id,
                            slug: None,
                        };
                        // Update cache
                        let updated = ModCacheEntry {
                            source: source.clone(),
                            // Keep existing metadata if present
                            metadata: cached
                                .iter()
                                .find(|(r, _)| r.sha1 == sha1)
                                .and_then(|(r, _)| r.metadata.clone()),
                        };
                        if let Ok(data) = postcard::to_stdvec(&updated) {
                            repo.put(
                                sha1.clone(),
                                ablage::Entry {
                                    version: 1,
                                    fingerprint: 0,
                                    data,
                                },
                            );
                            dirty = true;
                        }
                        // Update in-memory entry
                        if let Some((_, entry)) =
                            cached.iter_mut().find(|(r, _)| r.sha1 == sha1)
                        {
                            *entry = updated;
                        }
                    }
                }
                Err(e) => {
                    warn!("Error resolviendo hashes via Modrinth: {}", e);
                }
            }
        }
    }

    if dirty {
        let _ = repo.flush();
    }

    // --- Phase 5: Build ModDto list ---
    let mods: Vec<ModDto> = cached
        .into_iter()
        .map(|(raw, entry)| {
            let md = entry.metadata.as_ref();
            ModDto {
                name: md
                    .map(|m| m.name.clone())
                    .unwrap_or(raw.display_name),
                filename: raw.filename,
                version: md.and_then(|m| m.version.clone()),
                description: md.and_then(|m| m.description.clone()),
                authors: md.map(|m| {
                    m.authors
                        .clone()
                        .unwrap_or_default()
                }),
                icon: md
                    .and_then(|m| m.icon.clone().map(|s| (*s).clone())),
                enabled: raw.enabled,
                sha1: raw.sha1,
                file_size: raw.size,
                source: entry.source.source_str().to_string(),
                project_id: entry.source.project_id().map(|s| s.to_string()),
                slug: entry.source.slug().map(|s| s.to_string()),
            }
        })
        .collect();

    info!("{} mods listados en instancia {}", mods.len(), id);
    mods
}

async fn resolve_modrinth_hashes(
    hashes: &[String],
) -> Result<HashMap<String, ModrinthVersionEntry>, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
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

    let rp_paths = tokio::task::spawn_blocking(move || -> Vec<PathBuf> {
        match std::fs::read_dir(&resourcepacks_dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| e.path().is_file())
                .map(|e| e.path())
                .collect(),
            Err(_) => Vec::new(),
        }
    })
    .await
    .unwrap_or_default();

    let mut resourcepacks = Vec::new();
    for path in rp_paths {
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let filename = file_name.to_string_lossy().to_string();
        let path_clone = path.clone();
        let sha1_fut = tokio::task::spawn_blocking(move || compute_file_sha1(&path_clone));
        let size = std::fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0);
        let metadata_fut =
            tokio::task::spawn_blocking({
                let p = path.clone();
                move || AddonManager::get_resourcepack_info(&p)
            });

        let (sha1, metadata) = tokio::join!(sha1_fut, metadata_fut);
        let sha1 = sha1.unwrap_or(Ok(String::new())).unwrap_or_default();
        let metadata = metadata.unwrap_or(None);

        let (md_name, md_desc, md_icon) = match metadata {
            Some(m) => (m.name, m.description, m.icon),
            None => (filename.clone(), None, None),
        };

        resourcepacks.push(ModDto {
            name: md_name,
            filename,
            version: None,
            description: md_desc,
            authors: None,
            icon: md_icon.map(|s| (*s).clone()),
            enabled: true,
            sha1,
            file_size: size,
            source: "local".to_string(),
            project_id: None,
            slug: None,
        });
    }
    resourcepacks.sort_by_key(|a| a.name.to_lowercase());
    info!(
        "{} resourcepacks encontrados en instancia {}",
        resourcepacks.len(),
        id
    );
    resourcepacks
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

    let sp_paths = tokio::task::spawn_blocking(move || -> Vec<PathBuf> {
        match std::fs::read_dir(&shaderpacks_dir) {
            Ok(entries) => entries
                .flatten()
                .filter(|e| e.path().is_file())
                .map(|e| e.path())
                .collect(),
            Err(_) => Vec::new(),
        }
    })
    .await
    .unwrap_or_default();

    let mut shaderpacks = Vec::new();
    for path in sp_paths {
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let filename = file_name.to_string_lossy().to_string();
        let path_clone = path.clone();
        let sha1_fut = tokio::task::spawn_blocking(move || compute_file_sha1(&path_clone));
        let size = std::fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0);
        let metadata_fut =
            tokio::task::spawn_blocking({
                let p = path.clone();
                move || AddonManager::get_shaderpack_info(&p)
            });

        let (sha1, metadata) = tokio::join!(sha1_fut, metadata_fut);
        let sha1 = sha1.unwrap_or(Ok(String::new())).unwrap_or_default();
        let metadata = metadata.unwrap_or(None);

        let (md_name, md_desc, md_icon) = match metadata {
            Some(m) => (m.name, m.description, m.icon),
            None => (filename.clone(), None, None),
        };

        shaderpacks.push(ModDto {
            name: md_name,
            filename,
            version: None,
            description: md_desc,
            authors: None,
            icon: md_icon.map(|s| (*s).clone()),
            enabled: true,
            sha1,
            file_size: size,
            source: "local".to_string(),
            project_id: None,
            slug: None,
        });
    }
    shaderpacks.sort_by_key(|a| a.name.to_lowercase());
    info!(
        "{} shaderpacks encontrados en instancia {}",
        shaderpacks.len(),
        id
    );
    shaderpacks
}
