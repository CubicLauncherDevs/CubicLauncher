//! Cambia el directorio donde se guardan las instancias moviendo cada una a la
//! nueva ubicación con **hardlinks** (fallback a copia), sin duplicar espacio
//! en disco. La carpeta original solo se elimina cuando la instancia completa
//! quedó trasladada, así que un fallo deja los datos intactos en su lugar.

use crate::core::PathManager;
use crate::services::{InstanceManager, SettingsManager};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use tracing::{error, info, warn};

static MIGRATION_RUNNING: AtomicBool = AtomicBool::new(false);
static MIGRATION_CANCEL: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
pub struct InstancesDirInfo {
    pub current_dir: String,
    pub custom_dir: String,
    pub default_dir: String,
    pub instance_count: usize,
}

#[derive(Clone, Serialize)]
pub struct MoveProgress {
    pub instance_index: usize,
    pub instance_count: usize,
    pub instance_name: String,
    pub bytes_current: u64,
    pub bytes_total: u64,
    pub file: String,
    /// "hardlink" o "copy": el frontend lo informa si el traslado pierde la
    /// optimización (p. ej. discos distintos).
    pub strategy: &'static str,
}

#[derive(Clone, Serialize)]
pub struct MoveResult {
    pub moved: usize,
    pub failed: Vec<String>,
}

struct FileEntry {
    source: PathBuf,
    relative: PathBuf,
    len: u64,
}

/// Inventario recursivo de archivos regulares de una instancia.
/// Sin symlinks: Minecraft no los crea y evita ciclos infinitos.
fn collect_files(
    dir: &Path,
    root: &Path,
    entries: &mut Vec<FileEntry>,
    total: &mut u64,
    cancelled: &AtomicBool,
) -> Result<(), (String, std::io::Error)> {
    if cancelled.load(Ordering::Relaxed) {
        return Err((
            dir.to_string_lossy().to_string(),
            std::io::Error::other("Cancelado"),
        ));
    }
    for entry in std::fs::read_dir(dir).map_err(|e| (dir.to_string_lossy().to_string(), e))? {
        let entry = entry.map_err(|e| (dir.to_string_lossy().to_string(), e))?;
        let path = entry.path();
        // `file_type` sale de la propia entrada de readdir (d_type en Unix): evita
        // un `stat` por carpeta. El tamaño solo se consulta para archivos.
        let file_type = entry
            .file_type()
            .map_err(|e| (path.to_string_lossy().to_string(), e))?;
        if file_type.is_dir() {
            collect_files(&path, root, entries, total, cancelled)?;
        } else if file_type.is_file() {
            let meta = entry
                .metadata()
                .map_err(|e| (path.to_string_lossy().to_string(), e))?;
            *total += meta.len();
            entries.push(FileEntry {
                relative: path
                    .strip_prefix(root)
                    .expect("entry de read_dir siempre está bajo root")
                    .to_path_buf(),
                len: meta.len(),
                source: path,
            });
        }
    }
    Ok(())
}

/// Copia un archivo intentando crear un hardlink (idéntico bit a bit, sin I/O
/// de datos ni espacio extra). Si no es posible —sistemas de archivos distintos
/// o sin soporte— recurre a una copia incremental.
fn transfer_file(source: &Path, dest: &Path) -> Result<&'static str, std::io::Error> {
    match std::fs::hard_link(source, dest) {
        Ok(()) => Ok("hardlink"),
        Err(link_err) => {
            warn!(
                "hard_link falló para {} ({}); se copiará",
                source.display(),
                link_err
            );
            std::fs::copy(source, dest)?;
            Ok("copy")
        }
    }
}

/// Elimina un árbol borrando primero los archivos: `remove_dir_all` también
/// funciona con enlaces (el contenido sobrevive en el destino) pero puede
/// fallar por carreras de antivirus/indexadores; este bucle es tolerante.
fn remove_tree(dir: &Path) {
    fn walk(dir: &Path) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)?.flatten() {
            let path = entry.path();
            // `file_type` no sigue symlinks ni cuesta un `stat` extra por entrada.
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let _ = walk(&path);
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
        std::fs::remove_dir(dir)
    }
    if let Err(e) = walk(dir) {
        warn!(
            "No se pudo limpiar el árbol original {}: {}",
            dir.display(),
            e
        );
    }
}

