use crate::core::{AppEvent, InstanceError, emit};
use crate::services::{InstOverrides, SettingsManager};
use compact_str::ToCompactString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

use super::data::{InstanceData, InstanceDto};

use super::manager::signal_kill;
use super::status::{AtomicStatus, InstanceStatus};

fn is_asset_icon(icon: &str) -> bool {
    icon.starts_with("/images/")
}

fn normalize_icon_path(icon: &str, instance_dir: &Path) -> Arc<str> {
    if is_asset_icon(icon) {
        return icon.into();
    }
    let path = Path::new(icon);
    if path.is_absolute()
        && let Ok(rel) = path.strip_prefix(instance_dir)
    {
        return rel.to_string_lossy().to_string().into();
    }
    icon.into()
}

#[derive(Clone)]
pub struct InstanceHandle {
    pub uuid: Arc<str>,
    data: Arc<RwLock<InstanceData>>,
    status: Arc<AtomicStatus>,
    files_lock: Arc<tokio::sync::Mutex<()>>,
    deleted: Arc<AtomicBool>,
}

/// Admission to an instance's filesystem. Clones share deletion state and the lock.
pub struct InstanceFilesGuard {
    handle: InstanceHandle,
    _lock: tokio::sync::OwnedMutexGuard<()>,
}

impl InstanceFilesGuard {
    pub async fn save_if_dirty(&self) -> Result<(), io::Error> {
        self.handle.ensure_alive()?;
        self.handle.data.write().await.save().await
    }

    pub(super) fn mark_deleted(&self) {
        self.handle.deleted.store(true, Ordering::Release);
    }
}

impl InstanceHandle {
    pub(crate) fn new(data: InstanceData) -> Self {
        Self {
            uuid: data.uuid.clone(),
            data: Arc::new(RwLock::new(data)),
            status: Arc::new(AtomicStatus::new()),
            files_lock: Arc::new(tokio::sync::Mutex::new(())),
            deleted: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn load(name: &str) -> Option<Self> {
        let mut data = InstanceData::load(name).await?;

        // Migrate legacy absolute custom icon paths to instance-relative paths.
        if let Some(icon) = data.icon.as_ref() {
            let icon_str = icon.as_ref();
            if !is_asset_icon(icon_str) {
                let instance_dir = data.get_instance_dir();
                let path = Path::new(icon_str);
                if path.is_absolute()
                    && let Ok(rel) = path.strip_prefix(&instance_dir)
                {
                    data.icon = Some(rel.to_string_lossy().to_string().into());
                    data.dirty = true;
                }
            }
        }

        Some(Self::new(data))
    }

    pub fn get_status(&self) -> InstanceStatus {
        self.status.get()
    }

    pub fn set_status(&self, status: InstanceStatus) {
        self.status.set(status);
        emit(AppEvent::InstanceEdited {
            id: self.uuid.to_compact_string(),
            dto: None,
        });
    }

    pub fn is_busy(&self) -> bool {
        self.get_status().is_busy()
    }

    /// Held across world/server operations, instance moves/deletion and launch admission.
    pub fn try_lock_files(&self) -> Result<InstanceFilesGuard, String> {
        let lock =
            self.files_lock.clone().try_lock_owned().map_err(|_| {
                "Hay una operación de archivos en curso en esta instancia".to_string()
            })?;
        self.ensure_alive().map_err(|e| e.to_string())?;
        Ok(InstanceFilesGuard {
            handle: self.clone(),
            _lock: lock,
        })
    }

    fn ensure_alive(&self) -> Result<(), io::Error> {
        if self.deleted.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "La instancia fue eliminada",
            ));
        }
        Ok(())
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
        self.data.read().await.overrides
    }

    pub async fn get_pinned(&self) -> bool {
        self.data.read().await.pinned
    }

    pub async fn get_minecraft_jar(&self) -> crate::services::minecraft_jar::MinecraftJarConfig {
        self.data.read().await.minecraft_jar.clone()
    }

    /// Caller holds the instance files guard. Roll back memory on save failure.
    pub async fn save_minecraft_jar(
        &self,
        config: crate::services::minecraft_jar::MinecraftJarConfig,
    ) -> Result<(), String> {
        config.validate()?;
        let mut data = self.data.write().await;
        let old = std::mem::replace(&mut data.minecraft_jar, config);
        data.dirty = true;
        if let Err(e) = data.save().await {
            data.minecraft_jar = old;
            return Err(e.to_string());
        }
        Ok(())
    }

    pub async fn set_pinned(&self, pinned: bool) {
        let mut data = self.data.write().await;
        data.pinned = pinned;
        data.dirty = true;
    }

    pub async fn get_icon_absolute(&self) -> Option<Arc<str>> {
        let data = self.data.read().await;
        Self::resolve_icon_absolute(&data)
    }

    fn resolve_icon_absolute(data: &InstanceData) -> Option<Arc<str>> {
        let icon = data.icon.as_ref()?;
        let icon_str = icon.as_ref();
        if is_asset_icon(icon_str) {
            return Some(icon.clone());
        }
        Some(
            data.get_instance_dir()
                .join(icon_str)
                .to_string_lossy()
                .to_string()
                .into(),
        )
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
            icon: Self::resolve_icon_absolute(&data),
            uuid: self.uuid.clone(),
            path: data.get_instance_dir(),
            overrides: data.overrides,
            pinned: data.pinned,
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
        let instance_dir = data.get_instance_dir();
        data.icon = icon.map(|s| normalize_icon_path(&s, &instance_dir));
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
        let lock = self.files_lock.clone().lock_owned().await;
        let guard = InstanceFilesGuard {
            handle: self.clone(),
            _lock: lock,
        };
        guard.save_if_dirty().await
    }
}

#[cfg(test)]
#[path = "../../tests/services/instance_manager/handle.rs"]
mod tests;
