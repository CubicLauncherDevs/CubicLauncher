use super::launch::validate_uuid;
use crate::services::{
    InstanceManager,
    minecraft_jar::{self, JarMod, MinecraftJarConfig},
};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JarAction {
    Add { paths: Vec<PathBuf> },
    Replace { path: PathBuf },
    Restore,
    Remove { file: String },
    Toggle { file: String, enabled: bool },
    Move { file: String, offset: i32 },
}

#[tauri::command]
pub async fn get_instance_minecraft_jar(instance_id: String) -> Result<MinecraftJarConfig, String> {
    validate_uuid(&instance_id)?;
    let handle = InstanceManager::get()
        .get_handle(&instance_id)
        .await
        .ok_or("Instancia no encontrada")?;
    Ok(handle.get_minecraft_jar().await)
}

#[tauri::command]
pub async fn instance_minecraft_jar_action(
    instance_id: String,
    action: JarAction,
) -> Result<MinecraftJarConfig, String> {
    validate_uuid(&instance_id)?;
    let handle = InstanceManager::get()
        .get_handle(&instance_id)
        .await
        .ok_or("Instancia no encontrada")?;
    let guard = handle.try_lock_files()?;
    if handle.is_busy() {
        return Err("Cierra Minecraft antes de modificar minecraft.jar".into());
    }
    let dir = handle.get_instance_dir().await;
    let old = handle.get_minecraft_jar().await;
    let mut config = old.clone();
    let work_dir = dir.clone();
    let (guard, result) = tokio::task::spawn_blocking(move || {
        let result = apply_action(&work_dir, &mut config, action).map(|()| config);
        (guard, result)
    })
    .await
    .map_err(|e| e.to_string())?;
    let config = result?;
    if let Err(error) = handle.save_minecraft_jar(config.clone()).await {
        cleanup_removed(&dir, &config, &old);
        return Err(error);
    }
    cleanup_removed(&dir, &old, &config);
    drop(guard);
    Ok(config)
}

fn cleanup_removed(dir: &std::path::Path, old: &MinecraftJarConfig, new: &MinecraftJarConfig) {
    for file in old.files() {
        if !new.files().any(|f| f.file == file.file) {
            let _ = std::fs::remove_file(dir.join(minecraft_jar::INPUT_DIR).join(&file.file));
        }
    }
}

fn apply_action(
    dir: &std::path::Path,
    config: &mut MinecraftJarConfig,
    action: JarAction,
) -> Result<(), String> {
    config.validate()?;
    match action {
        JarAction::Add { paths } => {
            let mut added = Vec::new();
            for path in paths {
                match minecraft_jar::import_file(dir, &path, false) {
                    Ok(archive) => added.push(JarMod {
                        archive,
                        enabled: true,
                    }),
                    Err(error) => {
                        for item in added {
                            let _ = std::fs::remove_file(
                                dir.join(minecraft_jar::INPUT_DIR).join(item.archive.file),
                            );
                        }
                        return Err(error);
                    }
                }
            }
            config.mods.extend(added);
        }
        JarAction::Replace { path } => {
            config.replacement = Some(minecraft_jar::import_file(dir, &path, true)?)
        }
        JarAction::Restore => config.replacement = None,
        JarAction::Remove { file } => {
            let index = mod_index(config, &file)?;
            config.mods.remove(index);
        }
        JarAction::Toggle { file, enabled } => {
            let index = mod_index(config, &file)?;
            config.mods[index].enabled = enabled;
        }
        JarAction::Move { file, offset } => {
            let index = mod_index(config, &file)?;
            if !matches!(offset, -1 | 1) {
                return Err("Movimiento inválido".into());
            }
            let target = index as i64 + i64::from(offset);
            if target < 0 || target >= config.mods.len() as i64 {
                return Err("Posición fuera de la lista".into());
            }
            config.mods.swap(index, target as usize);
        }
    }
    Ok(())
}

fn mod_index(config: &MinecraftJarConfig, file: &str) -> Result<usize, String> {
    config
        .mods
        .iter()
        .position(|m| m.archive.file == file)
        .ok_or("Mod de JAR no encontrado".into())
}

#[cfg(test)]
#[path = "../../tests/commands/instance/minecraft_jar.rs"]
mod tests;
