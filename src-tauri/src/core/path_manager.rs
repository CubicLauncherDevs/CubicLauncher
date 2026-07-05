use directories::UserDirs;
use smallvec::SmallVec;
use std::env::temp_dir;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tracing::info;

static PATH_MANAGER: LazyLock<PathManager> = LazyLock::new(PathManager::initialize);

pub struct PathManager {
    instances_dir: Box<Path>,
    shared_dir: Box<Path>,
    settings_dir: Box<Path>,
    themes_dir: Box<Path>,
}

impl PathManager {
    pub fn get() -> &'static PathManager {
        &PATH_MANAGER
    }

    pub fn get_instance_dir(&self) -> &Path {
        &self.instances_dir
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

    pub fn ensure_dirs() -> Result<(), SmallVec<[String; 4]>> {
        let dirs = [
            Self::get().get_instance_dir(),
            Self::get().get_shared_dir(),
            Self::get().get_settings_dir(),
            Self::get().get_themes_dir(),
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
            instances_dir: base_dir.join(".cubic").join("instances").into_boxed_path(),
            shared_dir: base_dir.join(".cubic").join("shared").into_boxed_path(),
            settings_dir: base_dir.join(".cubic").join("settings").into_boxed_path(),
            themes_dir: base_dir.join(".cubic").join("themes").into_boxed_path(),
        }
    }
}

// utilidades
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
