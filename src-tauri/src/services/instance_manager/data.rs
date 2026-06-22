use crate::core::path_manager::PathManager;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as tokio_fs;

use super::status::InstanceStatus;

pub(crate) const MAX_LEN: u8 = 16;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct InstanceData {
    pub name: Arc<str>,
    pub version: Arc<str>,
    pub last_played: u64,
    pub min_memory: Option<u32>,
    pub max_memory: Option<u32>,
    pub cover_image: Option<PathBuf>,
    pub icon: Option<Arc<str>>,
    pub uuid: Arc<str>,
    #[serde(skip)]
    pub dirty: bool,
}

impl InstanceData {
    pub fn new(name: String, version: String, icon: Option<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            last_played: 0,
            min_memory: None,
            max_memory: None,
            cover_image: None,
            icon: icon.map(|s| s.into()),
            uuid: uuid::Uuid::new_v4().to_string().into(),
            dirty: true,
        }
    }

    pub fn get_loader(&self) -> &'static str {
        zellkern::Loader::from_version_id(&self.version).name()
    }

    pub fn get_instance_dir(&self) -> PathBuf {
        PathManager::get()
            .get_instance_dir()
            .join(self.name.as_ref())
    }

    pub async fn save(&mut self) -> Result<(), io::Error> {
        if !self.dirty {
            return Ok(());
        }
        let dir = self.get_instance_dir();
        tokio_fs::create_dir_all(&dir).await?;
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
    pub status: InstanceStatus,
    pub cover_image: Option<PathBuf>,
    pub icon: Option<Arc<str>>,
    pub uuid: Arc<str>,
    pub path: PathBuf,
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
    if name.contains(&"..") || name.chars().any(|c| forbidden.contains(&c)) {
        return Err(
            "El nombre contiene caracteres no permitidos (/, \\, <, >, :, \", |, ?, *, .., \\0)."
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_empty() {
        assert!(validate_instance_name("").is_err());
    }

    #[test]
    fn test_validate_name_non_ascii() {
        assert!(validate_instance_name("ñoña").is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        assert!(validate_instance_name("a".repeat(usize::from(MAX_LEN) + 1).as_str()).is_err());
    }

    #[test]
    fn test_validate_name_with_slash() {
        assert!(validate_instance_name("a/b").is_err());
    }

    #[test]
    fn test_validate_name_with_backslash() {
        assert!(validate_instance_name("a\\b").is_err());
    }

    #[test]
    fn test_validate_name_with_null() {
        assert!(validate_instance_name("a\0b").is_err());
    }

    #[test]
    fn test_validate_name_with_dotdot() {
        assert!(validate_instance_name("..").is_err());
    }

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_instance_name("MyInstance").is_ok());
    }

    #[test]
    fn test_validate_name_max_length() {
        assert!(validate_instance_name("a".repeat(usize::from(MAX_LEN)).as_str()).is_ok());
    }

    #[test]
    fn test_get_loader_fabric() {
        let data = InstanceData::new("test".into(), "1.21-fabric".into(), None);
        assert_eq!(data.get_loader(), "Fabric");
    }

    #[test]
    fn test_get_loader_forge() {
        let data = InstanceData::new("test".into(), "1.20.1-forge".into(), None);
        assert_eq!(data.get_loader(), "Forge");
    }

    #[test]
    fn test_get_loader_quilt() {
        let data = InstanceData::new("test".into(), "1.19-quilt".into(), None);
        assert_eq!(data.get_loader(), "Quilt");
    }

    #[test]
    fn test_get_loader_vanilla() {
        let data = InstanceData::new("test".into(), "1.21".into(), None);
        assert_eq!(data.get_loader(), "Vanilla");
    }
}
