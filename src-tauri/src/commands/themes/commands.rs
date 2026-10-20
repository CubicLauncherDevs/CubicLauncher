use super::v1::{ThemeEntry, ThemeFile};
use super::v2::V2Theme;
use super::{Theme, ThemeResponse, load_user_theme};
use crate::core::{PathManager, validate_identifier};
use crate::services::SettingsManager;
use tauri::command;
use tracing::info;

#[command]
pub fn list_themes() -> Result<Vec<ThemeEntry>, String> {
    let themes_dir = PathManager::get().get_themes_dir();
    let mut themes = Vec::new();

    let entries = match std::fs::read_dir(themes_dir) {
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
        let Some(id) = path.file_name().map(|name| name.to_string_lossy()) else {
            continue;
        };
        let theme = if path.join("theme.json").exists() {
            ThemeFile::list_entry(&path, &id)
        } else if path.join("Meta.toml").exists() {
            V2Theme::list_entry(&path, &id)
        } else {
            None
        };
        if let Some(theme) = theme {
            themes.push(theme);
        }
    }

    info!("{} temas listados", themes.len());
    Ok(themes)
}

#[command]
pub fn get_user_theme(id: String) -> Result<ThemeResponse, String> {
    info!("Leyendo theme '{}'", id);
    validate_identifier(&id)?;
    let theme_base = PathManager::get().get_themes_dir().join(&id);
    load_user_theme(&theme_base, &id)
}

#[command]
pub async fn set_theme(id: String) -> Result<(), String> {
    info!("Cambiando tema a '{}'", id);
    SettingsManager::write(|s| {
        s.theme = id.clone().into();
    })?;

    SettingsManager::save().await?;

    info!("Tema cambiado a '{}'", id);
    Ok(())
}

#[command]
pub fn get_current_theme() -> Result<String, String> {
    let theme = SettingsManager::read().theme.to_string();
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
