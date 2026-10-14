//! Perfiles globales de `options.txt`.
//!
//! Un perfil guarda el contenido de un `options.txt` con un nombre para poder
//! aplicarlo a una instancia (manualmente o al lanzar). El contenido se copia de
//! forma atómica: Minecraft reescribe `options.txt` al salir, así que un enlace
//! duro se rompería y dejaría de sincronizar.
//!
//! Los perfiles viven en un archivo propio (`options_profiles.cub`) y no dentro
//! de `settings.cub`, porque el frontend reenvía la configuración completa en
//! `update_settings` y sobrescribiría el contenido de los perfiles.

use crate::core::{AppError, CoreError, FsError, PathManager};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

const STORE_FILE: &str = "options_profiles.cub";
/// Subcarpeta de `settings_dir` donde se guardan los respaldos de `options.txt`.
pub const BACKUP_DIR: &str = "options-backups";
/// `options.txt` es pequeño; 1 MiB deja margen de sobra y evita abusos.
const MAX_CONTENT_BYTES: usize = 1024 * 1024;
const MAX_NAME_LEN: usize = 48;
/// Retención de respaldos con marca de tiempo por instancia.
const MAX_TIMESTAMPED_BACKUPS: usize = 5;

static STORE: LazyLock<RwLock<StoreFile>> =
    LazyLock::new(|| RwLock::new(StoreFile::load_from(&store_path())));

// ── Modelo ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptionsProfile {
    pub id: String,
    pub name: String,
    pub content: String,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Vista que viaja al frontend.
#[derive(Debug, Serialize, Clone)]
pub struct OptionsProfileDto {
    pub id: String,
    pub name: String,
    pub content: String,
    pub created_at: u64,
    pub updated_at: u64,
}

impl From<&OptionsProfile> for OptionsProfileDto {
    fn from(p: &OptionsProfile) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            content: p.content.clone(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StoreFile {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    profiles: Vec<OptionsProfile>,
}

impl Default for StoreFile {
    fn default() -> Self {
        Self {
            version: default_version(),
            profiles: Vec::new(),
        }
    }
}

fn default_version() -> u32 {
    1
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ── Persistencia ──────────────────────────────────────────────────────────────

fn store_path() -> PathBuf {
    PathManager::get().get_settings_dir().join(STORE_FILE)
}

/// Directorio raíz de respaldos de `options.txt`.
pub fn backup_root() -> PathBuf {
    PathManager::get().get_settings_dir().join(BACKUP_DIR)
}

impl StoreFile {
    fn load_from(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<Self>(&bytes) {
                Ok(store) => {
                    info!("Perfiles de options.txt cargados desde {:?}", path);
                    store
                }
                Err(e) => {
                    warn!(
                        "options_profiles.cub inválido en {:?} ({}), creando backup",
                        path, e
                    );
                    let _ = std::fs::copy(path, path.with_extension("cub.bak"));
                    Self::default()
                }
            },
            Err(e) => {
                warn!("Error leyendo options_profiles.cub en {:?}: {}", path, e);
                Self::default()
            }
        }
    }

    fn persist_to(&self, path: &Path) -> Result<(), AppError> {
        let parent = path
            .parent()
            .ok_or_else(|| CoreError::Other("Falta el directorio de ajustes".into()))?;
        std::fs::create_dir_all(parent).map_err(|source| FsError::CreateDir {
            path: parent.to_string_lossy().into_owned(),
            source,
        })?;
        let bytes = serde_json::to_vec(self).map_err(|e| CoreError::Serialize(e.to_string()))?;
        crate::core::atomic_file::write(path, &bytes).map_err(|source| FsError::WriteFile {
            path: path.to_string_lossy().into_owned(),
            source,
        })?;
        Ok(())
    }
}

// ── Validación ────────────────────────────────────────────────────────────────

fn validate_name(name: &str) -> Result<(), AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Other("El nombre del perfil no puede estar vacío".into()).into());
    }
    if trimmed.chars().count() > MAX_NAME_LEN {
        return Err(CoreError::Other(format!(
            "El nombre del perfil no puede superar {MAX_NAME_LEN} caracteres"
        ))
        .into());
    }
    if trimmed.chars().any(char::is_control) {
        return Err(CoreError::Other(
            "El nombre del perfil contiene caracteres no permitidos".into(),
        )
        .into());
    }
    Ok(())
}