/// Descuenta un worker activo al salir del scope, incluso si entra en pánico:
/// así el bucle de progreso nunca espera para siempre.
struct ActiveWorker<'a> {
    active: &'a AtomicUsize,
    done: &'a Mutex<bool>,
    done_cv: &'a Condvar,
}

impl Drop for ActiveWorker<'_> {
    fn drop(&mut self) {
        if self.active.fetch_sub(1, Ordering::Relaxed) == 1 {
            *self.done.lock().unwrap() = true;
            self.done_cv.notify_all();
        }
    }
}

/// Traslada el árbol de `source_root` a `dest_root` con hardlinks (fallback a
/// copia). Devuelve la estrategia global usada y los archivos que fallaron.
///
/// El trabajo pesado (enlazar/copiar) se reparte entre varios hilos: es
/// syscall + latencia de disco, así que un pool pequeño satura mejor la cola
/// que un recorrido secuencial. El progreso se emite desde un único hilo para
/// respetar el callback `FnMut`.
fn move_tree(
    source_root: &Path,
    dest_root: &Path,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(u64, u64, String, &'static str),
) -> Result<(&'static str, Vec<String>), (String, std::io::Error)> {
    let mut entries = Vec::new();
    let mut total = 0u64;
    collect_files(
        source_root,
        source_root,
        &mut entries,
        &mut total,
        cancelled,
    )?;

    // Esqueleto de directorios una sola vez: `create_dir_all` por archivo
    // multiplicaría syscalls sin aportar nada (los archivos de una carpeta son
    // contiguos, así que basta con crear cada carpeta una vez).
    let mut dirs = HashSet::new();
    for entry in &entries {
        if let Some(parent) = entry
            .relative
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            dirs.insert(dest_root.join(parent));
        }
    }
    for dir in &dirs {
        std::fs::create_dir_all(dir).map_err(|e| (dir.to_string_lossy().to_string(), e))?;
    }

    let next = AtomicUsize::new(0);
    let done_index = AtomicUsize::new(usize::MAX);
    let copied = AtomicU64::new(0);
    let used_copy = AtomicBool::new(false);
    let active = AtomicUsize::new(0);
    let cancelled_at = Mutex::new(None::<String>);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let done = Mutex::new(false);
    let done_cv = Condvar::new();

    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .clamp(1, 8)
        .min(entries.len().max(1));

    progress(0, total, String::new(), "hardlink");

    std::thread::scope(|scope| {
        active.store(worker_count, Ordering::Relaxed);
        for _ in 0..worker_count {
            scope.spawn(|| {
                let _worker = ActiveWorker {
                    active: &active,
                    done: &done,
                    done_cv: &done_cv,
                };
                loop {
                    if cancelled.load(Ordering::Relaxed) {
                        // Si ya no queda trabajo sin reclamar, un cancel tardío no
                        // debe abortar un traslado que en realidad terminó.
                        if next.load(Ordering::Relaxed) < entries.len() {
                            let mut slot = cancelled_at.lock().unwrap();
                            if slot.is_none() {
                                *slot = entries
                                    .get(next.load(Ordering::Relaxed))
                                    .map(|e| e.source.to_string_lossy().to_string());
                            }
                        }
                        break;
                    }
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(entry) = entries.get(index) else {
                        break;
                    };
                    let dest = dest_root.join(&entry.relative);
                    match transfer_file(&entry.source, &dest) {
                        Ok(strategy) => {
                            // Basta con que un archivo copie para que el traslado
                            // pierda la optimización; no la revertimos después.
                            if strategy == "copy" {
                                used_copy.store(true, Ordering::Relaxed);
                            }
                            copied.fetch_add(entry.len, Ordering::Relaxed);
                        }
                        Err(e) => {
                            error!(
                                "No se pudo trasladar {} a {}: {}",
                                entry.source.display(),
                                dest.display(),
                                e
                            );
                            failures
                                .lock()
                                .unwrap()
                                .push(entry.relative.to_string_lossy().to_string());
                        }
                    }
                    done_index.store(index, Ordering::Relaxed);
                }
            });
        }

        // Progreso espaciado desde el hilo principal: el callback es `FnMut` y
        // no puede tocarse desde varios workers. El `Condvar` despierta al
        // terminar para no añadir latencia al final de cada instancia.
        let mut last_report = std::time::Instant::now();
        let mut done_guard = done.lock().unwrap();
        while !*done_guard {
            let (guard, timeout) = done_cv
                .wait_timeout(done_guard, std::time::Duration::from_millis(100))
                .unwrap();
            done_guard = guard;
            if *done_guard {
                break;
            }
            if timeout.timed_out() && last_report.elapsed() >= std::time::Duration::from_millis(100)
            {
                let file = entries
                    .get(done_index.load(Ordering::Relaxed))
                    .map(|e| e.relative.to_string_lossy().to_string())
                    .unwrap_or_default();
                let strategy = if used_copy.load(Ordering::Relaxed) {
                    "copy"
                } else {
                    "hardlink"
                };
                progress(copied.load(Ordering::Relaxed), total, file, strategy);
                last_report = std::time::Instant::now();
            }
        }
    });

    if let Some(path) = cancelled_at.into_inner().unwrap() {
        return Err((path, std::io::Error::other("Cancelado")));
    }
    let strategy = if used_copy.load(Ordering::Relaxed) {
        "copy"
    } else {
        "hardlink"
    };
    let failures = failures.into_inner().unwrap();
    progress(
        copied.load(Ordering::Relaxed),
        total,
        String::new(),
        strategy,
    );
    Ok((strategy, failures))
}

