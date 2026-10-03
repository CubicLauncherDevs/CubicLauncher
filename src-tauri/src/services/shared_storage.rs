//! Uso de disco y purga del directorio **shared** (versions/libraries/assets).
//!
//! Toda su contenido se re-descarga bajo demanda (al lanzar una
//! instancia o instalar un loader), así que la purga nunca destruye datos del
//! usuario: solo caché regenerable. El cambio de ruta borra el contenido de la
//! ubicación elegida y reapunta: nada se copia ni se re-descarga hasta que se
//! lanza algo.

use crate::core::{PathManager, default_shared_dir};
use crate::services::{DownloadQueue, InstanceManager, SettingsManager};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use tracing::{error, info, warn};

static SHARED_ACCESS: LazyLock<Arc<RwLock<()>>> = LazyLock::new(|| Arc::new(RwLock::new(())));

pub(crate) type SharedGuard = Arc<OwnedRwLockReadGuard<()>>;

/// Acquire before resolving paths; keep this permission until every task/process
/// using those paths has finished. Clones may be moved into blocking workers.
pub(crate) async fn acquire() -> SharedGuard {
    Arc::new(SHARED_ACCESS.clone().read_owned().await)
}

pub(crate) fn try_exclusive() -> Result<OwnedRwLockWriteGuard<()>, String> {
    SHARED_ACCESS.clone().try_write_owned().map_err(|_| {
        "El almacenamiento compartido está en uso; espera a que terminen las operaciones activas".into()
    })
}

/// Tope de profundidad del inventario: los árboles de Minecraft nunca llegan
/// a este nivel, y un límite acotado el recorrido ante árboles patológicos.
const MAX_SCAN_DEPTH: usize = 40;

/// Tamaño y borrabilidad de un directorio, en bytes y archivos.
#[derive(Clone, Copy, Serialize, Default)]
pub struct DirUsage {
    pub bytes: u64,
    pub files: u64,
    /// true si alguna entrada no pudo inspeccionarse o borrarse.
    pub blocked: bool,
}

impl DirUsage {
    fn add(&mut self, other: DirUsage) {
        self.bytes += other.bytes;
        self.files += other.files;
        self.blocked |= other.blocked;
    }
}

/// Estado del directorio shared para la UI de ajustes.
#[derive(Clone, Serialize)]
pub struct SharedDirInfo {
    pub current_dir: String,
    pub custom_dir: String,
    pub default_dir: String,
    /// Tamaño total de shared.
    pub total: DirUsage,
    /// Desglose por subcarpeta, ordenado por nombre.
    pub breakdown: Vec<(String, DirUsage)>,
}

/// Inventario recursivo de un directorio: bytes de archivos regulares y si el
/// árbol es medible/borrable. Los symlinks (los JRE gestionados traen cientos
/// en `legal/`) no bloquean nada: borrar el enlace no toca su destino. Errores
/// de lectura sí marcan el árbol como no borrable.
fn compute_usage(dir: &Path, usage: &mut DirUsage) {
    compute_usage_inner(dir, usage, 0);
}

fn compute_usage_inner(dir: &Path, usage: &mut DirUsage, depth: usize) {
    if depth > MAX_SCAN_DEPTH {
        usage.blocked = true;
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        usage.blocked = true;
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // `file_type` sale de la propia entrada (d_type en Unix): evita un
        // `stat` por carpeta.
        let Ok(file_type) = entry.file_type() else {
            usage.blocked = true;
            return;
        };
        if file_type.is_symlink() {
            // Se salta: no aporta tamaño y su borrado es seguro (el enlace, no
            // el destino). Nunca se recorre su contenido.
            continue;
        }
        if file_type.is_dir() {
            compute_usage_inner(&path, usage, depth + 1);
            if usage.blocked {
                return;
            }
        } else if file_type.is_file() {
            match entry.metadata() {
                Ok(meta) => {
                    usage.bytes += meta.len();
                    usage.files += 1;
                }
                Err(_) => {
                    usage.blocked = true;
                    return;
                }
            }
        } else {
            usage.blocked = true;
            return;
        }
    }
}

/// Uso de disco de un directorio (vacío si no existe).
fn dir_usage(dir: &Path) -> DirUsage {
    let mut usage = DirUsage::default();
    if dir.exists() {
        compute_usage(dir, &mut usage);
    }
    usage
}

fn shared_breakdown() -> (DirUsage, Vec<(String, DirUsage)>) {
    let shared = PathManager::get().get_shared_dir();
    let mut total = DirUsage::default();
    let mut breakdown = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&shared) {
        let mut items: Vec<(String, DirUsage)> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let usage = dir_usage(&entry.path());
            total.add(usage);
            items.push((name, usage));
        }
        items.sort_by(|a, b| a.0.cmp(&b.0));
        breakdown = items;
    }
    (total, breakdown)
}

pub async fn get_shared_dir_info() -> SharedDirInfo {
    let custom_dir = SettingsManager::snapshot().custom_shared_dir;
    let (total, breakdown) = tokio::task::spawn_blocking(shared_breakdown)
        .await
        .unwrap_or_default();
    SharedDirInfo {
        current_dir: PathManager::get()
            .get_shared_dir()
            .to_string_lossy()
            .to_string(),
        custom_dir: custom_dir.to_string_lossy().to_string(),
        default_dir: default_shared_dir().to_string_lossy().to_string(),
        total,
        breakdown,
    }
}

