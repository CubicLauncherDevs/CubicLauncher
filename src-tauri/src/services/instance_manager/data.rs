use crate::core::path_manager::PathManager;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::status::InstanceStatus;

pub(crate) const MAX_LEN: u8 = 24;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct InstanceData {
    pub name: Arc<str>,
    pub version: Arc<str>,
    pub last_played: u64,
    /// Segundos acumulados de juego (estilo Prism). Se suma al cerrar cada sesión.
    #[serde(default)]
    pub playtime_seconds: u64,
    pub min_memory: Option<u32>,
    pub max_memory: Option<u32>,
    pub cover_image: Option<PathBuf>,
    pub icon: Option<Arc<str>>,
    pub uuid: Arc<str>,
    pub overrides: Option<InstOverrides>,
    #[serde(default)]
    pub minecraft_jar: crate::services::minecraft_jar::MinecraftJarConfig,
    #[serde(default)]
    pub pinned: bool,
    #[serde(skip)]
    pub dirty: bool,
    #[serde(skip)]
    pub instance_root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstOverrides {
    pub java_version: Option<u8>,
    pub memory: Option<RamOverrides>,
    /// Argumentos JVM adicionales de esta instancia. Se añaden después de los
    /// globales, igual que el `JvmArgs` por instancia de MultiMC/Prism.
    #[serde(default)]
    pub jvm_args: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct RamOverrides {
    pub min_mem: u32, // MB
    pub max_mem: u32, // MB
}

impl InstanceData {
    pub fn new(name: String, version: String, icon: Option<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            last_played: 0,
            playtime_seconds: 0,
            min_memory: None,
            max_memory: None,
            cover_image: None,
            icon: icon.map(|s| s.into()),
            uuid: uuid::Uuid::new_v4().to_string().into(),
            overrides: None,
            minecraft_jar: Default::default(),
            pinned: false,
            dirty: true,
            instance_root: PathManager::get().get_instance_dir().to_path_buf(),
        }
    }
    pub fn get_loader(&self) -> &'static str {
        zellkern::Loader::from_version_id(&self.version).name()
    }

    pub fn get_instance_dir(&self) -> PathBuf {
        self.instance_root.join(self.name.as_ref())
    }

    pub async fn save(&mut self) -> Result<(), io::Error> {
        if !self.dirty {
            return Ok(());
        }
        validate_instance_name(&self.name).map_err(io::Error::other)?;
        zellkern::path_security::validate_version(&self.version)?;
        uuid::Uuid::parse_str(&self.uuid).map_err(io::Error::other)?;
        let content = serde_json::to_vec(self).map_err(io::Error::other)?;
        let root = zellkern::path_security::ConfinedDir::open(&self.instance_root)?;
        root.open_dir(Path::new(self.name.as_ref()))?
            .write(Path::new("instance.cub"), &content)?;
        self.dirty = false;
        Ok(())
    }

    pub async fn load(name: &str) -> Option<Self> {
        Self::load_from(&PathManager::get().get_instance_dir(), name).await
    }

    async fn load_from(root: &Path, name: &str) -> Option<Self> {
        validate_instance_name(name).ok()?;
        let content = zellkern::path_security::ConfinedDir::open(root)
            .ok()?
            .read(&Path::new(name).join("instance.cub"))
            .ok()?;
        let mut data: InstanceData = serde_json::from_slice(&content).ok()?;
        // The on-disk folder is authoritative. Never adopt a different path
        // from a file a modpack or another application may have replaced.
        if data.name.as_ref() != name {
            return None;
        }
        uuid::Uuid::parse_str(&data.uuid).ok()?;
        zellkern::path_security::validate_version(&data.version).ok()?;
        data.instance_root = root.to_path_buf();
        data.dirty = false;
        Some(data)
    }
}

#[derive(Serialize, Clone)]
pub struct InstanceDto {
    pub name: Arc<str>,
    pub version: Arc<str>,
    pub loader: Cow<'static, str>,
    pub last_played: u64,
    pub playtime_seconds: u64,
    pub status: InstanceStatus,
    pub cover_image: Option<PathBuf>,
    pub icon: Option<Arc<str>>,
    pub uuid: Arc<str>,
    pub path: PathBuf,
    pub overrides: Option<InstOverrides>,
    pub pinned: bool,
}

pub fn validate_instance_name(name: &str) -> Result<(), String> {
    zellkern::path_security::validate_component(name).map_err(|e| e.to_string())?;
    if name.is_empty() {
        return Err("El nombre de la instancia no puede estar vacío.".into());
    }
    if !name.is_ascii() {
        return Err("El nombre de la instancia debe contener solo caracteres ASCII.".into());
    }
    if name.len() > usize::from(MAX_LEN) {
        return Err(format!(
            "El nombre de la instancia no puede superar {} caracteres.",
            MAX_LEN
        ));
    }
    let forbidden = ['/', '\\', '\0', '<', '>', ':', '"', '|', '?', '*'];
    if name.contains("..") || name.chars().any(|c| forbidden.contains(&c)) {
        return Err(
            "El nombre contiene caracteres no permitidos (/, \\, <, >, :, \", |, ?, *, .., \\0)."
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/services/instance_manager/data.rs"]
mod tests;
