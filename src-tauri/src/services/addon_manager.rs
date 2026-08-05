use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;
use tracing::{debug, warn};
use zip::ZipArchive;

// Cache unificado de metadata + icono (se extraen en una sola pasada del ZIP)
const MAX_CACHE_ENTRIES: usize = 200;

type CachedModInfo = (SystemTime, Option<AddonMetaNoIcon>, Option<Arc<String>>);

static ADDON_CACHE: LazyLock<Mutex<HashMap<PathBuf, CachedModInfo>>> =
    LazyLock::new(|| Mutex::new(HashMap::with_capacity(128)));

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddonMetaNoIcon {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModSource {
    Local,
    Modrinth {
        project_id: String,
        version_id: String,
        slug: Option<String>,
    },
    CurseForge {
        project_id: String,
        file_id: String,
    },
}

impl ModSource {
    pub fn source_str(&self) -> &str {
        match self {
            ModSource::Local => "local",
            ModSource::Modrinth { .. } => "modrinth",
            ModSource::CurseForge { .. } => "curseforge",
        }
    }

    pub fn project_id(&self) -> Option<&str> {
        match self {
            ModSource::Local => None,
            ModSource::Modrinth { project_id, .. } => Some(project_id),
            ModSource::CurseForge { project_id, .. } => Some(project_id),
        }
    }

    pub fn slug(&self) -> Option<&str> {
        match self {
            ModSource::Modrinth { slug, .. } => slug.as_deref(),
            _ => None,
        }
    }
}

/// Compute SHA1 of a file synchronously (for use in spawn_blocking)
pub fn compute_file_sha1(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open {:?}: {}", path, e))?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    Ok(hash.iter().map(|b| format!("{:02x}", b)).collect())
}

// ── Pack cache (resourcepacks / shaderpacks) via ablage ──────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackCacheEntry {
    pub source: ModSource,
}

/// Richer entry: stores metadata + icon + sha1 + source in one cache hit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackFullCacheEntry {
    pub sha1: String,
    pub metadata: Option<AddonMetaNoIcon>,
    pub icon: Option<String>,
    pub source: ModSource,
}

fn pack_cache_path(dir: &Path) -> PathBuf {
    dir.join(".pack_cache.crep")
}

pub fn pack_cache_path_custom(dir: &Path) -> PathBuf {
    dir.join(".pack_cache.crep")
}

/// Read all full pack cache entries keyed by filename (includes metadata + icon)
pub fn read_all_full_pack_cache(dir: &Path) -> HashMap<String, PackFullCacheEntry> {
    let path = pack_cache_path(dir);
    if !path.exists() {
        return HashMap::new();
    }
    let repo = ablage::Repo::open(&path);
    let mut map = HashMap::new();
    for key in repo.keys() {
        if let Some(entry) = repo
            .get(key)
            .and_then(|e| postcard::from_bytes::<PackFullCacheEntry>(&e.data).ok())
        {
            map.insert(key.clone(), entry);
        }
    }
    map
}

pub fn save_pack_cache(dir: &Path, filename: &str, entry: &PackCacheEntry) {
    let mut repo = ablage::Repo::open(pack_cache_path(dir));
    if let Ok(data) = postcard::to_stdvec(entry) {
        repo.put(
            filename.to_string(),
            ablage::Entry {
                version: 1,
                fingerprint: 0,
                data,
            },
        );
        let _ = repo.flush();
    }
}

pub fn remove_pack_cache(dir: &Path, filename: &str) {
    let path = pack_cache_path(dir);
    if !path.exists() {
        return;
    }
    let mut repo = ablage::Repo::open(&path);
    repo.remove(filename);
    let _ = repo.flush();
}

pub fn file_fingerprint(filename: &str, mtime: &std::time::SystemTime, size: u64) -> u64 {
    let nanos = mtime
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    filename.hash(&mut h);
    let name_hash = h.finish();
    name_hash ^ nanos ^ size
}

type ParserFn = fn(&mut ZipArchive<File>) -> Result<AddonMetaNoIcon, ()>;
const MOD_PARSERS: &[ParserFn] = &[
    AddonManager::try_parse_fabric,
    AddonManager::try_parse_quilt,
    AddonManager::try_parse_forge_modern,
    AddonManager::try_parse_forge_legacy,
];
pub struct AddonManager;

