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
    shared_dir: Box<Path>,
    settings_dir: Box<Path>,
    themes_dir: Box<Path>,
    skin_closet_dir: Box<Path>,
}

impl PathManager {
    pub fn get() -> &'static PathManager {
        &PATH_MANAGER
    }

    pub fn get_instance_dir(&self) -> PathBuf {
        self.instances_dir.read().to_path_buf()
    }
    pub fn get_shared_dir(&self) -> &Path {
        &self.shared_dir
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

    pub fn ensure_dirs() -> Result<(), SmallVec<[String; 4]>> {
        let dirs = [
            Self::get().get_instance_dir(),
            Self::get().get_shared_dir().to_path_buf(),
            Self::get().get_settings_dir().to_path_buf(),
            Self::get().get_themes_dir().to_path_buf(),
            Self::get().get_skin_closet_dir().to_path_buf(),
        ];

        let mut errors = SmallVec::<[String; 4]>::new();
        for dir in &dirs {
            if let Err(e) = std::fs::create_dir_all(dir) {
                errors.push(format!("{}: {}", dir.display(), e));
            } else {
                info!("Directorio asegurado: {:?}", dir);
            }
        }

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
            shared_dir: base_dir.join(".cubic").join("shared").into_boxed_path(),
            settings_dir: base_dir.join(".cubic").join("settings").into_boxed_path(),
            themes_dir: base_dir.join(".cubic").join("themes").into_boxed_path(),
            skin_closet_dir: base_dir.join(".cubic").join("skins").into_boxed_path(),
        }
    }
}

// utilidades

/// Directorio de instancias por defecto (para mensajes de UI).
pub fn default_instances_dir() -> PathBuf {
    resolve_base_dir().join(".cubic").join("instances")
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
