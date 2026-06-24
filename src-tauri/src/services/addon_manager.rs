use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;
use tracing::{debug, warn};
use zip::ZipArchive;

const MAX_CACHE_ENTRIES: usize = 500;

type CacheEntry = (SystemTime, Option<AddonMetadata>);

static ADDON_CACHE: LazyLock<Mutex<HashMap<PathBuf, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::with_capacity(128)));

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddonMetadata {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
    pub icon: Option<Arc<String>>, // Base64
}

type ParserFn = fn(&mut ZipArchive<File>) -> Result<AddonMetadata, ()>;
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
        parse_fn: impl FnOnce(&mut ZipArchive<File>) -> Option<AddonMetadata>,
    ) -> Option<AddonMetadata> {
        let mtime = std::fs::metadata(path).ok()?.modified().ok()?;

        {
            let cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((cached_mtime, cached_result)) = cache.get(path)
                && *cached_mtime == mtime
            {
                return cached_result.clone();
            }
        }

        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                debug!("No se pudo abrir {:?}: {}", path, e);
                return None;
            }
        };
        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                debug!("No se pudo leer ZIP {:?}: {}", path, e);
                return None;
            }
        };
        let result = parse_fn(&mut archive);

        let mut cache = ADDON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= MAX_CACHE_ENTRIES {
            cache.clear();
        }
        cache.insert(path.to_path_buf(), (mtime, result.clone()));

        result
    }

    pub fn get_mod_info(path: &Path) -> Option<AddonMetadata> {
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

    pub fn get_resourcepack_info(path: &Path) -> Option<AddonMetadata> {
        Self::cached_or_parse(path, |archive| {
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

            let icon = Self::extract_icon(archive, "pack.png");
            let name = path.file_stem()?.to_string_lossy().to_string();

            debug!("Resourcepack '{}' cargado: {:?}", name, description);
            Some(AddonMetadata {
                name,
                version: None,
                description,
                authors: None,
                icon,
            })
        })
    }
    pub fn get_shaderpack_info(path: &Path) -> Option<AddonMetadata> {
        Self::cached_or_parse(path, |archive| {
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

            let icon = Self::extract_icon(archive, "pack.png");
            let name = path.file_stem()?.to_string_lossy().to_string();

            debug!("Shaderpack '{}' cargado: {:?}", name, description);
            Some(AddonMetadata {
                name,
                version: None,
                description,
                authors: None,
                icon,
            })
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

    fn try_parse_fabric(archive: &mut ZipArchive<File>) -> Result<AddonMetadata, ()> {
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

        let icon = json["icon"]
            .as_str()
            .and_then(|p| Self::extract_icon(archive, p));

        Ok(AddonMetadata {
            name,
            version,
            description,
            authors,
            icon,
        })
    }

    fn try_parse_quilt(archive: &mut ZipArchive<File>) -> Result<AddonMetadata, ()> {
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

        let icon = metadata["icon"]
            .as_str()
            .and_then(|p| Self::extract_icon(archive, p));

        Ok(AddonMetadata {
            name,
            version,
            description,
            authors,
            icon,
        })
    }

    fn try_parse_forge_modern(archive: &mut ZipArchive<File>) -> Result<AddonMetadata, ()> {
        // let content = {
        //     let mut file = archive.by_name("META-INF/mods.toml").map_err(|_| ())?;
        //     let mut content = String::new();
        //     file.read_to_string(&mut content).map_err(|_| ())?;
        //     content
        // };
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

        let icon = first_mod
            .get("logoFile")
            .and_then(|v| v.as_str())
            .and_then(|p| Self::extract_icon(archive, p));

        Ok(AddonMetadata {
            name,
            version,
            description,
            authors,
            icon,
        })
    }

    fn try_parse_forge_legacy(archive: &mut ZipArchive<File>) -> Result<AddonMetadata, ()> {
        // let json: serde_json::Value = {
        //     let mut file = archive.by_name("mcmod.info").map_err(|_| ())?;
        //     let mut content = String::new();
        //     file.read_to_string(&mut content).map_err(|_| ())?;
        //     serde_json::from_str(&content).map_err(|_| ())?
        // };
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

        let icon = mod_data["logoFile"]
            .as_str()
            .and_then(|p| Self::extract_icon(archive, p));

        Ok(AddonMetadata {
            name,
            version,
            description,
            authors,
            icon,
        })
    }

    fn extract_icon(archive: &mut ZipArchive<File>, path: &str) -> Option<Arc<String>> {
        let clean_path = path.trim_start_matches('/');
        let mut file = archive.by_name(clean_path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;

        let mime_type = if clean_path.ends_with(".png") {
            "image/png"
        } else if clean_path.ends_with(".jpg") || clean_path.ends_with(".jpeg") {
            "image/jpeg"
        } else if clean_path.ends_with(".svg") {
            "image/svg+xml"
        } else {
            "image/png"
        };

        Some(Arc::new(format!(
            "data:{};base64,{}",
            mime_type,
            general_purpose::STANDARD.encode(buffer)
        )))
    }
}
