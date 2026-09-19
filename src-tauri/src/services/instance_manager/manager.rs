use crate::core::path_manager::PathManager;
use crate::core::{AppEvent, FsError, InstanceError, emit};
use crate::services::launcher::remove_log_ring;
use compact_str::ToCompactString;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use tokio::fs as tokio_fs;
use tokio::sync::RwLock;
use tokio::sync::oneshot;
use tokio::time::{self, Duration};
use tracing::{error, info};

use super::data::{InstOverrides, InstanceData, validate_instance_name};
use super::handle::InstanceHandle;

pub(crate) const SYNC_INTERVAL_SECS: u64 = 30;
// Longer than a valid instance name, and inside the root to stay on its filesystem.
const DELETION_DIR: &str = ".cubic-pending-instance-deletions";

pub struct InstanceManager {
    instances: Arc<RwLock<HashMap<String, InstanceHandle>>>,
    instance_dir: PathBuf,
    _sync_handle: tokio::task::JoinHandle<()>,
}

static INSTANCE_MANAGER: OnceLock<Arc<InstanceManager>> = OnceLock::new();

struct KillSender {
    tx: oneshot::Sender<()>,
    requested: Arc<AtomicBool>,
}

static KILL_SENDERS: LazyLock<Mutex<HashMap<String, KillSender>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn register_kill_sender(uuid: &str, tx: oneshot::Sender<()>) -> Arc<AtomicBool> {
    let requested = Arc::new(AtomicBool::new(false));
    KILL_SENDERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(
            uuid.to_string(),
            KillSender {
                tx,
                requested: requested.clone(),
            },
        );
    requested
}

pub fn unregister_kill_sender(uuid: &str) {
    KILL_SENDERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(uuid);
}

/// Envía la señal de kill. Retorna `true` si el proceso estaba corriendo.
pub fn signal_kill(uuid: &str) -> bool {
    let mut senders = KILL_SENDERS.lock().unwrap_or_else(|e| e.into_inner());
    let Some(sender) = senders.remove(uuid) else {
        return false;
    };
    // Publish the intent before exit cleanup can unregister this execution,
    // even if wait() wins the race against receiving the kill signal.
    sender.requested.store(true, Ordering::Release);
    drop(senders);
    sender.tx.send(()).is_ok()
}

impl InstanceManager {
    pub async fn init() -> Arc<Self> {
        let manager = Arc::new(Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            instance_dir: PathManager::get().get_instance_dir().to_path_buf(),
            _sync_handle: tokio::spawn(Self::sync_task()),
        });

