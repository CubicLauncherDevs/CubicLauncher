use crate::core::path_manager::PathManager;
use crate::core::{FsError, InstanceError};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use tokio::fs as tokio_fs;
use tokio::sync::RwLock;
use tokio::sync::oneshot;
use tokio::time::{self, Duration};
use tracing::{error, info, warn};

use super::data::{InstOverrides, InstanceData, TagData, validate_instance_name};
use super::handle::InstanceHandle;

pub(crate) const SYNC_INTERVAL_SECS: u64 = 30;

pub struct InstanceManager {
    pub instances: RwLock<HashMap<String, InstanceHandle>>,
    pub tags: RwLock<HashMap<String, TagData>>,
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
            tags: RwLock::new(HashMap::new()),
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

        let _ = Self::load_tags(&manager).await;
        let _ = INSTANCE_MANAGER.set(manager.clone());
        manager
    }

    async fn load_tags(manager: &Arc<Self>) {
        let path = PathManager::get().get_tags_dir().join("tags.json");
        let content = match tokio_fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(_) => return,
        };
        let tags: Vec<TagData> = match serde_json::from_str(&content) {
            Ok(t) => t,
            Err(e) => {
                warn!("Error parsing tags.json: {}", e);
                return;
            }
        };
        let mut guard = manager.tags.write().await;
        for tag in tags {
            guard.insert(tag.id.to_string(), tag);
        }
    }

    pub async fn save_tags(&self) {
        let path = PathManager::get().get_tags_dir().join("tags.json");
        let tags = self.tags.read().await;
        let vec: Vec<&TagData> = tags.values().collect();
        if let Ok(content) = serde_json::to_string(&vec) {
            if let Err(e) = tokio_fs::write(&path, content).await {
                error!("Error saving tags.json: {}", e);
            }
        }
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
        tag_ids: Vec<String>,
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
        if !tag_ids.is_empty() {
            handle.set_tags(tag_ids).await;
            let inst_dir = handle.get_instance_dir().await;
            handle.save_if_dirty().await.map_err(|e| {
                InstanceError::Fs(FsError::WriteFile {
                    path: inst_dir.join("instance.cub").to_string_lossy().to_string(),
                    source: e,
                })
            })?;
        }
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
        new_overrides: Option<InstOverrides>,
    ) -> Result<(), String> {
        let handle = self
            .get_handle(uuid)
            .await
            .ok_or_else(|| "Instancia no encontrada".to_string())?;

        if handle.is_busy() {
            return Err(
                "No se puede modificar una instancia mientras está en ejecución".to_string(),
            );
        }

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

        handle.set_overrides(new_overrides).await;

        handle
            .save_if_dirty()
            .await
            .map_err(|e| format!("Error al guardar la instancia: {}", e))?;

        Ok(())
    }

    // ─── Tag management ─────────────────────────────────────────────────────

    pub async fn get_all_tags(&self) -> Vec<TagData> {
        self.tags.read().await.values().cloned().collect()
    }

    pub async fn create_tag(
        &self,
        name: String,
        color: Option<String>,
    ) -> Result<TagData, String> {
        let mut guard = self.tags.write().await;
        // check unique name (case-insensitive)
        let name_lower = name.to_lowercase();
        if guard.values().any(|t| t.name.to_lowercase() == name_lower) {
            return Err("Ya existe una etiqueta con ese nombre".into());
        }
        let max_order = guard.values().map(|t| t.order).max().unwrap_or(0);
        let tag = TagData {
            id: uuid::Uuid::new_v4().to_string().into(),
            name: name.into(),
            color: color.map(|c| c.into()),
            order: max_order + 1,
        };
        let id = tag.id.to_string();
        guard.insert(id, tag.clone());
        drop(guard);
        self.save_tags().await;
        Ok(tag)
    }

    pub async fn update_tag(
        &self,
        id: String,
        name: Option<String>,
        color: Option<Option<String>>,
        order: Option<u32>,
    ) -> Result<TagData, String> {
        if let Some(ref n) = name {
            let n_lower = n.to_lowercase();
            let guard = self.tags.read().await;
            if guard.values().any(|t| t.id.as_ref() != id && t.name.to_lowercase() == n_lower) {
                return Err("Ya existe una etiqueta con ese nombre".into());
            }
        }
        let mut guard = self.tags.write().await;
        let tag = guard.get_mut(&id).ok_or_else(|| "Etiqueta no encontrada".to_string())?;
        if let Some(n) = name {
            tag.name = n.into();
        }
        if let Some(c) = color {
            tag.color = c.map(|c| c.into());
        }
        if let Some(o) = order {
            tag.order = o;
        }
        let result = tag.clone();
        drop(guard);
        self.save_tags().await;
        Ok(result)
    }

    pub async fn delete_tag(&self, id: &str) -> Result<(), String> {
        {
            let mut guard = self.tags.write().await;
            guard.remove(id).ok_or_else(|| "Etiqueta no encontrada".to_string())?;
        }
        self.save_tags().await;
        // remove tag from all instances
        let handles = self.get_all_handles().await;
        for handle in handles {
            let tags = handle.get_tags().await;
            if tags.iter().any(|t| t.as_ref() == id) {
                let new_tags: Vec<String> = tags.iter().filter(|t| t.as_ref() != id).map(|t| t.to_string()).collect();
                handle.set_tags(new_tags).await;
            }
        }
        Ok(())
    }

    pub async fn set_instance_tags(&self, uuid: &str, tag_ids: Vec<String>) -> Result<(), String> {
        let handle = self
            .get_handle(uuid)
            .await
            .ok_or_else(|| "Instancia no encontrada".to_string())?;
        handle.set_tags(tag_ids).await;
        handle
            .save_if_dirty()
            .await
            .map_err(|e| format!("Error al guardar la instancia: {}", e))?;
        Ok(())
    }
}
