//! Provider para instancias exportadas por MultiMC, Prism Launcher y forks compatibles.

use super::InstanceImporter;
use super::types::{ImportError, InstanceImportPlan, sanitize_instance_name};
use crate::core::{AppEvent, emit};
use crate::services::instance_manager::data::RamOverrides;
use crate::services::{DownloadQueue, InstOverrides, InstanceDto, InstanceManager};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// UIDs de componentes reconocidos en `mmc-pack.json`.
const UID_MINECRAFT: &str = "net.minecraft";
const UID_FORGE: &str = "net.minecraftforge";
const UID_NEOFORGE: &str = "net.neoforged";
const UID_FABRIC: &str = "net.fabricmc.fabric-loader";
const UID_QUILT: &str = "org.quiltmc.quilt-loader";

/// Datos de Minecraft que migraremos desde el directorio de juego.
const FOLDERS_TO_MIGRATE: &[&str] = &[
    "mods",
    "resourcepacks",
    "shaderpacks",
    "saves",
    "config",
    "scripts",
    "defaultconfigs",
    "kubejs",
    "options.txt",
    "servers.dat",
];

/// Provider para ZIPs de MultiMC / Prism Launcher.
pub struct MultimcProvider;

impl InstanceImporter for MultimcProvider {
    fn id(&self) -> &'static str {
        "multimc"
    }

    fn display_name(&self) -> &'static str {
        "MultiMC / Prism"
    }

    fn detect(&self, preview_dir: &Path) -> bool {
        preview_dir.join("instance.cfg").is_file()
    }

    fn preview(&self, preview_dir: &Path) -> Result<InstanceImportPlan, ImportError> {
        let meta = parse_multimc_instance(preview_dir)?;

        Ok(InstanceImportPlan {
            format_id: self.id(),
            format_name: self.display_name(),
            archive_path: PathBuf::new(),
            preview_dir: preview_dir.to_path_buf(),
            original_name: meta.original_name.clone(),
            sanitized_name: meta.sanitized_name.clone(),
            minecraft_version: meta.game_version.as_ref().map(|gv| gv.mc_version.clone()),
            loader: meta
                .game_version
                .as_ref()
                .map(|gv| gv.loader.name().to_string()),
            loader_version: meta
                .game_version
                .as_ref()
                .and_then(|gv| gv.loader.version().map(|s| s.to_string())),
            warnings: build_warnings(&meta),
        })
    }

    fn import<'a>(
        &'a self,
        preview_dir: &'a Path,
        target_name: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<InstanceDto, ImportError>> + Send + 'a>,
    > {
        Box::pin(import_multimc_instance(preview_dir, target_name))
    }
}

#[derive(Debug, Deserialize)]
struct MmcPack {
    components: Vec<MmcComponent>,
}

#[derive(Debug, Deserialize, Clone)]
struct MmcComponent {
    uid: String,
    version: String,
}

#[derive(Debug, Clone)]
struct MultiMcInstanceMeta {
    original_name: String,
    sanitized_name: String,
    icon_key: Option<String>,
    min_memory: Option<u32>,
    max_memory: Option<u32>,
    game_version: Option<zellkern::GameVersion>,
    unsupported_loaders: Vec<String>,
}

impl MultiMcInstanceMeta {
    fn version_id(&self) -> Option<String> {
        self.game_version.as_ref().map(|gv| gv.to_version_id())
    }
}

fn build_warnings(meta: &MultiMcInstanceMeta) -> Vec<String> {
    let mut warnings = Vec::new();
    if !meta.unsupported_loaders.is_empty() {
        warnings.push(format!(
            "Loaders no soportados detectados: {}. Se importará como Vanilla + archivos.",
            meta.unsupported_loaders.join(", ")
        ));
    }
    warnings
}

