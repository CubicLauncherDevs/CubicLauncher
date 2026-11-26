//! Nombre y migración de la carpeta de paquetes de texturas según la versión.
//!
//! Las versiones anteriores a 1.6 (snapshot `13w23a`) usan `texturepacks`;
//! desde 1.6 el juego lee `resourcepacks`.

use std::path::Path;

use tracing::info;

pub const RESOURCE_PACKS: &str = "resourcepacks";
pub const TEXTURE_PACKS: &str = "texturepacks";

/// Indica si `version` usa `texturepacks` en lugar de `resourcepacks`.
pub fn uses_texture_packs(version: &str) -> bool {
    zellkern::is_pre_1_6(version)
}

/// Traduce un subdirectorio lógico (`resourcepacks`, `mods`, …) al real.
pub fn physical_pack_dir<'a>(version: &str, subdir: &'a str) -> &'a str {
    if subdir == RESOURCE_PACKS && uses_texture_packs(version) {
        TEXTURE_PACKS
    } else {
        subdir
    }
}

/// Mueve los packs de `resourcepacks` a `texturepacks` en instancias legacy.
///
/// Es idempotente, nunca sobrescribe archivos y no toca enlaces simbólicos.
pub async fn migrate_legacy_packs(instance_dir: &Path, version: &str) -> Result<(), String> {
    if !uses_texture_packs(version) {
        return Ok(());
    }
    let root = instance_dir.to_path_buf();
    tokio::task::spawn_blocking(move || migrate_blocking(&root))
        .await
        .map_err(|e| format!("La migración de texturepacks falló: {e}"))?
}

fn migrate_blocking(instance_dir: &Path) -> Result<(), String> {
    let src = instance_dir.join(RESOURCE_PACKS);
    let dst = instance_dir.join(TEXTURE_PACKS);

    let meta = match std::fs::symlink_metadata(&src) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("No se pudo leer {:?}: {e}", src)),
    };
    if !meta.file_type().is_dir() {
        return Ok(());
    }

    match std::fs::symlink_metadata(&dst) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::rename(&src, &dst)
                .map_err(|e| format!("No se pudo mover {:?} a {:?}: {e}", src, dst))?;
            info!("Texturepacks migrados: {:?} -> {:?}", src, dst);
            return Ok(());
        }
        Err(e) => return Err(format!("No se pudo leer {:?}: {e}", dst)),
        Ok(meta) if !meta.file_type().is_dir() => {
            return Err(format!("{:?} existe y no es una carpeta", dst));
        }
        Ok(_) => {}
    }

    for entry in std::fs::read_dir(&src).map_err(|e| format!("No se pudo leer {:?}: {e}", src))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = dst.join(entry.file_name());
        if !target.exists() {
            std::fs::rename(entry.path(), &target)
                .map_err(|e| format!("No se pudo mover {:?} a {:?}: {e}", entry.path(), target))?;
        }
    }

    if std::fs::read_dir(&src)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
    {
        let _ = std::fs::remove_dir(&src);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/services/pack_dir.rs"]
mod tests;
