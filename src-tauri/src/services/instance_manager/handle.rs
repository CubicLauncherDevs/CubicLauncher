use crate::core::{AppEvent, InstanceError, emit};
use crate::services::{InstOverrides, SettingsManager};
use compact_str::ToCompactString;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::data::{InstanceData, InstanceDto};
use super::manager::signal_kill;
use super::status::{AtomicStatus, InstanceStatus};

#[derive(Clone)]
pub struct InstanceHandle {
    pub uuid: Arc<str>,
    data: Arc<RwLock<InstanceData>>,
    status: Arc<AtomicStatus>,
}

impl InstanceHandle {
    pub(crate) fn new(data: InstanceData) -> Self {
        Self {
            uuid: data.uuid.clone(),
            data: Arc::new(RwLock::new(data)),
            status: Arc::new(AtomicStatus::new()),
        }
    }

    pub async fn load(name: &str) -> Option<Self> {
        let data = InstanceData::load(name).await?;
        Some(Self::new(data))
    }

    pub fn get_status(&self) -> InstanceStatus {
        self.status.get()
    }

    pub fn set_status(&self, status: InstanceStatus) {
        self.status.set(status);
        emit(AppEvent::InstanceEdited {
            id: self.uuid.to_compact_string(),
        });
    }

    pub fn is_busy(&self) -> bool {
        self.get_status().is_busy()
    }

    pub async fn kill(&self) -> Result<(), InstanceError> {
        signal_kill(&self.uuid);
        self.set_status(InstanceStatus::Off);
        Ok(())
    }

    pub async fn get_name(&self) -> Arc<str> {
        self.data.read().await.name.clone()
    }

    pub async fn get_version(&self) -> Arc<str> {
        self.data.read().await.version.clone()
    }

    pub async fn get_min_memory(&self) -> u32 {
        self.data
            .read()
            .await
            .min_memory
            .unwrap_or_else(|| SettingsManager::read().get_min_memory())
    }

    pub async fn get_max_memory(&self) -> u32 {
        self.data
            .read()
            .await
            .max_memory
            .unwrap_or_else(|| SettingsManager::read().get_max_memory())
    }

    pub async fn get_instance_dir(&self) -> PathBuf {
        self.data.read().await.get_instance_dir()
    }

    pub async fn get_cover_image(&self) -> Option<PathBuf> {
        self.data.read().await.cover_image.clone()
    }

    pub async fn get_icon(&self) -> Option<Arc<str>> {
        self.data.read().await.icon.clone()
    }

    pub async fn get_overrides(&self) -> Option<InstOverrides> {
        self.data.read().await.overrides.clone()
    }

    pub async fn to_dto(&self) -> InstanceDto {
        let data = self.data.read().await;
        InstanceDto {
            name: data.name.clone(),
            version: data.version.clone(),
            loader: std::borrow::Cow::Borrowed(data.get_loader()),
            last_played: data.last_played,
            status: self.get_status(),
            cover_image: data.cover_image.clone(),
            icon: data.icon.clone(),
            uuid: self.uuid.clone(),
            path: data.get_instance_dir(),
            overrides: data.overrides,
        }
    }

    pub async fn set_name(&self, name: String) {
        let mut data = self.data.write().await;
        data.name = name.into();
        data.dirty = true;
    }

    pub async fn set_overrides(&self, overrides: Option<InstOverrides>) {
        let mut data = self.data.write().await;
        data.overrides = overrides;
        data.dirty = true;
    }

    pub async fn set_version(&self, version: String) {
        let mut data = self.data.write().await;
        data.version = version.into();
        data.dirty = true;
    }

    pub async fn set_icon(&self, icon: Option<String>) {
        let mut data = self.data.write().await;
        data.icon = icon.map(|s| s.into());
        data.dirty = true;
    }

    pub async fn set_cover_image(&self, cover_image: Option<PathBuf>) {
        let mut data = self.data.write().await;
        data.cover_image = cover_image;
        data.dirty = true;
    }

    pub async fn update_last_played(&self) {
        let mut data = self.data.write().await;
        data.last_played = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        data.dirty = true;
    }

    pub async fn save_if_dirty(&self) -> Result<(), io::Error> {
        if !self.data.read().await.dirty {
            return Ok(());
        }
        self.data.write().await.save().await
    }
}
