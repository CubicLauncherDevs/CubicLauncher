use crate::core::path_manager::PathManager;
use crate::core::{FsError, InstanceError};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use tokio::fs as tokio_fs;
use tokio::sync::RwLock;
use tokio::sync::oneshot;
use tokio::time::{self, Duration};
use tracing::{error, info};

use super::data::{InstanceData, validate_instance_name};
use super::handle::InstanceHandle;

pub(crate) const SYNC_INTERVAL_SECS: u64 = 30;

pub struct InstanceManager {
    pub instances: RwLock<HashMap<String, InstanceHandle>>,
    _sync_handle: tokio::task::JoinHandle<()>,
}

static INSTANCE_MANAGER: OnceLock<Arc<InstanceManager>> = OnceLock::new();

static KILL_SENDERS: LazyLock<Mutex<HashMap<String, oneshot::Sender<()>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn register_kill_sender(uuid: &str, tx: oneshot::Sender<()>) {
    KILL_SENDERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(uuid.to_string(), tx);
}

pub fn unregister_kill_sender(uuid: &str) {
    KILL_SENDERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(uuid);
}

/// Envía la señal de kill. Retorna `true` si el proceso estaba corriendo.
pub fn signal_kill(uuid: &str) -> bool {
    let tx = KILL_SENDERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(uuid);
    tx.is_some_and(|tx| tx.send(()).is_ok())
}

impl InstanceManager {
    pub async fn init() -> Arc<Self> {
        let manager = Arc::new(Self {
            instances: RwLock::new(HashMap::new()),
            _sync_handle: tokio::spawn(Self::sync_task()),
        });

        let base_dir = PathManager::get().get_instance_dir().to_path_buf();
        let names = if let Ok(mut dir) = tokio::fs::read_dir(&base_dir).await {
            let mut names = Vec::new();
            while let Ok(Some(entry)) = dir.next_entry().await {
                if entry.path().is_dir() {
                    names.push(entry.file_name().to_string_lossy().to_string());
                }
            }
            names
        } else {
            Vec::new()
        };

        let handles: Vec<Option<InstanceHandle>> =
            futures::future::join_all(names.iter().map(|name| InstanceHandle::load(name))).await;

        let mut guard = manager.instances.write().await;
        for handle in handles.into_iter().flatten() {
            guard.insert(handle.uuid.to_string(), handle);
        }
        drop(guard);

        let _ = INSTANCE_MANAGER.set(manager.clone());
        manager
    }

    pub fn get() -> &'static Arc<InstanceManager> {
        INSTANCE_MANAGER
            .get()
            .expect("BUG: InstanceManager usado antes de inicializar")
    }

    async fn sync_task() {
        let mut interval = time::interval(Duration::from_secs(SYNC_INTERVAL_SECS));
        interval.tick().await;
        loop {
            interval.tick().await;
            info!("Ejecutando tarea de sincronizacion");

            let manager = match INSTANCE_MANAGER.get() {
                Some(m) => m.clone(),
                None => continue,
            };

            let handles: Vec<InstanceHandle> =
                { manager.instances.read().await.values().cloned().collect() };

            for handle in handles {
                if let Err(e) = handle.save_if_dirty().await {
                    error!("Error guardando instancia {}: {:?}", handle.uuid, e);
                }
            }
        }
    }

    pub async fn create_instance(
        &self,
        name: String,
        version: String,
        icon: Option<String>,
    ) -> Result<InstanceHandle, InstanceError> {
        validate_instance_name(&name).map_err(InstanceError::InstNameParse)?;

        let mut data = InstanceData::new(name, version, icon);
        if data.get_instance_dir().exists() {
            Err(InstanceError::AlreadyExists)?;
        }
        data.save().await.map_err(|e| {
            InstanceError::Fs(FsError::WriteFile {
                path: data
                    .get_instance_dir()
                    .join("instance.cub")
                    .to_string_lossy()
                    .to_string(),
                source: e,
            })
        })?;

        let handle = InstanceHandle::new(data);
        self.instances
            .write()
            .await
            .insert(handle.uuid.to_string(), handle.clone());

        Ok(handle)
    }

    pub async fn get_handle(&self, uuid: &str) -> Option<InstanceHandle> {
        self.instances.read().await.get(uuid).cloned()
    }

    pub async fn get_all_handles(&self) -> Vec<InstanceHandle> {
        self.instances.read().await.values().cloned().collect()
    }

    pub async fn count(&self) -> usize {
        self.instances.read().await.len()
    }

    pub async fn get_all_dtos(&self) -> Vec<super::data::InstanceDto> {
        let handles = self.get_all_handles().await;
        futures::future::join_all(handles.iter().map(|h| h.to_dto())).await
    }

    pub async fn get_running_ids(&self) -> Vec<String> {
        self.instances
            .read()
            .await
            .values()
            .filter(|h| h.is_busy())
            .map(|h| h.uuid.to_string())
            .collect()
    }

    pub async fn delete_instance(&self, uuid: &str) -> Result<(), String> {
        let handle = {
            self.instances
                .write()
                .await
                .remove(uuid)
                .ok_or_else(|| "Instancia no encontrada".to_string())?
        };

        let dir = handle.get_instance_dir().await;
        if dir.exists() {
            tokio_fs::remove_dir_all(&dir)
                .await
                .map_err(|e| format!("Error al eliminar el directorio: {}", e))?;
        }
        Ok(())
    }

    pub async fn update_instance(
        &self,
        uuid: &str,
        new_name: Option<String>,
        new_version: Option<String>,
        new_icon: Option<Option<String>>,
    ) -> Result<(), String> {
        let handle = self
            .get_handle(uuid)
            .await
            .ok_or_else(|| "Instancia no encontrada".to_string())?;

        if let Some(name) = new_name {
            validate_instance_name(&name)?;

            let old_name = handle.get_name().await;
            if *old_name != name {
                let base_dir = PathManager::get().get_instance_dir();
                let old_dir = base_dir.join(&*old_name);
                let new_dir = base_dir.join(&name);

                if new_dir.exists() {
                    return Err("Ya existe una instancia con ese nombre".to_string());
                }
                if old_dir.exists() {
                    tokio_fs::rename(&old_dir, &new_dir)
                        .await
                        .map_err(|e| format!("Error al renombrar el directorio: {}", e))?;
                }
                handle.set_name(name).await;
            }
        }

        if let Some(version) = new_version {
            handle.set_version(version).await;
        }

        if let Some(icon) = new_icon {
            handle.set_icon(icon).await;
        }

        handle
            .save_if_dirty()
            .await
            .map_err(|e| format!("Error al guardar la instancia: {}", e))?;

        Ok(())
    }
}