        let base_dir = PathManager::get().get_instance_dir().to_path_buf();
        cleanup_deleted_instances(&trash_dir(&base_dir)).await;
        let names = if let Ok(mut dir) = tokio::fs::read_dir(&base_dir).await {
            let mut names = Vec::new();
            while let Ok(Some(entry)) = dir.next_entry().await {
                if entry.file_name() != DELETION_DIR && entry.path().is_dir() {
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
                // Skip active operations instead of blocking every other autosave.
                let Ok(files_guard) = handle.try_lock_files() else {
                    continue;
                };
                if let Err(e) = files_guard.save_if_dirty().await {
                    error!("Error guardando instancia {}: {:?}", handle.uuid, e);
                }
            }
            cleanup_deleted_instances(&trash_dir(&manager.instance_dir)).await;
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
        data.instance_root = self.instance_dir.clone();
        // Reserve the name while publishing the new instance, including against renames.
        let mut instances = self.instances.write().await;
        for existing in instances.values() {
            if existing.get_name().await == data.name {
                return Err(InstanceError::AlreadyExists);
            }
        }
        tokio_fs::create_dir(data.get_instance_dir())
            .await
            .map_err(|e| {
                if e.kind() == io::ErrorKind::AlreadyExists {
                    InstanceError::AlreadyExists
                } else {
                    InstanceError::Fs(FsError::CreateDir {
                        path: data.get_instance_dir().to_string_lossy().to_string(),
                        source: e,
                    })
                }
            })?;
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
        instances.insert(handle.uuid.to_string(), handle.clone());

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
        let mut dtos = futures::future::join_all(handles.iter().map(|h| h.to_dto())).await;
        dtos.sort_by(|a, b| b.pinned.cmp(&a.pinned).then_with(|| a.name.cmp(&b.name)));
        dtos
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
        let handle = self
            .get_handle(uuid)
            .await
            .ok_or_else(|| "Instancia no encontrada".to_string())?;

        let files_guard = handle.try_lock_files()?;
        if handle.is_busy() {
            return Err("No se puede eliminar una instancia mientras está en ejecución".into());
        }
        let instances = self.instances.clone();
        let trash = trash_dir(&self.instance_dir);
        // Finish the transaction even if the IPC caller is cancelled after the rename.
        tokio::spawn(async move {
            let uuid = handle.uuid.as_ref();
            let mut instances = instances.write().await;
            let dir = handle.get_instance_dir().await;
            quarantine_instance(&dir, &trash, uuid)
                .await
                .map_err(|e| format!("Error al retirar el directorio: {e}"))?;
            files_guard.mark_deleted();
            instances.remove(uuid);
            drop(instances);
            signal_kill(uuid);
            unregister_kill_sender(uuid);
            remove_log_ring(uuid);
            emit(AppEvent::InstanceDeleted {
                id: uuid.to_compact_string(),
            });
            drop(files_guard);
            // Failure here leaves only a quarantined directory, retried at startup/sync.
            cleanup_deleted_instances(&trash).await;
            Ok(())
        })
        .await
        .map_err(|e| format!("Error en la tarea de eliminación: {e}"))?
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

        let files_guard = handle.try_lock_files()?;

        if handle.is_busy() {
            return Err(
                "No se puede modificar una instancia mientras está en ejecución".to_string(),
            );
        }

        if let Some(name) = new_name {
            validate_instance_name(&name)?;
            let instances = self.instances.write().await;

            let old_name = handle.get_name().await;
            if *old_name != name {
                for existing in instances.values() {
                    if existing.get_name().await.as_ref() == name {
                        return Err("Ya existe una instancia con ese nombre".to_string());
                    }
                }
                let base_dir = &self.instance_dir;
                let old_dir = base_dir.join(&*old_name);
                let new_dir = base_dir.join(&name);

                if new_dir.exists() {
                    return Err("Ya existe una instancia con ese nombre".to_string());
                }
                tokio_fs::rename(&old_dir, &new_dir)
                    .await
                    .map_err(|e| format!("Error al renombrar el directorio: {}", e))?;
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

        files_guard
            .save_if_dirty()
            .await
            .map_err(|e| format!("Error al guardar la instancia: {}", e))?;

        Ok(())
    }
}

fn trash_dir(instances: &Path) -> PathBuf {
    instances.join(DELETION_DIR)
}

async fn quarantine_instance(dir: &Path, trash: &Path, uuid: &str) -> io::Result<()> {
    match tokio_fs::symlink_metadata(dir).await {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    tokio_fs::create_dir_all(trash).await?;
    tokio_fs::rename(dir, trash.join(uuid)).await
}

async fn cleanup_deleted_instances(trash: &Path) {
    let mut entries = match tokio_fs::read_dir(trash).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return,
        Err(e) => {
            error!("Error leyendo instancias pendientes de eliminar: {e}");
            return;
        }
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        if uuid::Uuid::parse_str(&entry.file_name().to_string_lossy()).is_err() {
            continue;
        }
        if let Err(e) = tokio_fs::remove_dir_all(entry.path()).await
            && e.kind() != io::ErrorKind::NotFound
        {
            error!(
                "No se pudo limpiar instancia eliminada {:?}: {e}",
                entry.path()
            );
        }
    }
}

#[cfg(test)]
#[path = "../../tests/services/instance_manager/manager.rs"]
mod tests;