/// Un lanzamiento en curso toca shared (nativos, versiones, descargas
/// pendientes que reapuntan la cola): purgar en paralelo podría dejar archivos
/// a medio escribir o fallar por archivos abiertos.
async fn shared_is_busy() -> Option<String> {
    for handle in InstanceManager::get().get_all_handles().await {
        if handle.is_busy() {
            return Some(handle.get_name().await.to_string());
        }
    }
    if !DownloadQueue::get().get_active_downloads().await.is_empty() {
        return Some(String::new());
    }
    None
}

/// Borra el contenido de `dir` (no el propio directorio). Devuelve los nombres
/// de las entradas que no se pudieron borrar. Los symlinks se eliminan como
/// enlaces: `remove_file` no sigue el enlace, así que el destino queda intacto.
fn clear_dir(dir: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(file_type) = entry.file_type() else {
            failures.push(name);
            continue;
        };
        let result = if file_type.is_dir() {
            std::fs::remove_dir_all(&path)
        } else if file_type.is_file() || file_type.is_symlink() {
            std::fs::remove_file(&path)
        } else {
            warn!("Entrada desconocida durante la purga: {}", path.display());
            failures.push(name);
            continue;
        };
        if let Err(e) = result {
            warn!("No se pudo borrar {}: {}", path.display(), e);
            failures.push(name);
        }
    }
    Ok(failures)
}

async fn purge_shared_at(path: PathBuf) -> Result<DirUsage, String> {
    // El tamaño se mide antes de borrar para informar cuánto se liberó.
    let total = tokio::task::spawn_blocking({
        let path = path.clone();
        move || {
            let mut usage = DirUsage::default();
            if path.exists() {
                compute_usage(&path, &mut usage);
            }
            usage
        }
    })
    .await
    .map_err(|e| format!("La tarea de purga falló: {e}"))?;

    if total.blocked {
        return Err(
            "La carpeta contiene enlaces o archivos inaccesibles: bórralos a mano".to_string(),
        );
    }

    let removed = tokio::task::spawn_blocking(move || {
        if !path.exists() {
            return Ok(Vec::new());
        }
        clear_dir(&path)
    })
    .await
    .map_err(|e| format!("La tarea de purga falló: {e}"))?
    .map_err(|e| format!("No se pudo leer la carpeta shared: {e}"))?;

    for name in &removed {
        error!("Entrada no borrada durante la purga de shared: {name}");
    }
    if !removed.is_empty() {
        return Err(format!(
            "No se pudieron borrar {} entrada(s) (¿archivos en uso?): {}",
            removed.len(),
            removed.join(", ")
        ));
    }
    Ok(total)
}

/// Borra todo el contenido de shared. Se re-descarga al lanzar.
pub async fn purge_shared_dir() -> Result<DirUsage, String> {
    let exclusive = try_exclusive()?;
    // Own admission inside the task: dropping an IPC future must not unlock
    // storage while spawn_blocking is still deleting its contents.
    tokio::spawn(async move {
        let _exclusive = exclusive;
        if shared_is_busy().await.is_some() {
            return Err("Hay instancias o descargas activas; espera a que terminen".into());
        }
        let result = purge_shared_at(PathManager::get().get_shared_dir()).await;
        match &result {
            Ok(usage) => info!(
                "Purga de shared completada: {} bytes en {} archivos liberados",
                usage.bytes, usage.files
            ),
            Err(e) => error!("Purga de shared falló: {e}"),
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reapunta shared a `new_dir`: borra el contenido de la ubicación elegida,
/// actualiza PathManager y guarda la ruta en settings. La ubicación anterior
/// se conserva vacía; borrarla es responsabilidad del usuario.
pub async fn change_shared_dir(new_dir: PathBuf) -> Result<DirUsage, String> {
    let exclusive = try_exclusive()?;
    tokio::spawn(async move {
        let _exclusive = exclusive;
        change_shared_dir_impl(new_dir).await
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn change_shared_dir_impl(new_dir: PathBuf) -> Result<DirUsage, String> {
    if !new_dir.is_absolute() {
        return Err("La ruta debe ser absoluta".into());
    }
    let new_dir = new_dir.canonicalize().unwrap_or_else(|_| new_dir.clone());

    if let Some(name) = shared_is_busy().await {
        return Err(if name.is_empty() {
            "Hay descargas activas; espera a que terminen".to_string()
        } else {
            format!("La instancia '{name}' está en ejecución; ciérrala antes de cambiar la ruta")
        });
    }

    // Anidar el destino dentro del origen (o al revés) provocaría que la purga
    // del destino recorra datos ajenos.
    let current_dir = PathManager::get().get_shared_dir();
    if new_dir.starts_with(&current_dir) || current_dir.starts_with(&new_dir) {
        return Err("La nueva ubicación no puede estar dentro de la actual (ni contenerla)".into());
    }
    std::fs::create_dir_all(&new_dir)
        .map_err(|e| format!("No se pudo crear {}: {e}", new_dir.display()))?;

    let usage = purge_shared_at(new_dir.clone()).await?;
    PathManager::set_shared_dir(new_dir.clone());
    SettingsManager::write(|s| {
        s.custom_shared_dir = new_dir.clone();
    })
    .map_err(|e| format!("No se pudo guardar la configuración: {e}"))?;
    SettingsManager::save()
        .await
        .map_err(|e| format!("No se pudo guardar la configuración: {e}"))?;
    info!("Directorio shared cambiado a {}", new_dir.display());
    Ok(usage)
}

#[cfg(test)]
#[path = "../tests/services/shared_storage.rs"]
mod tests;
