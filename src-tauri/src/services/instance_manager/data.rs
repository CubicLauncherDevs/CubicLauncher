use crate::core::path_manager::PathManager;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as tokio_fs;

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

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct InstOverrides {
    pub java_version: Option<u8>,
    pub memory: Option<RamOverrides>,
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
        let dir = self.get_instance_dir();
        let content = serde_json::to_string(self).map_err(io::Error::other)?;
        tokio_fs::write(dir.join("instance.cub"), content).await?;
        self.dirty = false;
        Ok(())
    }

    pub async fn load(name: &str) -> Option<Self> {
        let path = PathManager::get()
            .get_instance_dir()
            .join(name)
            .join("instance.cub");
        let content = tokio_fs::read_to_string(path).await.ok()?;
        let mut data: InstanceData = serde_json::from_str(&content).ok()?;
        data.instance_root = PathManager::get().get_instance_dir().to_path_buf();
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
