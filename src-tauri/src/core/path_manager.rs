use directories::UserDirs;
use parking_lot::RwLock;
use smallvec::SmallVec;
use std::env::temp_dir;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tracing::{info, warn};

static PATH_MANAGER: LazyLock<PathManager> = LazyLock::new(PathManager::initialize);

pub struct PathManager {
    /// Único directorio que puede cambiar en caliente (migración de disco).
    instances_dir: RwLock<Box<Path>>,
    /// También cambia en caliente: el cambio de ruta de shared purga el
    /// contenido y lo re-descarga bajo demanda.
    shared_dir: RwLock<Box<Path>>,
    settings_dir: Box<Path>,
    themes_dir: Box<Path>,
    skin_closet_dir: Box<Path>,
    /// Datos descartables (iconos del market); se puede borrar sin perder nada.
    cache_dir: Box<Path>,
}

impl PathManager {
    pub fn get() -> &'static PathManager {
        &PATH_MANAGER
    }

    pub fn get_instance_dir(&self) -> PathBuf {
        self.instances_dir.read().to_path_buf()
    }
    /// Copia del directorio shared vigente (se lee bajo lock porque puede
    /// reapuntarse en caliente).
    pub fn get_shared_dir(&self) -> PathBuf {
        self.shared_dir.read().to_path_buf()
    }
    pub fn get_settings_dir(&self) -> &Path {
        &self.settings_dir
    }
    pub fn get_themes_dir(&self) -> &Path {
        &self.themes_dir
    }
    pub fn get_skin_closet_dir(&self) -> &Path {
        &self.skin_closet_dir
    }
    pub fn get_cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Subdirectorio de caché por tipo, creado si falta. Mantiene los archivos
    /// descartables (manifiestos, idiomas, iconos) agrupados y fuera de
    /// `settings`.
    pub fn cache_subdir(&self, kind: &str) -> PathBuf {
        let dir = self.cache_dir.join(kind);
        if let Err(error) = std::fs::create_dir_all(&dir) {
            warn!("No se pudo crear la caché {}: {}", dir.display(), error);
        }
        dir
    }

    /// Reapunta el directorio de instancias en caliente. Solo se usa tras una
    /// migración completada; el arranque lo resuelve desde settings.cub.
    pub fn set_instances_dir(dir: PathBuf) {
        let manager = Self::get();
        let current = manager.instances_dir.read().clone();
        if current.as_ref() == dir.as_path() {
            return;
        }
        info!(
            "Directorio de instancias actualizado: {} -> {}",
            current.display(),
            dir.display()
        );
        *manager.instances_dir.write() = dir.into_boxed_path();
    }

    /// Reapunta el directorio shared en caliente. Solo se usa tras purgar el
    /// contenido; el arranque lo resuelve desde settings.cub.
    pub fn set_shared_dir(dir: PathBuf) {
        let manager = Self::get();
        let current = manager.shared_dir.read().clone();
        if current.as_ref() == dir.as_path() {
            return;
        }
        info!(
            "Directorio shared actualizado: {} -> {}",
            current.display(),
            dir.display()
        );
        *manager.shared_dir.write() = dir.into_boxed_path();
    }

    pub fn ensure_dirs() -> Result<(), SmallVec<[String; 4]>> {
        let dirs = [
            Self::get().get_instance_dir(),
            Self::get().get_shared_dir(),
            Self::get().get_settings_dir().to_path_buf(),
            Self::get().get_themes_dir().to_path_buf(),
            Self::get().get_skin_closet_dir().to_path_buf(),
            Self::get().get_cache_dir().to_path_buf(),
        ];

        let mut errors = SmallVec::<[String; 4]>::new();
        for dir in &dirs {
            if let Err(e) = std::fs::create_dir_all(dir) {
                errors.push(format!("{}: {}", dir.display(), e));
            } else {
                info!("Directorio asegurado: {:?}", dir);
            }
        }

        migrate_legacy_cache(Self::get().get_settings_dir(), Self::get().get_cache_dir());

        if errors.is_empty() {
            info!("Todos los directorios necesarios existen");
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn initialize() -> PathManager {
        let base_dir = resolve_base_dir();

        PathManager {
            instances_dir: RwLock::new(resolve_instances_dir(&base_dir).into_boxed_path()),
            shared_dir: RwLock::new(resolve_shared_dir(&base_dir).into_boxed_path()),
            settings_dir: base_dir.join(".cubic").join("settings").into_boxed_path(),
            themes_dir: base_dir.join(".cubic").join("themes").into_boxed_path(),
            skin_closet_dir: base_dir.join(".cubic").join("skins").into_boxed_path(),
            cache_dir: base_dir.join(".cubic").join("cache").into_boxed_path(),
        }
    }
}

// utilidades

/// Directorio de instancias por defecto (para mensajes de UI).
pub fn default_instances_dir() -> PathBuf {
    resolve_base_dir().join(".cubic").join("instances")
}

/// Directorio shared por defecto (para mensajes de UI).
pub fn default_shared_dir() -> PathBuf {
    resolve_base_dir().join(".cubic").join("shared")
}

/// Lee `custom_shared_dir` de settings.cub, si existe y es válido.
/// Debe tolerar un archivo corrupto: en ese caso se usa el directorio por defecto.
fn resolve_shared_dir(base_dir: &Path) -> PathBuf {
    let default = base_dir.join(".cubic").join("shared");
    let path = base_dir
        .join(".cubic")
        .join("settings")
        .join("settings.cub");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return default;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        warn!("settings.cub inválido al resolver el directorio shared");
        return default;
    };
    match json
        .get("custom_shared_dir")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        Some(dir) => {
            info!("Directorio shared configurado: {}", dir.display());
            dir
        }
        None => default,
    }
}

