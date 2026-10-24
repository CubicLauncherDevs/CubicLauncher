use super::FontFace;
use super::assets::{
    BgValidationError, resolve_theme_asset, validate_background_image, validate_theme_icon,
};
use super::v1::ThemeFile;
use super::v2::{Icons, read_definition, read_meta};
use serde::Serialize;
use std::path::Path;

const MAX_CSS_SIZE: u64 = 256 * 1024;
const MAX_FONTS: usize = 12;
const MAX_ICONS: usize = 100;

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeValidationReport {
    pub id: String,
    pub name: Option<String>,
    pub format: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ThemeValidationReport {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: None,
            format: "unknown".to_string(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    fn warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }
}

pub(super) fn validate_theme_at(theme_base: &Path, id: &str) -> ThemeValidationReport {
    let mut report = ThemeValidationReport::new(id);
    let has_v2 = theme_base.join("Meta.toml").is_file();
    let has_v1 = theme_base.join("theme.json").is_file();

    match (has_v2, has_v1) {
        (true, true) => {
            report.warning("Existen Meta.toml y theme.json; se usará Meta.toml (V2).");
            report.format = "v2".to_string();
            validate_v2(theme_base, &mut report);
        }
        (true, false) => {
            report.format = "v2".to_string();
            validate_v2(theme_base, &mut report);
        }
        (false, true) => {
            report.format = "v1".to_string();
            report.warning(
                "El formato V1 es legacy y solo se mantiene por compatibilidad. Migra a V2 (Meta.toml + Definition.toml).",
            );
            validate_v1(theme_base, &theme_base.join("theme.json"), &mut report);
        }
        (false, false) => {
            report.error("No se encontró Meta.toml ni theme.json en el tema.");
        }
    }

    report
}

fn validate_v2(theme_base: &Path, report: &mut ThemeValidationReport) {
    let meta = match read_meta(theme_base) {
        Ok(meta) => meta,
        Err(error) => {
            report.error(format!("Meta.toml inválido: {error}"));
            return;
        }
    };
    report.name = Some(meta.name.to_string());

    if meta.name.trim().is_empty() {
        report.error("Meta.toml: 'name' no puede estar vacío.");
    }
    if meta.author.trim().is_empty() {
        report.warning("Meta.toml: 'author' está vacío.");
    }
    if meta.version.trim().is_empty() {
        report.warning("Meta.toml: 'version' está vacío.");
    }

    let theme = match read_definition(theme_base) {
        Ok(theme) => theme,
        Err(error) => {
            report.error(format!("Definition.toml inválido: {error}"));
            return;
        }
    };

    validate_background(
        theme_base,
        theme.background.reference_path.as_deref(),
        report,
    );
    validate_fonts(theme_base, &theme.fonts, report);
    validate_icons(theme_base, &theme.icons, report);
    validate_inject(theme_base, meta.injects_css, report);
}

fn validate_v1(theme_base: &Path, manifest: &Path, report: &mut ThemeValidationReport) {
    let content = match std::fs::read_to_string(manifest) {
        Ok(content) => content,
        Err(error) => {
            report.error(format!("No se pudo leer {}: {error}", manifest.display()));
            return;
        }
    };
    let theme: ThemeFile = match serde_json::from_str(&content) {
        Ok(theme) => theme,
        Err(error) => {
            report.error(format!("{} inválido: {error}", manifest.display()));
            return;
        }
    };

    report.name = Some(theme.name.to_string());
    if theme.name.trim().is_empty() {
        report.error(format!(
            "{}: 'name' no puede estar vacío.",
            manifest.display()
        ));
    }
    if theme.author.trim().is_empty() {
        report.warning(format!("{}: 'author' está vacío.", manifest.display()));
    }

    validate_background(theme_base, theme.bg_image.as_deref(), report);
    validate_fonts(theme_base, &theme.fonts, report);
}

fn validate_background(
    theme_base: &Path,
    reference: Option<&str>,
    report: &mut ThemeValidationReport,
) {
    let Some(reference) = reference else {
        return;
    };
    if reference.trim().is_empty() {
        report.warning("El fondo declara una ruta vacía; se ignorará.");
        return;
    }
    warn_absolute_reference("El fondo", reference, report);

    let resolved = match resolve_theme_asset(theme_base, reference) {
        Ok(Some(path)) => path,
        Ok(None) => return,
        Err(error) => {
            report.error(format!("La ruta del fondo es inválida: {error}"));
            return;
        }
    };
    if !resolved.is_file() {
        report.warning(format!(
            "No se encontró el fondo '{}'; se ignorará.",
            display_relative(theme_base, &resolved)
        ));
        return;
    }

    match validate_background_image(&resolved) {
        Ok(()) => {}
        Err(BgValidationError::TooLarge) => {
            report.warning("El fondo supera el tamaño máximo permitido; se ignorará.");
        }
        Err(BgValidationError::InvalidImage) => {
            report.warning("El fondo no es una imagen válida; se ignorará.");
        }
        Err(BgValidationError::Io) => {
            report.warning("No se pudo leer el fondo; se ignorará.");
        }
    }
}

fn validate_fonts(theme_base: &Path, fonts: &[FontFace], report: &mut ThemeValidationReport) {
    if fonts.len() > MAX_FONTS {
        report.warning(format!(
            "El tema declara {} fuentes; se recomienda no superar {MAX_FONTS}.",
            fonts.len()
        ));
    }

    for font in fonts {
        let family = font.family.trim();
        if family.is_empty() {
            report.error("Una fuente no declara 'family'.");
        }
        if font.src.trim().is_empty() {
            report.error(format!("La fuente '{family}' no declara 'src'."));
            continue;
        }
        warn_absolute_reference(&format!("La fuente '{family}'"), &font.src, report);

        match resolve_theme_asset(theme_base, &font.src) {
            Ok(Some(path)) => {
                if !path.is_file() {
                    report.warning(format!(
                        "No se encontró la fuente '{}': {}",
                        family,
                        display_relative(theme_base, &path)
                    ));
                }
            }
            Ok(None) => {}
            Err(error) => {
                report.error(format!(
                    "La fuente '{family}' tiene una ruta inválida: {error}"
                ));
            }
        }
    }
}

fn validate_icons(theme_base: &Path, icons: &Icons, report: &mut ThemeValidationReport) {
    let mut count = 0;

    if let Some(preview) = icons.preview.as_deref() {
        count += 1;
        validate_icon_path(theme_base, "preview", preview, report);
    }

    for (group, items) in &icons.groups {
        for (name, path) in items {
            count += 1;
            validate_icon_path(theme_base, &format!("{group}:{name}"), path, report);
        }
    }

    if count > MAX_ICONS {
        report.warning(format!(
            "El tema declara {count} iconos; solo se precargarán los primeros {MAX_ICONS}."
        ));
    }
}

fn validate_icon_path(
    theme_base: &Path,
    key: &str,
    reference: &str,
    report: &mut ThemeValidationReport,
) {
    if reference.trim().is_empty() {
        report.warning(format!(
            "El icono '{key}' declara una ruta vacía; se ignorará."
        ));
        return;
    }
    warn_absolute_reference(&format!("El icono '{key}'"), reference, report);

    let resolved = match resolve_theme_asset(theme_base, reference) {
        Ok(Some(path)) => path,
        Ok(None) => return,
        Err(error) => {
            report.error(format!("El icono '{key}' tiene una ruta inválida: {error}"));
            return;
        }
    };
    if !resolved.is_file() {
        report.warning(format!(
            "No se encontró el icono '{key}': {}",
            display_relative(theme_base, &resolved)
        ));
        return;
    }
    if !validate_theme_icon(&resolved.to_string_lossy()) {
        report.warning(format!(
            "El icono '{key}' no es válido o usa un formato no soportado: {}",
            display_relative(theme_base, &resolved)
        ));
    }
}

fn validate_inject(theme_base: &Path, injects_css: bool, report: &mut ThemeValidationReport) {
    let path = theme_base.join("Inject.css");
    if !path.exists() {
        return;
    }
    if !injects_css {
        report.warning(
            "Meta.toml: 'injects_css' es false pero Inject.css existe; actualmente se carga igual.",
        );
    }
    match std::fs::metadata(&path) {
        Ok(metadata) if metadata.len() > MAX_CSS_SIZE => {
            report.warning(format!(
                "Inject.css ocupa {} KiB; se recomienda no superar {} KiB.",
                metadata.len() / 1024,
                MAX_CSS_SIZE / 1024
            ));
        }
        Ok(_) => {}
        Err(error) => {
            report.warning(format!("No se pudo leer Inject.css: {error}"));
        }
    }
}

fn warn_absolute_reference(label: &str, reference: &str, report: &mut ThemeValidationReport) {
    if reference.starts_with('/') || reference.starts_with("file:") {
        report.warning(format!(
            "{label} usa una ruta absoluta o file:; se recomienda una ruta relativa al tema."
        ));
    }
}

fn display_relative(theme_base: &Path, path: &Path) -> String {
    path.strip_prefix(theme_base)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
#[path = "../../tests/commands/themes/validate.rs"]
mod tests;
