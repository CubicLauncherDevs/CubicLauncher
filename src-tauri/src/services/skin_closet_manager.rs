use crate::core::{HTTP, PathManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info};

const SKIN_CLOSET_VERSION: u32 = 1;
const MAX_ENTRIES_PER_USER: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinClosetEntry {
    pub id: String,
    pub url: String,
    pub local_path: String,
    pub variant: String,
    #[serde(default)]
    pub alias: String,
    pub saved_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinClosetData {
    pub version: u32,
    pub entries: HashMap<String, Vec<SkinClosetEntry>>,
}

impl Default for SkinClosetData {
    fn default() -> Self {
        Self {
            version: SKIN_CLOSET_VERSION,
            entries: HashMap::new(),
        }
    }
}

static SKIN_CLOSET: LazyLock<RwLock<SkinClosetData>> =
    LazyLock::new(|| RwLock::new(SkinClosetData::load()));

#[allow(dead_code)]
pub enum SkinImageSource {
    File(PathBuf),
    Url(String),
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl SkinClosetData {
    pub fn file_path() -> PathBuf {
        PathManager::get()
            .get_settings_dir()
            .join("skin_closet.json")
    }

    fn load() -> Self {
        let path = Self::file_path();
        if !path.exists() {
            info!("No existe skin_closet.json, se creará uno nuevo");
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<SkinClosetData>(&content) {
                Ok(data) => data,
                Err(e) => {
                    error!("Error parseando skin_closet.json: {}", e);
                    Self::default()
                }
            },
            Err(e) => {
                error!("Error leyendo skin_closet.json: {}", e);
                Self::default()
            }
        }
    }

    pub async fn save(&self) -> Result<(), String> {
        let path = Self::file_path();
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| format!("Error serializando closet de skins: {}", e))?;
        tokio::fs::write(&path, json)
            .await
            .map_err(|e| format!("Error guardando closet de skins: {}", e))?;
        Ok(())
    }
}

pub struct SkinClosetManager;

impl SkinClosetManager {
    pub async fn get_entries(uuid: &str) -> Vec<SkinClosetEntry> {
        let data = SKIN_CLOSET.read().await;
        data.entries.get(uuid).cloned().unwrap_or_default()
    }

    pub async fn sync_entry(uuid: &str, entry: SkinClosetEntry) -> Result<(), String> {
        let data_to_save = {
            let mut data = SKIN_CLOSET.write().await;
            let list = data.entries.entry(uuid.to_string()).or_default();

            if let Some(idx) = list.iter().position(|e| e.id == entry.id) {
                list[idx] = entry;
            } else {
                list.push(entry);
            }

            list.sort_by_key(|a| std::cmp::Reverse(a.saved_at));
            if list.len() > MAX_ENTRIES_PER_USER {
                let removed = list.split_off(MAX_ENTRIES_PER_USER);
                for entry in removed {
                    if let Err(e) = Self::remove_image(&entry.local_path).await {
                        error!("Error eliminando imagen expirada del closet: {}", e);
                    }
                }
            }

            data.clone()
        };
        data_to_save.save().await
    }

    pub async fn remove_entry(uuid: &str, entry_id: &str) -> Result<(), String> {
        let data_to_save = {
            let mut data = SKIN_CLOSET.write().await;
            let Some(list) = data.entries.get_mut(uuid) else {
                return Ok(());
            };
            if let Some(entry) = list.iter().find(|e| e.id == entry_id) {
                Self::remove_image(&entry.local_path).await?;
            }
            list.retain(|e| e.id != entry_id);
            data.clone()
        };
        data_to_save.save().await
    }

    pub async fn update_alias(uuid: &str, entry_id: &str, alias: String) -> Result<(), String> {
        let data_to_save = {
            let mut data = SKIN_CLOSET.write().await;
            if let Some(list) = data.entries.get_mut(uuid)
                && let Some(entry) = list.iter_mut().find(|e| e.id == entry_id)
            {
                entry.alias = alias;
            }
            data.clone()
        };
        data_to_save.save().await
    }

    pub async fn store_image(
        uuid: &str,
        skin_id: &str,
        source: SkinImageSource,
    ) -> Result<PathBuf, String> {
        let dir = PathManager::get().get_skin_closet_dir().join(uuid);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| format!("Error creando directorio del closet: {}", e))?;

        let dest = dir.join(format!("{}.png", skin_id));

        match source {
            SkinImageSource::File(path) => {
                tokio::fs::copy(&path, &dest)
                    .await
                    .map_err(|e| format!("Error copiando imagen al closet: {}", e))?;
            }
            SkinImageSource::Url(url) => {
                let bytes = HTTP
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Error descargando skin: {}", e))?
                    .bytes()
                    .await
                    .map_err(|e| format!("Error leyendo respuesta de skin: {}", e))?;
                tokio::fs::write(&dest, bytes)
                    .await
                    .map_err(|e| format!("Error escribiendo imagen del closet: {}", e))?;
            }
        }

        Ok(dest)
    }

    async fn remove_image(local_path: &str) -> Result<(), String> {
        let path = Path::new(local_path);
        if path.exists() {
            tokio::fs::remove_file(path)
                .await
                .map_err(|e| format!("Error eliminando imagen del closet: {}", e))?;
        }
        Ok(())
    }
}

pub fn now_ts() -> i64 {
    current_timestamp()
}