fn validate_content(content: &str) -> Result<(), AppError> {
    if content.len() > MAX_CONTENT_BYTES {
        return Err(CoreError::Other(format!(
            "El options.txt supera el tamaño máximo de {} MiB",
            MAX_CONTENT_BYTES / (1024 * 1024)
        ))
        .into());
    }
    Ok(())
}

// ── Store (testeable) ─────────────────────────────────────────────────────────

impl StoreFile {
    pub fn list(&self) -> Vec<OptionsProfileDto> {
        self.profiles.iter().map(OptionsProfileDto::from).collect()
    }

    pub fn get(&self, id: &str) -> Option<&OptionsProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn create(&mut self, name: &str, content: &str) -> Result<OptionsProfile, AppError> {
        validate_name(name)?;
        validate_content(content)?;
        let now = now_secs();
        let profile = OptionsProfile {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            content: content.to_string(),
            created_at: now,
            updated_at: now,
        };
        self.profiles.push(profile.clone());
        Ok(profile)
    }

    pub fn update(
        &mut self,
        id: &str,
        name: &str,
        content: &str,
    ) -> Result<OptionsProfile, AppError> {
        validate_name(name)?;
        validate_content(content)?;
        let profile = self
            .profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| CoreError::Other("Perfil no encontrado".into()))?;
        profile.name = name.trim().to_string();
        profile.content = content.to_string();
        profile.updated_at = now_secs();
        Ok(profile.clone())
    }

    pub fn delete(&mut self, id: &str) -> Result<(), AppError> {
        let before = self.profiles.len();
        self.profiles.retain(|p| p.id != id);
        if self.profiles.len() == before {
            return Err(CoreError::Other("Perfil no encontrado".into()).into());
        }
        Ok(())
    }

    fn remove_raw(&mut self, id: &str) {
        self.profiles.retain(|p| p.id != id);
    }

    fn restore(&mut self, profile: OptionsProfile) {
        self.remove_raw(&profile.id);
        self.profiles.push(profile);
    }
}

// ── API global ────────────────────────────────────────────────────────────────

pub fn list() -> Vec<OptionsProfileDto> {
    STORE.read().list()
}

pub fn get(id: &str) -> Option<OptionsProfile> {
    STORE.read().get(id).cloned()
}

pub fn create(name: &str, content: &str) -> Result<OptionsProfileDto, AppError> {
    let mut store = STORE.write();
    let profile = store.create(name, content)?;
    if let Err(e) = store.persist_to(&store_path()) {
        store.remove_raw(&profile.id);
        return Err(e);
    }
    Ok(OptionsProfileDto::from(&profile))
}

pub fn update(id: &str, name: &str, content: &str) -> Result<OptionsProfileDto, AppError> {
    let mut store = STORE.write();
    let previous = store.get(id).cloned();
    let profile = store.update(id, name, content)?;
    if let Err(e) = store.persist_to(&store_path()) {
        if let Some(previous) = previous {
            store.restore(previous);
        }
        return Err(e);
    }
    Ok(OptionsProfileDto::from(&profile))
}

pub fn delete(id: &str) -> Result<(), AppError> {
    let mut store = STORE.write();
    let previous = store.get(id).cloned();
    store.delete(id)?;
    if let Err(e) = store.persist_to(&store_path()) {
        if let Some(previous) = previous {
            store.restore(previous);
        }
        return Err(e);
    }
    Ok(())
}