fn parse_multimc_instance(path: &Path) -> Result<MultiMcInstanceMeta, ImportError> {
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
    let sanitized_name = sanitize_instance_name(&original_name);

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

fn read_mmc_pack(path: &Path) -> Option<MmcPack> {
    let pack_path = path.join("mmc-pack.json");
    let content = std::fs::read_to_string(&pack_path).ok()?;
    serde_json::from_str(&content)
        .inspect_err(|e| warn!("mmc-pack.json inválido en {:?}: {}", pack_path, e))
        .ok()
}

fn resolve_game_version(pack: &MmcPack) -> (Option<zellkern::GameVersion>, Vec<String>) {
    let mut mc_version: Option<String> = None;
    let mut loader: Option<zellkern::Loader> = None;
    let mut unsupported = Vec::new();

    for component in &pack.components {
        match component.uid.as_str() {
            UID_MINECRAFT => mc_version = Some(component.version.clone()),
            UID_FABRIC => {
                loader = Some(zellkern::Loader::Fabric(component.version.clone()));
            }
            UID_QUILT => {
                loader = Some(zellkern::Loader::Quilt(component.version.clone()));
            }
            UID_FORGE => {
                loader = Some(zellkern::Loader::Forge(component.version.clone()));
            }
            UID_NEOFORGE => {
                loader = Some(zellkern::Loader::NeoForge(component.version.clone()));
            }
            "org.lwjgl" | "org.lwjgl3" => {}
            _ => {
                if component.uid.contains("liteloader")
                    || component.uid.contains("optifine")
                    || component.uid.contains("modloader")
                {
                    unsupported.push(format!("{} {}", component.uid, component.version));
                }
            }
        }
    }

    let game_version = mc_version.map(|mc| zellkern::GameVersion {
        mc_version: mc,
        loader: loader.unwrap_or(zellkern::Loader::Vanilla),
    });

    (game_version, unsupported)
}

async fn import_multimc_instance(
    preview_dir: &Path,
    target_name: &str,
) -> Result<InstanceDto, ImportError> {
    let meta = parse_multimc_instance(preview_dir)?;
    let final_name = if target_name.trim().is_empty() {
        meta.sanitized_name.clone()
    } else {
        target_name.to_string()
    };

    info!(
        "Importando instancia MultiMC/Prism desde {:?} como '{}'",
        preview_dir, final_name
    );

    let version_id = meta
        .version_id()
        .ok_or(ImportError::UnknownMinecraftVersion)?;

    let handle = InstanceManager::get()
        .create_instance(final_name, version_id.clone(), None)
        .await
        .map_err(|e| match e {
            crate::core::errors::instance::InstanceError::AlreadyExists => {
                ImportError::ProviderError {
                    provider: "MultiMC".into(),
                    message: "Ya existe una instancia con ese nombre".into(),
                }
            }
            other => ImportError::Instance(other),
        })?;

    let instance_dir = handle.get_instance_dir().await;
    let source_game_dir = resolve_game_dir(preview_dir);

    if source_game_dir.exists() {
        migrate_game_data(&source_game_dir, &instance_dir).await?;
    }

    if let Some(icon_path) = resolve_icon_path(preview_dir, &meta.icon_key) {
        let dest_icon = instance_dir.join("icon.png");
        if let Err(e) = tokio::fs::copy(&icon_path, &dest_icon).await {
            warn!("No se pudo copiar el icono {:?}: {}", icon_path, e);
        } else {
            handle
                .set_icon(Some(dest_icon.to_string_lossy().to_string()))
                .await;
        }
    }

    if meta.min_memory.is_some() || meta.max_memory.is_some() {
        let overrides = InstOverrides {
            java_version: None,
            memory: Some(RamOverrides {
                min_mem: meta.min_memory.unwrap_or(512),
                max_mem: meta.max_memory.unwrap_or(2048),
            }),
        };
        handle.set_overrides(Some(overrides)).await;
    }

    handle
        .save_if_dirty()
        .await
        .map_err(|e| ImportError::ProviderError {
            provider: "MultiMC".into(),
            message: format!("Error guardando la instancia importada: {e}"),
        })?;

    DownloadQueue::get().enqueue(version_id.clone()).await;

    emit(AppEvent::InstanceCreated {
        id: handle.uuid.to_string().into(),
        dto: handle.to_dto().await,
    });

    info!("Instancia importada exitosamente: uuid={}", handle.uuid);
    Ok(handle.to_dto().await)
}

fn resolve_game_dir(instance_path: &Path) -> PathBuf {
    let dot_minecraft = instance_path.join(".minecraft");
    if dot_minecraft.exists() {
        dot_minecraft
    } else {
        instance_path.join("minecraft")
    }
}

async fn migrate_game_data(source: &Path, target: &Path) -> Result<(), ImportError> {
    for relative in FOLDERS_TO_MIGRATE {
        let src = source.join(relative);
        if !src.exists() {
            continue;
        }
        let dst = target.join(relative);

        if src.is_dir() {
            copy_dir_recursively(&src, &dst).await?;
        } else {
            if let Some(parent) = dst.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
					.map_err(ImportError::Io)?;
            }
            tokio::fs::copy(&src, &dst)
                .await
                .map_err(|e| ImportError::ProviderError {
                    provider: "MultiMC".into(),
                    message: format!("Error copiando {:?} a {:?}: {e}", src, dst),
                })?;
        }
    }
    Ok(())
}

async fn copy_dir_recursively(src: &Path, dst: &Path) -> Result<(), ImportError> {
    tokio::fs::create_dir_all(dst)
        .await
        .map_err(|e| ImportError::ProviderError {
            provider: "MultiMC".into(),
            message: format!("Error creando directorio {:?}: {e}", dst),
        })?;

    let mut entries = tokio::fs::read_dir(src)
        .await
        .map_err(|e| ImportError::ProviderError {
            provider: "MultiMC".into(),
            message: format!("Error leyendo directorio {:?}: {e}", src),
        })?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let dest = dst.join(entry.file_name());

        if path.is_dir() {
            Box::pin(copy_dir_recursively(&path, &dest)).await?;
        } else {
            tokio::fs::copy(&path, &dest)
                .await
                .map_err(|e| ImportError::ProviderError {
                    provider: "MultiMC".into(),
                    message: format!("Error copiando {:?} a {:?}: {e}", path, dest),
                })?;
        }
    }

    Ok(())
}

fn resolve_icon_path(instance_path: &Path, icon_key: &Option<String>) -> Option<PathBuf> {
    let game_icon = resolve_game_dir(instance_path).join("icon.png");
    if game_icon.is_file() {
        return Some(game_icon);
    }

    if let Some(key) = icon_key
        && key != "default"
        && key != "logo"
    {
        let custom = instance_path.join(format!("{}.png", key));
        if custom.is_file() {
            return Some(custom);
        }
    }

    None
}

#[derive(Debug, Default)]
struct IniFile {
    general: HashMap<String, String>,
}

fn parse_ini(content: &str) -> IniFile {
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
                },
                MmcComponent {
                    uid: UID_FABRIC.into(),
                    version: "0.15.11".into(),
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
                },
                MmcComponent {
                    uid: UID_FORGE.into(),
                    version: "47.2.0".into(),
                },
            ],
        };
        let (gv, unsupported) = resolve_game_version(&pack);
        assert!(unsupported.is_empty());
        let gv = gv.unwrap();
        assert_eq!(gv.to_version_id(), "1.20.1-forge-47.2.0");
    }
}
