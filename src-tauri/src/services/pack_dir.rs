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
    let base = zellkern::GameVersion::from_version_id(version).mc_version;
    let base = base.trim().to_ascii_lowercase();

    let legacy_prefix = matches!(base.as_bytes().first(), Some(b'a' | b'b' | b'c'))
        && base.as_bytes().get(1).is_some_and(u8::is_ascii_digit);
    if legacy_prefix || base.starts_with("rd-") || base.starts_with("inf-") {
        return true;
    }

    if let Some((year, week)) = snapshot_year_week(&base) {
        return (year, week) < (13, 23);
    }

    release_major_minor(&base).is_some_and(|(major, minor)| (major, minor) < (1, 6))
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

fn snapshot_year_week(version: &str) -> Option<(u32, u32)> {
    let (year, rest) = version.split_once('w')?;
    let year: u32 = year.parse().ok()?;
    if !(10..=99).contains(&year) {
        return None;
    }
    let week: u32 = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()?;
    Some((year, week))
}

fn release_major_minor(version: &str) -> Option<(u32, u32)> {
    if !version.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts
        .next()?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some((major, minor))
}

#[cfg(test)]
#[path = "../tests/services/pack_dir.rs"]
mod tests;
