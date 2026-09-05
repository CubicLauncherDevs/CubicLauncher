//! Parseo de metadatos de instancias MultiMC / Prism Launcher.

use super::ImportError;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use tracing::warn;

/// UIDs de componentes reconocidos en `mmc-pack.json`.
const UID_MINECRAFT: &str = "net.minecraft";
const UID_FORGE: &str = "net.minecraftforge";
const UID_NEOFORGE: &str = "net.neoforged";
const UID_FABRIC: &str = "net.fabricmc.fabric-loader";
const UID_QUILT: &str = "org.quiltmc.quilt-loader";

#[derive(Debug, Deserialize)]
pub struct MmcPack {
    pub components: Vec<MmcComponent>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MmcComponent {
    pub uid: String,
    #[serde(default)]
    pub version: String,
    #[serde(alias = "cachedVersion", default)]
    pub cached_version: Option<String>,
}

impl MmcComponent {
    /// Devuelve la versión útil del componente.
    ///
    /// Algunos forks de MultiMC / Prism Launcher guardan la versión en
    /// `cachedVersion` en lugar de `version`, o dejan `version` vacía. Usamos
    /// `version` si tiene contenido; si no, recurrimos a `cachedVersion`.
    pub fn effective_version(&self) -> Option<&str> {
        if !self.version.is_empty() {
            return Some(&self.version);
        }
        self.cached_version.as_deref().filter(|v| !v.is_empty())
    }
}

#[derive(Debug, Clone)]
pub struct MultiMcInstanceMeta {
    pub original_name: String,
    pub sanitized_name: String,
    pub icon_key: Option<String>,
    pub min_memory: Option<u32>,
    pub max_memory: Option<u32>,
    pub game_version: Option<zellkern::GameVersion>,
    pub unsupported_loaders: Vec<String>,
}

impl MultiMcInstanceMeta {
    pub fn version_id(&self) -> Option<String> {
        self.game_version.as_ref().map(|gv| gv.to_version_id())
    }
}

pub fn parse_multimc_instance(path: &Path) -> Result<MultiMcInstanceMeta, ImportError> {
    let cfg_path = path.join("instance.cfg");
    let cfg_content =
        std::fs::read_to_string(&cfg_path).map_err(|e| ImportError::ProviderError {
            provider: "MultiMC".into(),
            message: format!("No se pudo leer instance.cfg: {e}"),
        })?;

    let cfg = parse_ini(&cfg_content);
    let original_name = cfg
        .general
        .get("name")
        .cloned()
        .unwrap_or_else(|| "Imported".to_string());
    let sanitized_name = crate::services::instance_import::sanitize_instance_name(&original_name);

    let icon_key = cfg.general.get("iconKey").cloned();
    let min_memory = cfg
        .general
        .get("MinMemAlloc")
        .and_then(|v| v.parse::<u32>().ok());
    let max_memory = cfg
        .general
        .get("MaxMemAlloc")
        .and_then(|v| v.parse::<u32>().ok());

    let (game_version, unsupported_loaders) = read_mmc_pack(path)
        .map(|pack| resolve_game_version(&pack))
        .unwrap_or((None, Vec::new()));

    Ok(MultiMcInstanceMeta {
        original_name,
        sanitized_name,
        icon_key,
        min_memory,
        max_memory,
        game_version,
        unsupported_loaders,
    })
}

pub fn read_mmc_pack(path: &Path) -> Option<MmcPack> {
    let pack_path = path.join("mmc-pack.json");
    let content = std::fs::read_to_string(&pack_path).ok()?;
    serde_json::from_str(&content)
        .inspect_err(|e| warn!("mmc-pack.json inválido en {:?}: {}", pack_path, e))
        .ok()
}

pub fn resolve_game_version(pack: &MmcPack) -> (Option<zellkern::GameVersion>, Vec<String>) {
    let mut mc_version: Option<String> = None;
    let mut loader: Option<zellkern::Loader> = None;
    let mut unsupported = Vec::new();

    for component in &pack.components {
        let Some(version) = component.effective_version() else {
            tracing::debug!(
                "Ignorando componente MMC sin versión usable: uid={}",
                component.uid
            );
            continue;
        };

        tracing::debug!(
            "Componente MMC detectado: uid={}, version={}",
            component.uid,
            version
        );

        match component.uid.as_str() {
            UID_MINECRAFT => mc_version = Some(version.to_string()),
            UID_FABRIC => {
                loader = Some(zellkern::Loader::Fabric(version.to_string()));
            }
            UID_QUILT => {
                loader = Some(zellkern::Loader::Quilt(version.to_string()));
            }
            UID_FORGE => {
                loader = Some(zellkern::Loader::Forge(version.to_string()));
            }
            UID_NEOFORGE => {
                loader = Some(zellkern::Loader::NeoForge(version.to_string()));
            }
            "org.lwjgl" | "org.lwjgl3" => {}
            _ => {
                if component.uid.contains("liteloader")
                    || component.uid.contains("optifine")
                    || component.uid.contains("modloader")
                {
                    unsupported.push(format!("{} {}", component.uid, version));
                }
            }
        }
    }

    let game_version = mc_version.map(|mc| {
        let detected_loader = loader.clone().unwrap_or(zellkern::Loader::Vanilla);
        tracing::info!(
            "Versión de juego resuelta: {} con loader {:?}",
            mc,
            detected_loader
        );
        zellkern::GameVersion {
            mc_version: mc,
            loader: detected_loader,
        }
    });

    (game_version, unsupported)
}

#[derive(Debug, Default)]
pub struct IniFile {
    pub general: HashMap<String, String>,
}

pub fn parse_ini(content: &str) -> IniFile {
    let mut ini = IniFile::default();
    let mut current_section = &mut ini.general;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(section_name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            let _ = section_name;
            current_section = &mut ini.general;
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            current_section.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    ini
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::instance_import::sanitize_instance_name;

    #[test]
    fn test_sanitize_instance_name() {
        assert_eq!(sanitize_instance_name("Mi Instancia"), "Mi Instancia");
        assert_eq!(sanitize_instance_name("ñoña"), "oa");
        assert_eq!(sanitize_instance_name("a/b"), "ab");
        assert_eq!(sanitize_instance_name(".."), "Imported");
        assert_eq!(sanitize_instance_name("<invalid>"), "invalid");
    }

    #[test]
    fn test_resolve_game_version_fabric() {
        let pack = MmcPack {
            components: vec![
                MmcComponent {
                    uid: UID_MINECRAFT.into(),
                    version: "1.20.1".into(),
                    cached_version: None,
                },
                MmcComponent {
                    uid: UID_FABRIC.into(),
                    version: "0.15.11".into(),
                    cached_version: None,
                },
            ],
        };
        let (gv, unsupported) = resolve_game_version(&pack);
        assert!(unsupported.is_empty());
        let gv = gv.unwrap();
        assert_eq!(gv.to_version_id(), "fabric-loader-0.15.11-1.20.1");
    }

    #[test]
    fn test_resolve_game_version_forge() {
        let pack = MmcPack {
            components: vec![
                MmcComponent {
                    uid: UID_MINECRAFT.into(),
                    version: "1.20.1".into(),
                    cached_version: None,
                },
                MmcComponent {
                    uid: UID_FORGE.into(),
                    version: "47.2.0".into(),
                    cached_version: None,
                },
            ],
        };
        let (gv, unsupported) = resolve_game_version(&pack);
        assert!(unsupported.is_empty());
        let gv = gv.unwrap();
        assert_eq!(gv.to_version_id(), "1.20.1-forge-47.2.0");
    }

    #[test]
    fn test_resolve_game_version_fabric_from_cached_version() {
        let pack = MmcPack {
            components: vec![
                MmcComponent {
                    uid: UID_MINECRAFT.into(),
                    version: String::new(),
                    cached_version: Some("1.20.1".into()),
                },
                MmcComponent {
                    uid: UID_FABRIC.into(),
                    version: String::new(),
                    cached_version: Some("0.15.11".into()),
                },
            ],
        };
        let (gv, unsupported) = resolve_game_version(&pack);
        assert!(unsupported.is_empty());
        let gv = gv.unwrap();
        assert_eq!(gv.to_version_id(), "fabric-loader-0.15.11-1.20.1");
    }

    #[test]
    fn test_resolve_game_version_with_extra_components() {
        let pack = MmcPack {
            components: vec![
                MmcComponent {
                    uid: UID_MINECRAFT.into(),
                    version: "1.21".into(),
                    cached_version: None,
                },
                MmcComponent {
                    uid: "org.lwjgl3".into(),
                    version: "3.3.2".into(),
                    cached_version: None,
                },
                MmcComponent {
                    uid: UID_NEOFORGE.into(),
                    version: "21.0.0".into(),
                    cached_version: None,
                },
            ],
        };
        let (gv, unsupported) = resolve_game_version(&pack);
        assert!(unsupported.is_empty());
        let gv = gv.unwrap();
        assert_eq!(gv.to_version_id(), "1.21-neoforge-21.0.0");
    }

    #[test]
    fn test_resolve_game_version_empty_version_is_skipped() {
        let pack = MmcPack {
            components: vec![
                MmcComponent {
                    uid: UID_MINECRAFT.into(),
                    version: String::new(),
                    cached_version: None,
                },
                MmcComponent {
                    uid: UID_FABRIC.into(),
                    version: "0.15.11".into(),
                    cached_version: None,
                },
            ],
        };
        let (gv, unsupported) = resolve_game_version(&pack);
        assert!(unsupported.is_empty());
        // Sin versión de Minecraft no se debe fabricar un GameVersion.
        assert!(gv.is_none());
    }

    #[test]
    fn test_resolve_game_version_quilt() {
        let pack = MmcPack {
            components: vec![
                MmcComponent {
                    uid: UID_MINECRAFT.into(),
                    version: "1.20.1".into(),
                    cached_version: None,
                },
                MmcComponent {
                    uid: UID_QUILT.into(),
                    version: "0.25.0".into(),
                    cached_version: None,
                },
            ],
        };
        let (gv, unsupported) = resolve_game_version(&pack);
        assert!(unsupported.is_empty());
        let gv = gv.unwrap();
        assert_eq!(gv.to_version_id(), "quilt-loader-0.25.0-1.20.1");
    }
}