impl AddonManager {
    fn cached_or_parse(
        path: &Path,
        parse_fn: impl FnOnce(&mut ZipArchive<File>) -> Option<AddonMetaNoIcon>,
    ) -> Option<AddonMetaNoIcon> {
        let mtime = std::fs::metadata(path).ok()?.modified().ok()?;

        {
            let cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((cached_mtime, cached_result, _)) = cache.get(path)
                && *cached_mtime == mtime
            {
                return cached_result.clone();
            }
        }

        let (meta, icon) = Self::parse_meta_and_icon(path, parse_fn);

        let mut cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= MAX_CACHE_ENTRIES {
            // Evicción parcial: mantener la mitad más reciente
            let mut entries: Vec<_> = cache.drain().collect();
            entries.sort_by_key(|b| std::cmp::Reverse(b.1.0));
            cache.extend(entries.into_iter().take(MAX_CACHE_ENTRIES / 2));
        }
        cache.insert(path.to_path_buf(), (mtime, meta.clone(), icon));

        meta
    }

    fn parse_meta_and_icon(
        path: &Path,
        parse_fn: impl FnOnce(&mut ZipArchive<File>) -> Option<AddonMetaNoIcon>,
    ) -> (Option<AddonMetaNoIcon>, Option<Arc<String>>) {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                debug!("No se pudo abrir {:?}: {}", path, e);
                return (None, None);
            }
        };
        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                debug!("No se pudo leer ZIP {:?}: {}", path, e);
                return (None, None);
            }
        };
        let meta = parse_fn(&mut archive);
        let icon = Self::extract_icon_from_archive(&mut archive);
        (meta, icon)
    }

    /// Extrae icono desde un archive ya abierto (evita re-abrir el ZIP)
    fn extract_icon_from_archive(archive: &mut ZipArchive<File>) -> Option<Arc<String>> {
        for icon_path in &[
            "pack.png",
            "fabric.mod.json",
            "quilt.mod.json",
            "META-INF/mods.toml",
            "mcmod.info",
        ] {
            let icon_name: Option<String> = match *icon_path {
                "pack.png" => Some("pack.png".to_string()),
                "fabric.mod.json" => Self::icon_from_fabric_json(archive),
                "quilt.mod.json" => Self::icon_from_quilt_json(archive),
                "META-INF/mods.toml" => Self::icon_from_forge_toml(archive),
                "mcmod.info" => Self::icon_from_mcmod_info(archive),
                _ => None,
            };
            if let Some(ref name) = icon_name {
                let clean = name.trim_start_matches('/');
                if let Ok(mut f) = archive.by_name(clean) {
                    let mut buf = Vec::new();
                    if f.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                        let mime = if clean.ends_with(".png") {
                            "image/png"
                        } else if clean.ends_with(".jpg") || clean.ends_with(".jpeg") {
                            "image/jpeg"
                        } else if clean.ends_with(".svg") {
                            "image/svg+xml"
                        } else {
                            "image/png"
                        };
                        return Some(Arc::new(format!(
                            "data:{};base64,{}",
                            mime,
                            general_purpose::STANDARD.encode(buf)
                        )));
                    }
                }
            }
        }
        None
    }

    /// Extrae solo el icono, reutilizando el cache combinado
    pub fn get_mod_icon(path: &Path) -> Option<Arc<String>> {
        let mtime = std::fs::metadata(path).ok()?.modified().ok()?;
        {
            let cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((cached_mtime, _, cached_icon)) = cache.get(path)
                && *cached_mtime == mtime
            {
                return cached_icon.clone();
            }
        }
        let file = File::open(path).ok()?;
        let mut archive = ZipArchive::new(file).ok()?;
        let result = Self::extract_icon_from_archive(&mut archive);
        let mut cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= MAX_CACHE_ENTRIES {
            let mut entries: Vec<_> = cache.drain().collect();
            entries.sort_by_key(|b| std::cmp::Reverse(b.1.0));
            cache.extend(entries.into_iter().take(MAX_CACHE_ENTRIES / 2));
        }
        cache.insert(path.to_path_buf(), (mtime, None, result.clone()));
        result
    }

    fn icon_from_json(
        archive: &mut ZipArchive<File>,
        json_path: &str,
        icon_key: &str,
    ) -> Option<String> {
        let mut file = archive.by_name(json_path).ok()?;
        let mut content = String::new();
        file.read_to_string(&mut content).ok()?;
        let val: serde_json::Value = serde_json::from_str(&content).ok()?;
        val.get(icon_key)?.as_str().map(|s| s.to_string())
    }

    fn icon_from_fabric_json(archive: &mut ZipArchive<File>) -> Option<String> {
        Self::icon_from_json(archive, "fabric.mod.json", "icon")
    }

    fn icon_from_quilt_json(archive: &mut ZipArchive<File>) -> Option<String> {
        let mut file = archive.by_name("quilt.mod.json").ok()?;
        let mut content = String::new();
        file.read_to_string(&mut content).ok()?;
        let val: serde_json::Value = serde_json::from_str(&content).ok()?;
        val.get("quilt_loader")?
            .get("metadata")?
            .get("icon")?
            .as_str()
            .map(|s| s.to_string())
    }

    fn icon_from_forge_toml(archive: &mut ZipArchive<File>) -> Option<String> {
        let mut file = archive.by_name("META-INF/mods.toml").ok()?;
        let mut content = String::new();
        file.read_to_string(&mut content).ok()?;
        let toml_val: toml::Value = toml::from_str(&content).ok()?;
        toml_val
            .get("mods")?
            .as_array()?
            .first()?
            .get("logoFile")?
            .as_str()
            .map(|s| s.to_string())
    }

    fn icon_from_mcmod_info(archive: &mut ZipArchive<File>) -> Option<String> {
        let mut file = archive.by_name("mcmod.info").ok()?;
        let mut content = String::new();
        file.read_to_string(&mut content).ok()?;
        let val: serde_json::Value = serde_json::from_str(&content).ok()?;
        let first = val.as_array()?.first()?;
        first
            .get("logoFile")?
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| {
                val.get("modList")?
                    .as_array()?
                    .first()?
                    .get("logoFile")?
                    .as_str()
                    .map(|s| s.to_string())
            })
    }

    pub fn get_mod_info(path: &Path) -> Option<AddonMetaNoIcon> {
        Self::cached_or_parse(path, |archive| {
            for parser in MOD_PARSERS {
                if let Ok(meta) = parser(archive) {
                    debug!("Mod detectado: {:?}", path);
                    return Some(meta);
                }
            }
            warn!("No se pudo detectar tipo de mod: {:?}", path);
            None
        })
    }

    /// Parse both metadata and icon from a ZIP in one pass (resourcepacks)
    pub fn get_resourcepack_info_full(
        path: &Path,
    ) -> (Option<AddonMetaNoIcon>, Option<Arc<String>>) {
        let mtime = match std::fs::metadata(path).and_then(|m| m.modified()) {
            Ok(m) => m,
            Err(_) => return (None, None),
        };
        {
            let cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((cached_mtime, meta, icon)) = cache.get(path)
                && *cached_mtime == mtime
            {
                return (meta.clone(), icon.clone());
            }
        }

        let (meta, icon) = Self::parse_pack_mcmeta_and_icon(path);

        let mut cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= MAX_CACHE_ENTRIES {
            let mut entries: Vec<_> = cache.drain().collect();
            entries.sort_by_key(|b| std::cmp::Reverse(b.1.0));
            cache.extend(entries.into_iter().take(MAX_CACHE_ENTRIES / 2));
        }
        cache.insert(path.to_path_buf(), (mtime, meta.clone(), icon.clone()));

        (meta, icon)
    }

    /// Parse both metadata and icon from a ZIP in one pass (shaderpacks)
    pub fn get_shaderpack_info_full(path: &Path) -> (Option<AddonMetaNoIcon>, Option<Arc<String>>) {
        if path.extension().is_some_and(|ext| ext == "txt") {
            return (None, None);
        }
        let mtime = match std::fs::metadata(path).and_then(|m| m.modified()) {
            Ok(m) => m,
            Err(_) => return (None, None),
        };
        {
            let cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((cached_mtime, meta, icon)) = cache.get(path)
                && *cached_mtime == mtime
            {
                return (meta.clone(), icon.clone());
            }
        }

        let (meta, icon) = Self::parse_pack_mcmeta_and_icon(path);

        let mut cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= MAX_CACHE_ENTRIES {
            let mut entries: Vec<_> = cache.drain().collect();
            entries.sort_by_key(|b| std::cmp::Reverse(b.1.0));
            cache.extend(entries.into_iter().take(MAX_CACHE_ENTRIES / 2));
        }
        cache.insert(path.to_path_buf(), (mtime, meta.clone(), icon.clone()));

        (meta, icon)
    }

    fn parse_pack_mcmeta_and_icon(path: &Path) -> (Option<AddonMetaNoIcon>, Option<Arc<String>>) {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                debug!("No se pudo abrir {:?}: {}", path, e);
                return (None, None);
            }
        };
        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                debug!("No se pudo leer ZIP {:?}: {}", path, e);
                return (None, None);
            }
        };

        let meta = Self::parse_pack_mcmeta_inner(&mut archive, path);
        let icon = Self::extract_icon_from_archive(&mut archive);
        (meta, icon)
    }

    fn parse_pack_mcmeta_inner(
        archive: &mut ZipArchive<File>,
        path: &Path,
    ) -> Option<AddonMetaNoIcon> {
        let json: serde_json::Value = {
            let mut file = archive.by_name("pack.mcmeta").ok()?;
            let mut content = String::new();
            file.read_to_string(&mut content).ok()?;
            serde_json::from_str(&content).ok()?
        };

        let description = json["pack"]["description"]
            .as_str()
            .or_else(|| json["pack"]["description"]["text"].as_str())
            .map(|s| s.to_string());

        let name = path.file_stem()?.to_string_lossy().to_string();

        Some(AddonMetaNoIcon {
            name,
            version: None,
            description,
            authors: None,
        })
    }

    fn read_zip_json(archive: &mut ZipArchive<File>, path: &str) -> Result<serde_json::Value, ()> {
        let mut file = archive.by_name(path).map_err(|_| ())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|_| ())?;
        serde_json::from_str(&content).map_err(|_| ())
    }

    fn read_zip_toml(archive: &mut ZipArchive<File>, path: &str) -> Result<serde_json::Value, ()> {
        let mut file = archive.by_name(path).map_err(|_| ())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|_| ())?;
        let toml_val: toml::Value = toml::from_str(&content).map_err(|_| ())?;
        serde_json::to_value(toml_val).map_err(|_| ())
    }

    fn try_parse_fabric(archive: &mut ZipArchive<File>) -> Result<AddonMetaNoIcon, ()> {
        let json = Self::read_zip_json(archive, "fabric.mod.json")?;
        let name = json["name"].as_str().ok_or(())?.to_string();
        let version = json["version"].as_str().map(|s| s.to_string());
        let description = json["description"].as_str().map(|s| s.to_string());

        let authors = json.get("authors").and_then(|v| {
            v.as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| a.as_str().or_else(|| a["name"].as_str()).map(String::from))
                        .collect()
                })
                .or_else(|| v.as_str().map(|s| vec![s.to_string()]))
        });

        Ok(AddonMetaNoIcon {
            name,
            version,
            description,
            authors,
        })
    }

    fn try_parse_quilt(archive: &mut ZipArchive<File>) -> Result<AddonMetaNoIcon, ()> {
        let json = Self::read_zip_json(archive, "quilt.mod.json")?;
        let metadata = json
            .get("quilt_loader")
            .and_then(|ql| ql.get("metadata"))
            .unwrap_or(&json);

        let name = metadata["name"].as_str().ok_or(())?.to_string();
        let version = json
            .get("quilt_loader")
            .and_then(|ql| ql["version"].as_str())
            .map(|s| s.to_string());
        let description = metadata["description"].as_str().map(|s| s.to_string());
        let authors = metadata
            .get("contributors")
            .and_then(|c| c.as_object())
            .map(|map| map.keys().cloned().collect());

        Ok(AddonMetaNoIcon {
            name,
            version,
            description,
            authors,
        })
    }

    fn try_parse_forge_modern(archive: &mut ZipArchive<File>) -> Result<AddonMetaNoIcon, ()> {
        let toml_val = Self::read_zip_toml(archive, "META-INF/mods.toml")?;
        let first_mod = toml_val
            .get("mods")
            .and_then(|m| m.as_array())
            .and_then(|a| a.first())
            .ok_or(())?;

        let name = first_mod
            .get("displayName")
            .and_then(|v| v.as_str())
            .or_else(|| first_mod.get("modId").and_then(|v| v.as_str()))
            .ok_or(())?
            .to_string();

        let version = first_mod
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let description = first_mod
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let authors = first_mod
            .get("authors")
            .and_then(|v| v.as_str())
            .map(|s| vec![s.to_string()]);

        Ok(AddonMetaNoIcon {
            name,
            version,
            description,
            authors,
        })
    }

    fn try_parse_forge_legacy(archive: &mut ZipArchive<File>) -> Result<AddonMetaNoIcon, ()> {
        let json = Self::read_zip_json(archive, "mcmod.info")?;
        let mod_data = json
            .as_array()
            .and_then(|a| a.first())
            .or_else(|| json.get("modList").and_then(|m| m.as_array())?.first())
            .unwrap_or(&json);

        let name = mod_data["name"].as_str().ok_or(())?.to_string();
        let version = mod_data["version"].as_str().map(|s| s.to_string());
        let description = mod_data["description"].as_str().map(|s| s.to_string());
        let authors = mod_data["authorList"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

        Ok(AddonMetaNoIcon {
            name,
            version,
            description,
            authors,
        })
    }
}
