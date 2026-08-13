//! Migración de datos de juego desde instancias MultiMC / Prism Launcher.

use super::ImportError;
use std::path::{Path, PathBuf};

/// Datos de Minecraft que migraremos desde el directorio de juego.
const FOLDERS_TO_MIGRATE: &[&str] = &[
    "mods",
    "resourcepacks",
    "shaderpacks",
    "saves",
    "config",
    "scripts",
    "defaultconfigs",
    "kubejs",
    "options.txt",
    "servers.dat",
];

/// Devuelve el directorio de juego real dentro de la instancia extraída.
pub fn resolve_game_dir(instance_path: &Path) -> PathBuf {
    let dot_minecraft = instance_path.join(".minecraft");
    if dot_minecraft.exists() {
        dot_minecraft
    } else {
        instance_path.join("minecraft")
    }
}

/// Migra carpetas y archivos relevantes del directorio de juego fuente al destino.
pub async fn migrate_game_data(source: &Path, target: &Path) -> Result<(), ImportError> {
    for relative in FOLDERS_TO_MIGRATE {
        let src = source.join(relative);
        if !src.exists() {
            continue;
        }
        let dst = target.join(relative);

        if src.is_dir() {
            copy_dir_recursively(&src, &dst).await?;
        } else if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(ImportError::Io)?;
            tokio::fs::copy(&src, &dst)
                .await
                .map_err(|e| ImportError::ProviderError {
                    provider: "MultiMC".into(),
                    message: format!("Error copiando {:?} a {:?}: {e}", src, dst),
                })?;
        }
    }
    Ok(())
}

async fn copy_dir_recursively(src: &Path, dst: &Path) -> Result<(), ImportError> {
    tokio::fs::create_dir_all(dst)
        .await
        .map_err(|e| ImportError::ProviderError {
            provider: "MultiMC".into(),
            message: format!("Error creando directorio {:?}: {e}", dst),
        })?;

    let mut entries = tokio::fs::read_dir(src)
        .await
        .map_err(|e| ImportError::ProviderError {
            provider: "MultiMC".into(),
            message: format!("Error leyendo directorio {:?}: {e}", src),
        })?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let dest = dst.join(entry.file_name());

        if path.is_dir() {
            Box::pin(copy_dir_recursively(&path, &dest)).await?;
        } else {
            tokio::fs::copy(&path, &dest)
                .await
                .map_err(|e| ImportError::ProviderError {
                    provider: "MultiMC".into(),
                    message: format!("Error copiando {:?} a {:?}: {e}", path, dest),
                })?;
        }
    }

    Ok(())
}

/// Resuelve la ruta del icono de la instancia si existe.
pub fn resolve_icon_path(instance_path: &Path, icon_key: &Option<String>) -> Option<PathBuf> {
    let game_icon = resolve_game_dir(instance_path).join("icon.png");
    if game_icon.is_file() {
        return Some(game_icon);
    }

    if let Some(key) = icon_key
        && key != "default"
        && key != "logo"
    {
        let custom = instance_path.join(format!("{}.png", key));
        if custom.is_file() {
            return Some(custom);
        }
    }

    None
}