// ── Aplicación a instancias ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupMode {
    /// No respalda (uso interno).
    None,
    /// Respaldo rodante único (`options.txt.bak`). Se usa en el auto-apply.
    Rolling,
    /// Respaldo con marca de tiempo, con retención. Se usa en el apply manual.
    Timestamped,
}

#[derive(Debug, Serialize)]
pub struct ApplyOutcome {
    /// `true` si el `options.txt` cambió; `false` si ya coincidía.
    pub changed: bool,
    pub backup: Option<PathBuf>,
}

/// Lee el `options.txt` de una instancia, si existe.
pub fn read_instance_options(instance_dir: &Path) -> Option<String> {
    std::fs::read_to_string(instance_dir.join("options.txt")).ok()
}

/// Copia `content` al `options.txt` de la instancia de forma atómica.
///
/// Si el contenido ya coincide no escribe nada. Antes de sobrescribir crea un
/// respaldo del `options.txt` actual según `mode`, dentro de
/// `backup_root/<instance_key>/`.
pub fn apply_to_instance(
    instance_dir: &Path,
    instance_key: &str,
    content: &str,
    mode: BackupMode,
    backup_root: &Path,
) -> Result<ApplyOutcome, AppError> {
    validate_content(content)?;
    let target = instance_dir.join("options.txt");
    let existing = std::fs::read_to_string(&target).ok();
    if existing.as_deref() == Some(content) {
        return Ok(ApplyOutcome {
            changed: false,
            backup: None,
        });
    }

    let backup = if existing.is_some() && mode != BackupMode::None {
        create_backup(&target, instance_key, mode, backup_root)?
    } else {
        None
    };

    std::fs::create_dir_all(instance_dir).map_err(|source| FsError::CreateDir {
        path: instance_dir.to_string_lossy().into_owned(),
        source,
    })?;
    crate::core::atomic_file::write(&target, content.as_bytes()).map_err(|source| {
        FsError::WriteFile {
            path: target.to_string_lossy().into_owned(),
            source,
        }
    })?;
    info!(
        "options.txt aplicado a {:?} (backup: {:?})",
        instance_dir, backup
    );
    Ok(ApplyOutcome {
        changed: true,
        backup,
    })
}

fn sanitize_key(key: &str) -> String {
    let cleaned: String = key
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if cleaned.is_empty() {
        "unknown".to_string()
    } else {
        cleaned
    }
}

fn create_backup(
    target: &Path,
    instance_key: &str,
    mode: BackupMode,
    backup_root: &Path,
) -> Result<Option<PathBuf>, AppError> {
    let dir = backup_root.join(sanitize_key(instance_key));
    std::fs::create_dir_all(&dir).map_err(|source| FsError::CreateDir {
        path: dir.to_string_lossy().into_owned(),
        source,
    })?;
    let filename = match mode {
        BackupMode::None => return Ok(None),
        BackupMode::Rolling => "options.txt.bak".to_string(),
        BackupMode::Timestamped => format!("options-{}.txt", now_secs()),
    };
    let dest = dir.join(filename);
    std::fs::copy(target, &dest).map_err(|source| FsError::Copy {
        from: target.to_string_lossy().into_owned(),
        to: dest.to_string_lossy().into_owned(),
        source,
    })?;
    if mode == BackupMode::Timestamped {
        prune_backups(&dir, MAX_TIMESTAMPED_BACKUPS);
    }
    Ok(Some(dest))
}

/// Conserva los `keep` respaldos más recientes (los nombres ordenan por tiempo).
fn prune_backups(dir: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    if files.len() <= keep {
        return;
    }
    files.sort();
    let remove_count = files.len() - keep;
    for path in files.into_iter().take(remove_count) {
        if let Err(e) = std::fs::remove_file(&path) {
            warn!("No se pudo eliminar respaldo {:?}: {}", path, e);
        }
    }
}

#[cfg(test)]
#[path = "../tests/services/options_profiles.rs"]
mod tests;