fn is_empty_dir(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut d| d.next().is_none())
        .unwrap_or(false)
}

/// Cancela la migración en curso (las instancias ya trasladadas se conservan).
pub fn cancel_instances_dir_change() {
    MIGRATION_CANCEL.store(true, Ordering::Relaxed);
}

/// Cambia el directorio de instancias y mueve todas las existentes.
pub async fn change_instances_dir(
    new_dir: PathBuf,
    on_progress: tauri::ipc::Channel<MoveProgress>,
) -> Result<MoveResult, String> {
    if MIGRATION_RUNNING.swap(true, Ordering::Acquire) {
        return Err("Ya hay un cambio de directorio en curso".into());
    }
    MIGRATION_CANCEL.store(false, Ordering::Relaxed);
    let result = change_instances_dir_impl(new_dir, &on_progress).await;
    MIGRATION_RUNNING.store(false, Ordering::Release);
    result
}

async fn change_instances_dir_impl(
    new_dir: PathBuf,
    on_progress: &tauri::ipc::Channel<MoveProgress>,
) -> Result<MoveResult, String> {
    if !new_dir.is_absolute() {
        return Err("La ruta debe ser absoluta".into());
    }
    let new_dir = new_dir.canonicalize().unwrap_or_else(|_| new_dir.clone());

    let manager = InstanceManager::get();
    let mut handles = manager.get_all_handles().await;
    handles.sort_by(|a, b| {
        // Orden determinista sin bloquearse: comparar UUID es estable y barato.
        a.uuid.cmp(&b.uuid)
    });

    // Retiene el lock de archivos de cada instancia hasta el final: impide que
    // autosave, borrados o ediciones toquen una instancia mientras se mueve.
    let mut guards = Vec::with_capacity(handles.len());
    for handle in &handles {
        if handle.is_busy() {
            return Err(format!(
                "La instancia '{}' está en ejecución; ciérrala antes de mover las instancias",
                handle.get_name().await
            ));
        }
        guards.push(
            handle
                .try_lock_files()
                .map_err(|e| format!("No se pudo bloquear la instancia: {e}"))?,
        );
    }

    let current_dir = PathManager::get().get_instance_dir();
    let already_there = current_dir == new_dir;

    // Anidar el destino dentro del origen (o al revés) corrompería el traslado:
    // las instancias movidas aparecerían como fuentes de las siguientes.
    if !already_there && (new_dir.starts_with(&current_dir) || current_dir.starts_with(&new_dir)) {
        return Err("La nueva ubicación no puede estar dentro de la actual (ni contenerla)".into());
    }

    if !already_there && new_dir.exists() && !is_empty_dir(&new_dir) {
        return Err(format!(
            "El destino debe estar vacío o no existir: {}",
            new_dir.display()
        ));
    }
    std::fs::create_dir_all(&new_dir)
        .map_err(|e| format!("No se pudo crear {}: {e}", new_dir.display()))?;

    let mut moved = 0usize;
    let mut all_failures = Vec::new();
    let count = handles.len();
    for (index, handle) in handles.iter().enumerate() {
        let name = handle.get_name().await;
        let source = handle.get_instance_dir().await;
        let dest = new_dir.join(name.as_ref());
        if source == dest {
            moved += 1;
            continue;
        }
        let cancelled = &MIGRATION_CANCEL;
        let progress_channel = on_progress.clone();
        let instance_name = name.to_string();
        let dest_task = dest.clone();
        let source_task = source.clone();
        let result = tokio::task::spawn_blocking(move || {
            move_tree(
                &source_task,
                &dest_task,
                cancelled,
                |bytes_current, bytes_total, file, strategy| {
                    let _ = progress_channel.send(MoveProgress {
                        instance_index: index + 1,
                        instance_count: count,
                        instance_name: instance_name.clone(),
                        bytes_current,
                        bytes_total,
                        file,
                        strategy,
                    });
                },
            )
        })
        .await
        .map_err(|e| format!("La tarea de traslado falló: {e}"))?;

        match result {
            Ok((strategy, failures)) => {
                if failures.is_empty() {
                    // El original ya es prescindible: solo quedan enlaces muertos.
                    let _ = tokio::task::spawn_blocking({
                        let source = source.clone();
                        move || remove_tree(&source)
                    })
                    .await;
                    info!(
                        "Instancia '{name}' trasladada a {} ({strategy})",
                        dest.display()
                    );
                } else {
                    for f in &failures {
                        all_failures.push(format!("{name}: {f}"));
                    }
                    error!(
                        "Instancia '{name}' trasladada con {} archivos fallidos",
                        failures.len()
                    );
                }
                moved += 1;
            }
            Err((file, e)) => {
                // Limpieza del destino parcial; el original permanece intacto.
                let _ = tokio::fs::remove_dir_all(&dest).await;
                let msg = format!("No se pudo mover '{name}' ({file}): {e}");
                error!("{msg}");
                return Err(msg);
            }
        }
    }
    // Commit: persistir el nuevo directorio y reapuntar en caliente.
    // Orden crítico: PathManager primero (InstanceData::new y las cargas de
    // disco lo leen), luego los handles —aún con files_lock retenido, para que
    // ningún autosave escriba en la ruta vieja mientras se reapuntan— y por
    // último la base del manager y el guardado de settings.
    PathManager::set_instances_dir(new_dir.clone());
    for handle in &handles {
        handle.repoint_root(new_dir.clone()).await;
    }
    manager.update_base_dir(new_dir.clone());
    drop(guards);
    SettingsManager::write(|s| {
        s.custom_instances_dir = new_dir.clone();
    })
    .map_err(|e| format!("No se pudo guardar la configuración: {e}"))?;
    SettingsManager::save()
        .await
        .map_err(|e| format!("No se pudo guardar la configuración: {e}"))?;
    info!("Directorio de instancias cambiado a {}", new_dir.display());

    Ok(MoveResult {
        moved,
        failed: all_failures,
    })
}

pub async fn get_instances_dir_info() -> InstancesDirInfo {
    let custom_dir = SettingsManager::snapshot().custom_instances_dir;
    let instance_count = InstanceManager::get().count().await;
    InstancesDirInfo {
        current_dir: PathManager::get()
            .get_instance_dir()
            .to_string_lossy()
            .to_string(),
        custom_dir: custom_dir.to_string_lossy().to_string(),
        default_dir: crate::core::path_manager::default_instances_dir()
            .to_string_lossy()
            .to_string(),
        instance_count,
    }
}

#[cfg(test)]
#[path = "../tests/services/storage_migration.rs"]
mod tests;