/// Lee `custom_instances_dir` de settings.cub, si existe y es válido.
/// Debe tolerar un archivo corrupto: en ese caso se usa el directorio por defecto.
fn resolve_instances_dir(base_dir: &Path) -> PathBuf {
    let default = base_dir.join(".cubic").join("instances");
    let path = base_dir
        .join(".cubic")
        .join("settings")
        .join("settings.cub");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return default;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        warn!("settings.cub inválido al resolver el directorio de instancias");
        return default;
    };
    match json
        .get("custom_instances_dir")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        Some(dir) => {
            info!("Directorio de instancias configurado: {}", dir.display());
            dir
        }
        None => default,
    }
}

/// Cachés que versiones anteriores guardaban junto a `settings.cub`.
const LEGACY_CACHE_FILES: [&str; 5] = [
    "manifest.crep",
    "fabric.crep",
    "forge_v3.crep",
    "neoforge_v3.crep",
    "quilt.crep",
];

/// Mueve las cachés antiguas al directorio de caché. Es best-effort: si algo
/// falla se vuelve a descargar, así que nunca bloquea el arranque.
fn migrate_legacy_cache(settings_dir: &Path, cache_dir: &Path) {
    let manifests = cache_dir.join("manifests");
    for name in LEGACY_CACHE_FILES {
        let from = settings_dir.join(name);
        if !from.is_file() {
            continue;
        }
        let to = manifests.join(name);
        if to.exists() {
            continue;
        }
        if let Err(error) = std::fs::create_dir_all(&manifests) {
            warn!("No se pudo crear la caché de manifiestos: {error}");
            return;
        }
        match std::fs::rename(&from, &to) {
            Ok(()) => info!("Caché migrada: {} -> {}", from.display(), to.display()),
            Err(error) => warn!("No se pudo migrar {}: {}", from.display(), error),
        }
    }

    let from = settings_dir.join("i18n");
    let to = cache_dir.join("i18n");
    if from.is_dir() && !to.exists() {
        if let Err(error) = std::fs::create_dir_all(cache_dir) {
            warn!("No se pudo crear la caché: {error}");
            return;
        }
        match std::fs::rename(&from, &to) {
            Ok(()) => info!("Caché de idiomas migrada a {}", to.display()),
            Err(error) => warn!("No se pudo migrar {}: {}", from.display(), error),
        }
    }
}

fn resolve_base_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from)
        && home.is_dir()
    {
        info!("Directorio base resuelto: home dir {:?}", home);
        return home;
    }
    if let Some(d) = UserDirs::new() {
        let path = d.home_dir().to_path_buf();
        info!("Directorio base resuelto: home dir {:?}", path);
        return path;
    }
    // en el caso de que no obtenga path que use el path actual de donde se ejecuta el binario
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        info!("Directorio base resuelto: exe dir {:?}", parent);
        return parent.to_path_buf();
    }
    // si eso no funciona
    // entonces dir de trabajo actual lpm
    if let Ok(cwd) = std::env::current_dir() {
        info!("Directorio base resuelto: cwd {:?}", cwd);
        return cwd;
    }
    // si tampoco da entonces temp
    let tmp = temp_dir();
    info!("Directorio base resuelto: temp dir {:?}", tmp);
    tmp
}

#[cfg(test)]
#[path = "../tests/core/path_manager.rs"]
mod tests;
