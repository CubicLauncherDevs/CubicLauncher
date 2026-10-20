use super::FontFace;
use super::v1::ThemePreview;
use crate::core::sanitize_path;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Resuelve una referencia relativa en base al directorio del theme.
pub(super) fn resolve_theme_asset(
    theme_base: &Path,
    reference: &str,
) -> Result<Option<PathBuf>, String> {
    if reference.starts_with('/') || reference.starts_with("file:") {
        return Ok(Some(PathBuf::from(reference)));
    }
    if reference.is_empty() {
        return Ok(None);
    }
    Ok(Some(sanitize_path(theme_base, Path::new(reference))?))
}

pub(super) fn extract_preview(vars: &HashMap<String, String>) -> ThemePreview {
    ThemePreview {
        bg: vars
            .get("--bg-main")
            .or_else(|| vars.get("--bg-card"))
            .or_else(|| vars.get("--bg-sidebar"))
            .cloned()
            .unwrap_or_else(|| "#0c0c0c".into()),
        accent: vars
            .get("--accent")
            .cloned()
            .unwrap_or_else(|| "#ffffff".into()),
        text: vars
            .get("--text-primary")
            .cloned()
            .unwrap_or_else(|| "#d8d8d8".into()),
    }
}

const MAX_ICON_SIZE: u64 = 2 * 1024 * 1024;
const MAX_BG_SIZE: u64 = 25 * 1024 * 1024;

#[derive(Debug)]
pub(super) enum BgValidationError {
    TooLarge,
    InvalidImage,
    Io,
}

fn validate_background_image(path: &std::path::Path) -> Result<(), BgValidationError> {
    let mut file = std::fs::File::open(path).map_err(|_| BgValidationError::Io)?;
    let meta = file.metadata().map_err(|_| BgValidationError::Io)?;
    if meta.len() > MAX_BG_SIZE {
        return Err(BgValidationError::TooLarge);
    }

    let mut buf = [0u8; 16];
    file.read_exact(&mut buf)
        .map_err(|_| BgValidationError::Io)?;

    if infer::is_image(&buf) {
        Ok(())
    } else {
        Err(BgValidationError::InvalidImage)
    }
}

pub(super) fn resolve_background(
    theme_base: &Path,
    background: &mut Option<String>,
) -> Result<Option<BgValidationError>, String> {
    if let Some(path) = background.as_ref()
        && let Some(resolved) = resolve_theme_asset(theme_base, path)?
    {
        *background = Some(resolved.to_string_lossy().into_owned());
    }
    let Some(path) = background.as_ref() else {
        return Ok(None);
    };
    match validate_background_image(Path::new(path)) {
        Ok(()) => Ok(None),
        Err(error) => {
            *background = None;
            Ok(Some(error))
        }
    }
}

pub(super) fn resolve_fonts(theme_base: &Path, fonts: &mut [FontFace]) -> Result<(), String> {
    for font in fonts {
        if let Some(resolved) = resolve_theme_asset(theme_base, font.src.as_ref())? {
            font.src = resolved.to_string_lossy().to_string().into();
        }
    }
    Ok(())
}

pub(super) fn validate_theme_icon(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("svg") => std::fs::metadata(path).is_ok(),
        Some("png" | "webp" | "jpg" | "jpeg") => {
            if let Ok(meta) = std::fs::metadata(path)
                && meta.len() > MAX_ICON_SIZE
            {
                warn!("Icono demasiado grande ({} bytes): {}", meta.len(), path);
                return false;
            }
            let is_image = std::fs::File::open(path)
                .ok()
                .and_then(|mut f| {
                    let mut buf = [0u8; 16];
                    f.read_exact(&mut buf).ok()?;
                    Some(infer::is_image(&buf))
                })
                .unwrap_or(false);
            if !is_image {
                warn!("Icono no es una imagen válida: {}", path);
            }
            is_image
        }
        _ => {
            warn!("Extensión de icono no soportada: {}", path);
            false
        }
    }
}
