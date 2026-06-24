use crate::core::errors::InstanceError;
use crate::services::{AddonManager, AddonMetadata, InstanceManager};
use std::path::PathBuf;
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
    let mods_dir2 = mods_dir.clone();
    info!("Listando mods de instancia {} en {:?}", id, mods_dir);

    struct ModEntry {
        path: PathBuf,
        filename: String,
        display_name: String,
        enabled: bool,
        mtime: std::time::SystemTime,
        size: u64,
    }

    let entries = tokio::task::spawn_blocking(move || -> Vec<ModEntry> {
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

                Some(ModEntry {
                    path,
                    filename: file_name,
                    display_name,
                    enabled,
                    mtime,
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

    // cache via ablage
    let repo_path = mods_dir2.join(".mod_cache.crep");
    let mut repo = ablage::Repo::open(&repo_path);

    fn entry_fingerprint(filename: &str, mtime: &std::time::SystemTime, size: u64) -> u64 {
        let nanos = mtime
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let name_hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            filename.hash(&mut h);
            h.finish()
        };
        name_hash ^ nanos ^ size
    }

    // compute global fingerprint (XOR of all individual fingerprints)
    let global_fp: u64 = entries.iter().fold(0, |acc, e| {
        acc ^ entry_fingerprint(&e.filename, &e.mtime, e.size)
    });

    let cache_hit = repo
        .get("__global")
        .and_then(|entry| {
            if entry.data.len() != 8 {
                return None;
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&entry.data);
            Some(u64::from_le_bytes(buf))
        })
        .map(|stored| stored == global_fp)
        .unwrap_or(false);

    if cache_hit {
        // fast path: read all metadata from cache
        let mods: Vec<ModDto> = entries
            .into_iter()
            .map(|e| {
                let meta: Option<AddonMetadata> = repo
                    .get(&e.filename)
                    .and_then(|entry| postcard::from_bytes(&entry.data).ok());
                let (md_name, md_version, md_desc, md_authors, md_icon) = match meta {
                    Some(m) => (m.name, m.version, m.description, m.authors, m.icon),
                    None => (e.display_name, None, None, None, None),
                };
                ModDto {
                    name: md_name,
                    filename: e.filename,
                    version: md_version,
                    description: md_desc,
                    authors: md_authors,
                    icon: md_icon.map(|s| (*s).clone()),
                    enabled: e.enabled,
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

    // slow path: parse only changed files in parallel
    let handles: Vec<_> = entries
        .iter()
        .map(|e| {
            let path = e.path.clone();
            let filename = e.filename.clone();
            let fp = entry_fingerprint(&e.filename, &e.mtime, e.size);
            // check individual cache first
            let cached = repo.get(&filename).and_then(|entry| {
                if entry.fingerprint == fp {
                    postcard::from_bytes::<AddonMetadata>(&entry.data).ok()
                } else {
                    None
                }
            });
            if let Some(meta) = cached {
                // fully cached
                tokio::task::spawn_blocking(move || (filename, fp, Some(meta)))
            } else {
                tokio::task::spawn_blocking(move || {
                    let meta = AddonManager::get_mod_info(&path);
                    (filename, fp, meta)
                })
            }
        })
        .collect();

    let results = futures::future::join_all(handles).await;
    let mut dirty = false;

    let mods: Vec<ModDto> = entries
        .into_iter()
        .zip(results)
        .map(|(entry, result)| {
            let (filename, fp, meta_option) = match result {
                Ok(r) => r,
                Err(_) => {
                    warn!("Error parsing mod {}", entry.filename);
                    return ModDto {
                        name: entry.display_name,
                        filename: entry.filename,
                        version: None,
                        description: None,
                        authors: None,
                        icon: None,
                        enabled: entry.enabled,
                    };
                }
            };
            let (md_name, md_version, md_desc, md_authors, md_icon) = match &meta_option {
                Some(m) => (
                    m.name.clone(),
                    m.version.clone(),
                    m.description.clone(),
                    m.authors.clone(),
                    m.icon.clone().map(|s| (*s).clone()),
                ),
                None => (entry.display_name, None, None, None, None),
            };
            // update cache for this entry
            if let Some(meta) = &meta_option
                && repo.get(&filename).map(|e| e.fingerprint) != Some(fp)
                && let Ok(data) = postcard::to_stdvec(meta)
            {
                repo.put(
                    filename,
                    ablage::Entry {
                        version: 1,
                        fingerprint: fp,
                        data,
                    },
                );
                dirty = true;
            }
            ModDto {
                name: md_name,
                filename: entry.filename,
                version: md_version,
                description: md_desc,
                authors: md_authors,
                icon: md_icon,
                enabled: entry.enabled,
            }
        })
        .collect();

    // update global fingerprint
    let global_entry = ablage::Entry {
        version: 1,
        fingerprint: 0,
        data: global_fp.to_le_bytes().to_vec(),
    };
    repo.put("__global", global_entry);
    dirty = true;

    if dirty {
        let _ = repo.flush();
    }

    info!("{} mods parseados en instancia {}", mods.len(), id);
    mods
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
        let metadata =
            tokio::task::spawn_blocking(move || AddonManager::get_resourcepack_info(&path_clone))
                .await
                .unwrap_or(None);

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
        let metadata =
            tokio::task::spawn_blocking(move || AddonManager::get_shaderpack_info(&path_clone))
                .await
                .unwrap_or(None);

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
