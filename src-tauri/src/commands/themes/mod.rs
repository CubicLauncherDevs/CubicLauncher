use crate::commands::themes::v1::{ThemeEntry, ThemeFile};
use crate::core::errors::{CoreError, FsError};
use crate::core::{AppEvent, PathManager, emit};
use crate::services::SettingsManager;
use crate::theme_watcher::ThemeWatcher;
use std::io::Read;
use tauri::command;
use tracing::{error, info, warn};
mod v1;
mod v2;

#[command]
pub fn list_themes() -> Result<Vec<ThemeEntry>, String> {
    let themes_dir = PathManager::get().get_themes_dir().to_path_buf();
    let mut themes = Vec::new();

    let entries = match std::fs::read_dir(&themes_dir) {
        Ok(e) => e,
        Err(_) => {
            info!("Directorio de themes no encontrado: {:?}", themes_dir);
            return Ok(themes);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let theme_file = path.join("theme.json");
        if !theme_file.exists() {
            continue;
        }
        let id = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };
        let content = match std::fs::read_to_string(&theme_file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let theme: ThemeFile = match serde_json::from_str(&content) {
            Ok(t) => t,
            Err(_) => continue,
        };
        themes.push(ThemeEntry {
            id: id.into(),
            name: theme.name,
            author: theme.author,
            version: theme.version,
            r#type: theme.r#type,
        });
    }

    info!("{} temas listados", themes.len());
    Ok(themes)
}

#[command]
pub fn get_user_theme(id: String) -> Result<ThemeFile, String> {
    info!("Leyendo theme '{}'", id);
    let theme_path = PathManager::get()
        .get_themes_dir()
        .join(&id)
        .join("theme.json");

    let content = std::fs::read_to_string(&theme_path).map_err(|e| {
        FsError::ReadFile {
            path: theme_path.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;

    let mut theme: ThemeFile = serde_json::from_str(&content)
        .map_err(|e| CoreError::Other(format!("Theme '{}' inválido: {}", id, e)).to_string())?;

    // Resolver bg_image relativa al directorio del theme si no es absoluta
    if let Some(ref bg) = theme.bg_image
        && !bg.starts_with('/')
        && !bg.starts_with("file:")
    {
        let abs_path = PathManager::get().get_themes_dir().join(&id).join(bg);
        theme.bg_image = Some(abs_path.to_string_lossy().to_string());
    }

    // Valida si un archivo pesa mas de 25MB
    if let Some(ref bg) = theme.bg_image
        && let Ok(meta) = std::fs::metadata(bg)
        && meta.len() > 25 * 1024 * 1024
    {
        warn!(
            "Theme '{}': bg_image demasiado grande ({} bytes), ignorando",
            id,
            meta.len()
        );
        theme.bg_image_warning_key = Some("themes.warning.largeFile".into());
        theme.bg_image = None;
    }

    // Validar magic bytes para asegurar que es una imagen
    if let Some(ref bg) = theme.bg_image {
        let is_image = std::fs::File::open(bg)
            .ok()
            .and_then(|mut f| {
                let mut buf = [0u8; 16];
                f.read_exact(&mut buf).ok()?;
                Some(infer::is_image(&buf))
            })
            .unwrap_or(false);

        if !is_image {
            warn!("Theme '{}': bg_image no es una imagen válida", id);
            theme.bg_image_warning_key = Some("themes.warning.notAnImage".into());
            theme.bg_image = None;
        }
    }

    // Resolver rutas de fuentes relativas al directorio del theme
    for font in &mut theme.fonts {
        if !font.src.starts_with('/') && !font.src.starts_with("file:") {
            let abs_path = PathManager::get()
                .get_themes_dir()
                .join(&id)
                .join(&font.src);
            font.src = abs_path.to_string_lossy().to_string().into();
        }
    }

    info!("Theme '{}' cargado exitosamente", id);
    Ok(theme)
}

#[command]
pub async fn set_theme(id: String) -> Result<(), String> {
    info!("Cambiando tema a '{}'", id);
    SettingsManager::write(|s| {
        s.theme = id.clone().into();
    })?;

    SettingsManager::save().await?;

    if let Some(dir) = id.strip_prefix("user:") {
        info!("Iniciando watcher para tema de usuario: {}", dir);
        ThemeWatcher::watch(Some(dir.to_string()));
    } else {
        info!("Tema built-in seleccionado, deteniendo watcher");
        ThemeWatcher::watch(None);
    }

    emit(AppEvent::ThemeChanged {
        id: id.clone().into(),
    });
    info!("Tema cambiado a '{}'", id);
    Ok(())
}

#[command]
pub fn get_current_theme() -> Result<String, String> {
    let theme = SettingsManager::read().theme.clone().to_string();
    info!("Tema actual: '{}'", theme);
    Ok(theme)
}

#[command]
pub fn get_themes_dir_path() -> Result<String, String> {
    let path = PathManager::get()
        .get_themes_dir()
        .to_string_lossy()
        .to_string();
    info!("Ruta de directorio de themes: {}", path);
    Ok(path)
}

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

    let theme_id = if theme_file.author.is_empty() {
        theme_file.name.to_lowercase().replace(' ', "_")
    } else {
        format!(
            "{}_{}",
            theme_file.name.to_lowercase().replace(' ', "_"),
            theme_file.author.to_lowercase().replace(' ', "_")
        )
    };
    let theme_dir = PathManager::get().get_themes_dir().join(&theme_id);

    if theme_dir.exists() {
        error!("El theme '{}' ya existe", theme_file.name);
        return Err(CoreError::Other(format!(
            "Ya existe un theme con el nombre '{}'",
            theme_file.name
        ))
        .to_string());
    }

    std::fs::create_dir_all(&theme_dir).map_err(|e| {
        FsError::CreateDir {
            path: theme_dir.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;

    let dest_path = theme_dir.join("theme.json");
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
    {
        let bg_source = source.parent().map(|p| p.join(bg));
        if let Some(bg_src) = bg_source
            && bg_src.exists()
        {
            let bg_dest = theme_dir.join(bg);
            info!("Copiando bg_image a {:?}", bg_dest);
            if let Err(e) = std::fs::copy(&bg_src, &bg_dest) {
                warn!("Error copiando bg_image a {:?}: {}", bg_dest, e);
            }
        }
    }

    info!(
        "Theme importado: id='{}', name='{}'",
        theme_id, theme_file.name
    );
    Ok(ThemeEntry {
        id: theme_id.into(),
        name: theme_file.name,
        author: theme_file.author,
        version: theme_file.version,
        r#type: "user".into(),
    })
}

#[command]
pub fn import_theme_zip(zip_path: String) -> Result<ThemeEntry, String> {
    info!("Importando theme ZIP desde '{}'", zip_path);
    let source = std::path::Path::new(&zip_path);
    if !source.exists() {
        error!("Archivo ZIP no existe: {}", zip_path);
        return Err(FsError::NotFound(zip_path.clone()).to_string());
    }

    let file = std::fs::File::open(source).map_err(|e| {
        FsError::ReadFile {
            path: zip_path.clone(),
            source: e,
        }
        .to_string()
    })?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| CoreError::Other(format!("Archivo ZIP inválido: {}", e)).to_string())?;

    // Buscar theme.json en la raíz o en un único subdirectorio
    let theme_json_name = {
        let mut found_root = false;
        let mut found_subdir: Option<String> = None;
        let mut invalid = false;

        for i in 0..archive.len() {
            let entry = archive
                .by_index(i)
                .map_err(|e| CoreError::Other(format!("Error leyendo ZIP: {}", e)).to_string())?;
            let name = entry.name().to_string();

            if name == "theme.json" {
                found_root = true;
            } else if name.ends_with("/theme.json") {
                if found_subdir.is_some() || found_root {
                    invalid = true;
                    break;
                }
                found_subdir = Some(name);
            }
        }

        if invalid || (found_root && found_subdir.is_some()) {
            return Err(
                CoreError::Other("ZIP inválido: múltiples theme.json encontrados".into())
                    .to_string(),
            );
        }

        match (found_root, found_subdir) {
            (true, _) => Some("theme.json".to_string()),
            (_, Some(sub)) => Some(sub),
            _ => None,
        }
    };

    let theme_json_name = match theme_json_name {
        Some(name) => name,
        None => {
            return Err(
                CoreError::Other("ZIP inválido: no se encontró theme.json".into()).to_string(),
            );
        }
    };

    // Leer y validar theme.json
    let theme_json_content = {
        let mut buf = String::new();
        let mut entry = archive.by_name(&theme_json_name).map_err(|e| {
            CoreError::Other(format!("Error leyendo theme.json: {}", e)).to_string()
        })?;
        entry.read_to_string(&mut buf).map_err(|e| {
            CoreError::Other(format!("Error leyendo theme.json: {}", e)).to_string()
        })?;
        buf
    };

    let theme_file: ThemeFile = serde_json::from_str(&theme_json_content)
        .map_err(|e| CoreError::Other(format!("theme.json inválido: {}", e)).to_string())?;

    let theme_id = if theme_file.author.is_empty() {
        theme_file.name.to_lowercase().replace(' ', "_")
    } else {
        format!(
            "{}_{}",
            theme_file.name.to_lowercase().replace(' ', "_"),
            theme_file.author.to_lowercase().replace(' ', "_")
        )
    };
    let theme_dir = PathManager::get().get_themes_dir().join(&theme_id);

    // Sobreescribir si ya existe
    if theme_dir.exists() {
        info!("Sobreescribiendo theme existente '{}'", theme_id);
        if let Err(e) = std::fs::remove_dir_all(&theme_dir) {
            error!("Error eliminando theme existente '{}': {}", theme_id, e);
            return Err(FsError::Remove {
                path: theme_dir.to_string_lossy().to_string(),
                source: e,
            }
            .to_string());
        }
    }

    std::fs::create_dir_all(&theme_dir).map_err(|e| {
        FsError::CreateDir {
            path: theme_dir.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;

    // Determinar el prefijo del directorio dentro del ZIP (si lo hay)
    let prefix = if theme_json_name == "theme.json" {
        String::new()
    } else {
        // "some_dir/theme.json" → "some_dir/"
        theme_json_name
            .strip_suffix("theme.json")
            .unwrap_or("")
            .to_string()
    };

    // Extraer todos los archivos del ZIP
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CoreError::Other(format!("Error leyendo ZIP: {}", e)).to_string())?;

        let entry_name = entry.name().to_string();

        // Saltar directorios y el prefijo
        let relative = match entry_name.strip_prefix(&prefix) {
            Some(r) => r.to_string(),
            None => continue,
        };

        if relative.is_empty() || relative.ends_with('/') {
            continue;
        }

        let out_path = theme_dir.join(&relative);

        // Crear directorio padre si necesario
        if let Some(parent) = out_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            warn!("Error creando directorio {:?}: {}", parent, e);
            continue;
        }

        let mut out_file = std::fs::File::create(&out_path).map_err(|e| {
            FsError::WriteFile {
                path: out_path.to_string_lossy().to_string(),
                source: e,
            }
            .to_string()
        })?;

        std::io::copy(&mut entry, &mut out_file).map_err(|e| {
            FsError::WriteFile {
                path: out_path.to_string_lossy().to_string(),
                source: e,
            }
            .to_string()
        })?;
    }

    info!(
        "Theme ZIP importado: id='{}', name='{}'",
        theme_id, theme_file.name
    );
    Ok(ThemeEntry {
        id: theme_id.into(),
        name: theme_file.name,
        author: theme_file.author,
        version: theme_file.version,
        r#type: "user".into(),
    })
}
