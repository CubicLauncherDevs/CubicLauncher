use crate::core::{AppEvent, InstanceError, emit};
use crate::services::{InstOverrides, SettingsManager};
use compact_str::ToCompactString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
}

impl InstanceHandle {
    pub(crate) fn new(data: InstanceData) -> Self {
        Self {
            uuid: data.uuid.clone(),
            data: Arc::new(RwLock::new(data)),
            status: Arc::new(AtomicStatus::new()),
            files_lock: Arc::new(tokio::sync::Mutex::new(())),
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

    /// Held across world operations, instance moves/deletion and launch admission.
    pub fn try_lock_files(&self) -> Result<tokio::sync::OwnedMutexGuard<()>, String> {
        self.files_lock
            .clone()
            .try_lock_owned()
            .map_err(|_| "Hay una operación de archivos en curso en esta instancia".to_string())
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
        if !self.data.read().await.dirty {
            return Ok(());
        }
        self.data.write().await.save().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn world_operation_lock_survives_cancelled_async_caller() {
        let handle = InstanceHandle::new(InstanceData::new("test".into(), "1.21".into(), None));
        let cloned = handle.clone();
        let guard = handle.try_lock_files().unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                drop(guard);
                done_tx.send(()).unwrap();
            })
            .await
            .unwrap();
        });
        started_rx.await.unwrap();
        assert!(cloned.try_lock_files().is_err());
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(handle.try_lock_files().is_err());
        release_tx.send(()).unwrap();
        done_rx.await.unwrap();
        assert!(handle.try_lock_files().is_ok());
    }
}
