use super::FontFace;
use super::assets::{
    BgValidationError, resolve_theme_asset, validate_background_image, validate_theme_icon,
};
use super::v1::ThemeFile;
use super::v2::{
    Icons, MAX_EXTENDS_DEPTH, ThemeDef, ThemeMeta, flatten_with_diagnostics, is_valid_token_name,
    read_definition, read_meta,
};
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
            validate_v2(theme_base, id, &mut report);
        }
        (true, false) => {
            report.format = "v2".to_string();
            validate_v2(theme_base, id, &mut report);
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

fn validate_v2(theme_base: &Path, id: &str, report: &mut ThemeValidationReport) {
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
    if meta.format != 2 {
        report.error(format!(
            "Meta.toml: 'format' {} no soportado; se esperaba 2.",
            meta.format
        ));
    }
    if let Some(api_version) = meta.api_version.as_deref()
        && api_version != "2"
        && !api_version.starts_with("2.")
    {
        report.warning(format!(
            "Meta.toml: 'api_version' '{}' no pertenece a la serie 2.x.",
            api_version
        ));
    }
    warn_unknown_meta(&meta, report);

    let theme = match read_definition(theme_base) {
        Ok(theme) => theme,
        Err(error) => {
            report.error(format!("Definition.toml inválido: {error}"));
            return;
        }
    };

    warn_unknown_definition(&theme, report);
    validate_tokens(&theme, report);
    validate_background(
        theme_base,
        theme.background.reference_path.as_deref(),
        report,
    );
    validate_fonts(theme_base, &theme.fonts, report);
    validate_icons(theme_base, &theme.icons, report);
    validate_inject(theme_base, &meta, &theme, report);
    validate_layout_pack(theme_base, &theme, report);
    validate_extends(theme_base, id, &meta, report);
}

fn warn_unknown_meta(meta: &ThemeMeta, report: &mut ThemeValidationReport) {
    for key in meta.extra.keys() {
        report.warning(format!("Meta.toml: campo desconocido '{}'.", key));
    }
}

fn warn_unknown_definition(theme: &ThemeDef, report: &mut ThemeValidationReport) {
    for key in theme.extra.keys() {
        report.warning(format!("Definition.toml: campo desconocido '{}'.", key));
    }
    for key in theme.background.extra.keys() {
        report.warning(format!(
            "Definition.toml: campo desconocido 'background.{}'.",
            key
        ));
    }
    for key in theme.typography.extra.keys() {
        report.warning(format!(
            "Definition.toml: campo desconocido 'typography.{}'.",
            key
        ));
    }
    if let Some(inject) = theme.inject.as_ref() {
        for key in inject.extra.keys() {
            report.warning(format!(
                "Definition.toml: campo desconocido 'inject.{}'.",
                key
            ));
        }
    }
}

fn validate_tokens(theme: &ThemeDef, report: &mut ThemeValidationReport) {
    for key in theme.tokens.keys() {
        if !is_valid_token_name(key) {
            report.error(format!(
                "Definition.toml: token '{}' inválido; debe empezar por '--' y usar solo letras, números, '_' o '-'.",
                key
            ));
        }
    }

    for warning in flatten_with_diagnostics(theme).warnings {
        if warning.starts_with("Token '") {
            continue;
        }
        report.warning(warning);
    }
}

fn validate_layout_pack(
    theme_base: &Path,
    theme: &ThemeDef,
    report: &mut ThemeValidationReport,
) {
    let Some(config) = theme.layout_pack.as_ref() else {
        return;
    };

    for key in config.extra.keys() {
        report.warning(format!(
            "Definition.toml: campo desconocido 'layout_pack.{}'.",
            key
        ));
    }
    if config.enabled == Some(false) {
        return;
    }

    let path = config.path.as_deref().unwrap_or("Layout.css");
    if path.trim().is_empty() {
        report.warning("Definition.toml: [layout_pack].path está vacío; no se cargará CSS.");
        return;
    }
    warn_absolute_reference("[layout_pack].path", path, report);

    match resolve_theme_asset(theme_base, path) {
        Ok(Some(resolved)) => {
            if !resolved.is_file() {
                report.warning(format!(
                    "No se encontró el layout pack '{}'; se ignorará.",
                    display_relative(theme_base, &resolved)
                ));
                return;
            }
            validate_css_size(&resolved, report);
        }
        Ok(None) => {}
        Err(error) => {
            report.error(format!(
                "[layout_pack].path tiene una ruta inválida: {error}"
            ));
        }
    }
}

fn validate_extends(
    theme_base: &Path,
    id: &str,
    meta: &ThemeMeta,
    report: &mut ThemeValidationReport,
) {
    let Some(base_id) = meta
        .extends
        .as_deref()
        .map(str::trim)
        .filter(|base| !base.is_empty())
    else {
        return;
    };

    let Some(themes_root) = theme_base.parent() else {
        report.error("Meta.toml: no se pudo resolver el directorio de temas para 'extends'.");
        return;
    };

    let mut chain = vec![id.to_string()];
    let mut current = base_id.strip_prefix("user:").unwrap_or(base_id).to_string();
    let mut depth = 0;

    loop {
        if chain.iter().any(|seen| seen == &current) {
            chain.push(current.clone());
            report.error(format!("Ciclo de herencia: {}.", chain.join(" -> ")));
            return;
        }
        if depth >= MAX_EXTENDS_DEPTH {
            report.error(format!(
                "Herencia demasiado profunda: '{}' supera el máximo de {MAX_EXTENDS_DEPTH} niveles.",
                chain.join(" -> ")
            ));
            return;
        }
        if let Err(error) = crate::core::validate_identifier(&current) {
            report.error(format!("Meta.toml: 'extends' inválido: {error}."));
            return;
        }

        let base_dir = themes_root.join(&current);
        if !base_dir.join("Meta.toml").is_file() {
            report.error(format!(
                "Meta.toml: el tema base '{current}' no está instalado o no es V2."
            ));
            return;
        }
        chain.push(current.clone());

        let base_meta = match read_meta(&base_dir) {
            Ok(base_meta) => base_meta,
            Err(error) => {
                report.error(format!("Meta.toml del tema base '{current}' inválido: {error}"));
                return;
            }
        };
        let Some(next) = base_meta
            .extends
            .as_deref()
            .map(str::trim)
            .filter(|base| !base.is_empty())
        else {
            return;
        };
        current = next.strip_prefix("user:").unwrap_or(next).to_string();
        depth += 1;
    }
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

fn validate_inject(
    theme_base: &Path,
    meta: &ThemeMeta,
    theme: &ThemeDef,
    report: &mut ThemeValidationReport,
) {
    if let Some(config) = theme.inject.as_ref() {
        if let Some(layer) = config.layer.as_deref()
            && layer != "theme"
            && layer != "unlayered"
        {
            report.warning(format!(
                "Definition.toml: [inject].layer '{}' inválido; usa 'theme' o 'unlayered'.",
                layer
            ));
        }

        let enabled = config.enabled.or(meta.injects_css).unwrap_or(true);
        if !enabled {
            return;
        }

        let path = config.path.as_deref().unwrap_or("Inject.css");
        validate_inject_path(theme_base, path, report);
        return;
    }

    let path = theme_base.join("Inject.css");
    if !path.exists() {
        return;
    }
    report.warning(
        "Inject.css se carga por compatibilidad; añade [inject] en Definition.toml para controlar su carga y capa.",
    );
    validate_css_size(&path, report);
}

fn validate_inject_path(theme_base: &Path, reference: &str, report: &mut ThemeValidationReport) {
    if reference.trim().is_empty() {
        report.warning("Definition.toml: [inject].path está vacío; no se cargará CSS.");
        return;
    }
    warn_absolute_reference("[inject].path", reference, report);

    let resolved = match resolve_theme_asset(theme_base, reference) {
        Ok(Some(path)) => path,
        Ok(None) => return,
        Err(error) => {
            report.error(format!("[inject].path tiene una ruta inválida: {error}"));
            return;
        }
    };
    if !resolved.is_file() {
        report.warning(format!(
            "No se encontró el CSS de [inject]: {}",
            display_relative(theme_base, &resolved)
        ));
        return;
    }
    validate_css_size(&resolved, report);
}

fn validate_css_size(path: &Path, report: &mut ThemeValidationReport) {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Inject.css");
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() > MAX_CSS_SIZE => {
            report.warning(format!(
                "{} ocupa {} KiB; se recomienda no superar {} KiB.",
                name,
                metadata.len() / 1024,
                MAX_CSS_SIZE / 1024
            ));
        }
        Ok(_) => {}
        Err(error) => {
            report.warning(format!("No se pudo leer {}: {error}", name));
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
