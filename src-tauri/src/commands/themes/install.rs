use super::archive::{StagedTheme, build_theme_id, stage_zip};
use super::assets::{extract_preview, resolve_theme_asset};
use super::v1::{ThemeEntry, ThemeFile};
use super::v2::ThemeMeta;
use super::{ZipImportable, load_user_theme};
use crate::core::errors::{CoreError, FsError};
use crate::core::{PathManager, safe_join, validate_identifier};
use crate::theme_watcher::ThemeWatcher;
use tauri::command;
use tracing::{error, info, warn};

// Lock de instalasion hehe
static INSTALL_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

fn import_zip_inner<T: ZipImportable>(zip_path: &str) -> Result<Option<ThemeEntry>, String> {
    let themes_dir = PathManager::get().get_themes_dir();
    let Some((entry, staged)) = stage_zip::<T>(zip_path, themes_dir)? else {
        return Ok(None);
    };
    let _install = INSTALL_LOCK.lock();
    let destination = themes_dir.join(entry.id.as_str());
    let watch = ThemeWatcher::pause_import(&destination);
    staged.install(&destination)?;
    watch.finish();
    info!("Theme importado: id='{}'", entry.id);
    Ok(Some(entry))
}

// Separe esto pq me agobiaban esas 700 lineas 
#[command]
pub fn import_theme(source_path: String) -> Result<ThemeEntry, String> {
    info!("Importando theme desde '{}'", source_path);
    let source = std::path::Path::new(&source_path);
    if !source.exists() {
        error!("Archivo de theme no existe: {}", source_path);
        return Err(FsError::NotFound(source_path.clone()).to_string());
    }

    let content = std::fs::read_to_string(source).map_err(|e| {
        FsError::ReadFile {
            path: source_path.clone(),
            source: e,
        }
        .to_string()
    })?;

    let theme_file: ThemeFile = serde_json::from_str(&content).map_err(|e| {
        CoreError::Other(format!("El archivo no es un theme válido: {}", e)).to_string()
    })?;

    let theme_id = build_theme_id(&theme_file.name, &theme_file.author)?;
    let theme_dir = PathManager::get().get_themes_dir().join(&theme_id);
    let _install = INSTALL_LOCK.lock();

    if theme_dir.exists() {
        error!("El theme '{}' ya existe", theme_file.name);
        return Err(CoreError::Other(format!(
            "Ya existe un theme con el nombre '{}'",
            theme_file.name
        ))
        .to_string());
    }

    let staged = StagedTheme::new(PathManager::get().get_themes_dir())?;
    let staged_dir = staged.root.join("theme");
    let dest_path = staged_dir.join("theme.json");
    std::fs::write(&dest_path, &content).map_err(|e| {
        FsError::WriteFile {
            path: dest_path.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;

    // Si el bg_image es una ruta relativa, intentar copiar el archivo
    if let Some(ref bg) = theme_file.bg_image
        && !bg.starts_with('/')
        && !bg.starts_with("file:")
        && let Some(resolved_src) = source.parent().and_then(|p| safe_join(p, bg.as_ref()).ok())
        && let Some(resolved_dest) = resolve_theme_asset(&staged_dir, bg.as_ref())?
    {
        info!("Copiando bg_image a {:?}", resolved_dest);
        if let Err(e) = std::fs::copy(&resolved_src, &resolved_dest) {
            warn!("Error copiando bg_image a {:?}: {}", resolved_dest, e);
        }
    }

    load_user_theme(&staged_dir, &theme_id)?;
    let watch = ThemeWatcher::pause_import(&theme_dir);
    staged.install(&theme_dir)?;
    watch.finish();
    info!(
        "Theme importado: id='{}', name='{}'",
        theme_id, theme_file.name
    );
    let preview = Some(extract_preview(&theme_file.variables));
    Ok(ThemeEntry {
        id: theme_id.into(),
        name: theme_file.name,
        author: theme_file.author.to_lowercase(),
        version: theme_file.version,
        r#type: "user".into(),
        preview,
        icon: None,
    })
}

#[command]
pub fn import_theme_cbth(cbth_path: String) -> Result<ThemeEntry, String> {
    info!("Importando theme CBTH desde '{}'", cbth_path);
    match import_zip_inner::<ThemeMeta>(&cbth_path)? {
        Some(entry) => Ok(entry),
        None => Err(CoreError::Other("ZIP inválido: no se encontró Meta.toml".into()).to_string()),
    }
}

#[tauri::command]
pub fn import_theme_zip(zip_path: String) -> Result<ThemeEntry, String> {
    info!("Importando theme ZIP desde '{}'", zip_path);
    match import_zip_inner::<ThemeFile>(&zip_path)? {
        Some(entry) => Ok(entry),
        None => {
            info!("No se encontró theme.json, intentando como tema V2");
            import_theme_cbth(zip_path)
        }
    }
}

#[command]
pub fn remove_theme(id: String) -> Result<(), String> {
    info!("Eliminando theme '{}'", id);
    validate_identifier(&id)?;
    let _install = INSTALL_LOCK.lock();
    let theme_dir = PathManager::get().get_themes_dir().join(&id);
    if !theme_dir.exists() {
        return Err(FsError::NotFound(theme_dir.to_string_lossy().to_string()).to_string());
    }
    std::fs::remove_dir_all(&theme_dir).map_err(|e| FsError::Remove {
        path: theme_dir.to_string_lossy().to_string(),
        source: e,
    })?;
    info!("Theme '{}' eliminado", id);
    Ok(())
}
