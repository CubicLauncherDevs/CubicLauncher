use super::FontFace;
use super::assets::{
    BgValidationError, extract_preview, resolve_background, resolve_fonts, resolve_theme_asset,
    validate_theme_icon,
};
use super::v1::{ThemeEntry, ThemePreview};
use super::{Theme, ZipImportable};
use crate::core::errors::{CoreError, FsError};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

/// Estructura que representa los metadatos de segunda version del
/// sistema de Themes. Implementa el Trait `Theme` el cual es el
/// que lo permite ser parseado como tal. Estos se traducen a
/// un struct intermedio el cual el frontend carga.
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct V2Theme {
    pub meta: ThemeMeta,
    pub theme: ThemeDef,
}

fn default_format() -> u32 {
    2
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Estructura que guarda los metadatos de un theme
///
pub struct ThemeMeta {
    pub name: CompactString,
    #[serde(default)]
    pub author: CompactString,
    #[serde(default)]
    pub version: CompactString,
    #[serde(default)]
    pub description: CompactString,
    #[serde(default)]
    pub injects_css: Option<bool>,
    #[serde(default = "default_format")]
    pub format: u32,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default, flatten)]
    pub extra: HashMap<String, toml::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Estructura que guarda las definiciones de un tema.
///
pub struct ThemeDef {
    #[serde(default)]
    pub colors: HashMap<String, String>,
    #[serde(default)]
    pub background: Background,
    #[serde(default)]
    pub text: HashMap<String, String>,
    #[serde(default)]
    pub fonts: Vec<FontFace>,
    #[serde(default)]
    pub icons: Icons,
    #[serde(default)]
    pub layout: HashMap<String, String>,
    #[serde(default)]
    pub borders: HashMap<String, String>,
    #[serde(default)]
    pub shadows: HashMap<String, String>,
    #[serde(default)]
    pub backgrounds: HashMap<String, String>,
    #[serde(default)]
    pub backdrop: HashMap<String, f64>,
    #[serde(default)]
    pub others: HashMap<String, String>,
    #[serde(default)]
    pub tokens: HashMap<String, String>,
    #[serde(default)]
    pub typography: Typography,
    #[serde(default)]
    pub spacing: HashMap<String, String>,
    #[serde(default)]
    pub motion: HashMap<String, String>,
    #[serde(default)]
    pub components: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub inject: Option<InjectConfig>,
    #[serde(default, flatten)]
    pub extra: HashMap<String, toml::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Background {
    #[serde(default)]
    pub reference_path: Option<String>,
    #[serde(default)]
    pub image_blur: Option<f64>,
    #[serde(default)]
    pub image_opacity: Option<f64>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub position: Option<String>,
    #[serde(default)]
    pub brightness: Option<f64>,
    #[serde(default, flatten)]
    pub extra: HashMap<String, toml::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Typography {
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub size_base: Option<String>,
    #[serde(default)]
    pub size_sm: Option<String>,
    #[serde(default)]
    pub size_lg: Option<String>,
    #[serde(default)]
    pub size_control: Option<String>,
    #[serde(default)]
    pub size_label: Option<String>,
    #[serde(default)]
    pub weight_normal: Option<String>,
    #[serde(default)]
    pub weight_medium: Option<String>,
    #[serde(default)]
    pub weight_bold: Option<String>,
    #[serde(default)]
    pub line_height: Option<String>,
    #[serde(default, flatten)]
    pub extra: HashMap<String, toml::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct InjectConfig {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default, flatten)]
    pub extra: HashMap<String, toml::Value>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Icons {
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default, flatten)]
    pub groups: HashMap<String, HashMap<String, String>>,
}

impl ZipImportable for ThemeMeta {
    const ZIP_TARGET_FILE: &'static str = "Meta.toml";

    fn parse_import(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Meta.toml inválido: {}", e))
    }

    fn import_name(&self) -> &str {
        self.name.as_str()
    }

    fn import_author(&self) -> &str {
        self.author.as_str()
    }

    fn import_version(&self) -> &str {
        self.version.as_str()
    }
}

impl Theme for V2Theme {
    fn get_name(&self) -> CompactString {
        self.meta.name.clone()
    }
    fn get_author(&self) -> CompactString {
        CompactString::new(self.meta.author.to_lowercase())
    }
    fn get_version(&self) -> CompactString {
        self.meta.version.clone()
    }
    fn to_theme_res(&self) -> super::ThemeResponse {
        let vars = flatten_variables(&self.theme);
        let icons = collect_icons(&self.theme.icons);

        super::ThemeResponse {
            name: self.get_name().to_string(),
            author: self.get_author().to_string(),
            version: self.get_version().to_string(),
            r#type: "user".to_string(),
            variables: vars,
            bg_image: self.theme.background.reference_path.clone(),
            bg_image_blur: self.theme.background.image_blur,
            bg_image_opacity: self.theme.background.image_opacity,
            fonts: self.theme.fonts.clone(),
            icons,
            inject_css: None,
            format: self.meta.format,
            api_version: self.meta.api_version.clone(),
            bg_image_size: self.theme.background.mode.clone(),
            bg_image_position: self.theme.background.position.clone(),
            bg_image_brightness: self.theme.background.brightness,
            inject_layer: None,
        }
    }

    fn load(theme_base: &Path, id: &str) -> Result<super::ThemeResponse, String> {
        let meta = read_meta(theme_base)?;
        let mut theme = read_definition(theme_base)?;

        match resolve_background(theme_base, &mut theme.background.reference_path)? {
            Some(BgValidationError::TooLarge) => {
                warn!("Theme '{}': background demasiado grande, ignorando", id)
            }
            Some(BgValidationError::InvalidImage) => {
                warn!("Theme '{}': bg_image no es una imagen válida", id)
            }
            Some(BgValidationError::Io) | None => {}
        }

        if let Some(preview) = theme.icons.preview.as_ref()
            && let Some(resolved) = resolve_theme_asset(theme_base, preview)?
        {
            theme.icons.preview = Some(resolved.to_string_lossy().into_owned());
        }
        if let Some(preview) = theme.icons.preview.as_ref()
            && !validate_theme_icon(preview)
        {
            warn!("Theme '{}': preview icon inválido, ignorando", id);
            theme.icons.preview = None;
        }

        for items in theme.icons.groups.values_mut() {
            for path in items.values_mut() {
                if let Some(resolved) = resolve_theme_asset(theme_base, path)? {
                    *path = resolved.to_string_lossy().into_owned();
                }
            }
        }
        theme.icons.groups.retain(|group, items| {
            items.retain(|name, path| {
                let valid = validate_theme_icon(path);
                if !valid {
                    warn!(
                        "Theme '{}': icon '{}:{}' inválido, ignorando",
                        id, group, name
                    );
                }
                valid
            });
            !items.is_empty()
        });
        resolve_fonts(theme_base, &mut theme.fonts)?;

        let (inject_css, inject_layer) = resolve_inject(theme_base, &meta, &theme, id)?;

        let mut response = Self { meta, theme }.to_theme_res();
        response.inject_css = inject_css;
        response.inject_layer = inject_layer;
        info!("Theme V2 convertido a intermediario correctamente");
        Ok(response)
    }

    fn list_entry(theme_base: &Path, id: &str) -> Option<ThemeEntry> {
        let content = std::fs::read_to_string(theme_base.join("Meta.toml")).ok()?;
        let meta: ThemeMeta = toml::from_str(&content).ok()?;
        let (preview, icon) = Self::list_assets(theme_base).unwrap_or((None, None));
        Some(ThemeEntry {
            id: id.into(),
            name: meta.name,
            author: meta.author.to_lowercase(),
            version: meta.version,
            r#type: "v2".into(),
            preview,
            icon,
        })
    }
}

impl V2Theme {
    fn list_assets(theme_base: &Path) -> Option<(Option<ThemePreview>, Option<String>)> {
        let content = std::fs::read_to_string(theme_base.join("Definition.toml")).ok()?;
        let definitions: ThemeDef = toml::from_str(&content).ok()?;
        let preview = Some(extract_preview(&flatten_variables(&definitions)));
        let icon = definitions.icons.preview.as_ref().and_then(|icon_path| {
            let abs = theme_base.join(icon_path);
            let path = abs.to_string_lossy().into_owned();
            validate_theme_icon(&path).then_some(path)
        });
        Some((preview, icon))
    }
}

pub(super) fn read_meta(theme_base: &Path) -> Result<ThemeMeta, String> {
    let path = theme_base.join("Meta.toml");
    let bytes = std::fs::read(&path).map_err(|e| {
        FsError::ReadFile {
            path: path.to_string_lossy().into_owned(),
            source: e,
        }
        .to_string()
    })?;
    toml::from_slice(&bytes).map_err(|e| CoreError::Serialize(e.to_string()).to_string())
}

pub(super) fn read_definition(theme_base: &Path) -> Result<ThemeDef, String> {
    let path = theme_base.join("Definition.toml");
    let bytes = std::fs::read(&path).map_err(|e| {
        FsError::ReadFile {
            path: path.to_string_lossy().into_owned(),
            source: e,
        }
        .to_string()
    })?;
    toml::from_slice(&bytes).map_err(|e| CoreError::Serialize(e.to_string()).to_string())
}

fn read_css(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| {
        FsError::ReadFile {
            path: path.to_string_lossy().into_owned(),
            source: e,
        }
        .to_string()
    })
}

fn resolve_inject(
    theme_base: &Path,
    meta: &ThemeMeta,
    theme: &ThemeDef,
    id: &str,
) -> Result<(Option<String>, Option<String>), String> {
    if let Some(config) = theme.inject.as_ref() {
        let enabled = config.enabled.or(meta.injects_css).unwrap_or(true);
        if !enabled {
            info!("Theme '{}': [inject] deshabilitado", id);
            return Ok((None, None));
        }

        let layer = match config.layer.as_deref() {
            Some("unlayered") => "unlayered".to_string(),
            Some("theme") | None => "theme".to_string(),
            Some(other) => {
                warn!(
                    "Theme '{}': [inject].layer '{}' inválido; se usará 'theme'",
                    id, other
                );
                "theme".to_string()
            }
        };

        let path = config.path.as_deref().unwrap_or("Inject.css");
        let Some(resolved) = resolve_theme_asset(theme_base, path)? else {
            return Ok((None, Some(layer)));
        };
        if !resolved.exists() {
            warn!(
                "Theme '{}': [inject].path '{}' no existe, ignorando",
                id, path
            );
            return Ok((None, Some(layer)));
        }

        let content = read_css(&resolved)?;
        info!("Theme '{}': [inject] leído, {} bytes", id, content.len());
        return Ok((Some(content), Some(layer)));
    }

    let path = theme_base.join("Inject.css");
    if !path.exists() {
        info!("Inject.css no encontrado en {:?}", path);
        return Ok((None, None));
    }

    warn!(
        "Theme '{}': Inject.css sin [inject]; cargando por compatibilidad. Añade [inject] para controlarlo.",
        id
    );
    let content = read_css(&path)?;
    info!("Inject.css leido, {} bytes", content.len());
    Ok((Some(content), None))
}

pub(super) fn is_valid_token_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("--") else {
        return false;
    };
    !rest.is_empty()
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[derive(Debug, Default)]
pub(super) struct FlattenOutput {
    pub variables: HashMap<String, String>,
    pub warnings: Vec<String>,
}

fn insert_var(out: &mut FlattenOutput, key: String, value: String) {
    if let Some(old) = out.variables.insert(key.clone(), value) {
        let warning = format!(
            "Key collision en theme: '{}' fue sobrescrito (anterior: '{}')",
            key, old
        );
        warn!("{warning}");
        out.warnings.push(warning);
    }
}

fn insert_optional(out: &mut FlattenOutput, key: &str, value: &Option<String>) {
    if let Some(value) = value {
        insert_var(out, key.to_string(), value.clone());
    }
}

fn insert_typography(out: &mut FlattenOutput, typography: &Typography) {
    insert_optional(out, "--font-family", &typography.family);
    insert_optional(out, "--font-size-base", &typography.size_base);
    insert_optional(out, "--font-size-sm", &typography.size_sm);
    insert_optional(out, "--font-size-lg", &typography.size_lg);
    insert_optional(out, "--font-size-control", &typography.size_control);
    insert_optional(out, "--font-size-label", &typography.size_label);
    insert_optional(out, "--font-weight-normal", &typography.weight_normal);
    insert_optional(out, "--font-weight-medium", &typography.weight_medium);
    insert_optional(out, "--font-weight-bold", &typography.weight_bold);
    insert_optional(out, "--line-height", &typography.line_height);
}

/// Toma la definición de íconos del theme y la aplana a un mapa de
/// clave única. `preview` se guarda bajo `"preview"` y los grupos se
/// serializan como `"<group>:<name>"`.
pub fn collect_icons(icons: &Icons) -> HashMap<String, String> {
    let mut collected = HashMap::new();

    if let Some(ref preview) = icons.preview {
        collected.insert("preview".to_string(), preview.clone());
    }

    for (group, items) in &icons.groups {
        for (name, path) in items {
            collected.insert(format!("{group}:{name}"), path.clone());
        }
    }

    collected
}

pub(super) fn flatten_with_diagnostics(theme: &ThemeDef) -> FlattenOutput {
    let mut out = FlattenOutput::default();

    for (k, v) in &theme.colors {
        insert_var(&mut out, format!("--{}", k), v.clone());
    }
    for (k, v) in &theme.text {
        insert_var(&mut out, format!("--text-{}", k), v.clone());
    }
    for (k, v) in &theme.borders {
        insert_var(&mut out, format!("--border-{}", k), v.clone());
    }
    for (k, v) in &theme.layout {
        insert_var(&mut out, format!("--{}", k), v.clone());
    }
    for (k, v) in &theme.shadows {
        insert_var(&mut out, format!("--{}", k), v.clone());
    }
    for (k, v) in &theme.backgrounds {
        insert_var(&mut out, format!("--bg-{}", k), v.clone());
    }
    for (k, v) in &theme.backdrop {
        insert_var(
            &mut out,
            format!("--backdrop-blur-{}", k),
            format!("{}px", v),
        );
    }
    for (k, v) in &theme.others {
        insert_var(&mut out, format!("--{}", k), v.clone());
    }

    insert_typography(&mut out, &theme.typography);

    for (k, v) in &theme.spacing {
        insert_var(&mut out, format!("--space-{}", k), v.clone());
    }
    for (k, v) in &theme.motion {
        insert_var(&mut out, format!("--transition-{}", k), v.clone());
    }
    for (component, entries) in &theme.components {
        for (k, v) in entries {
            insert_var(
                &mut out,
                format!("--component-{}-{}", component, k),
                v.clone(),
            );
        }
    }

    for (k, v) in &theme.tokens {
        if !is_valid_token_name(k) {
            let warning = format!(
                "Token '{}' inválido: debe empezar por '--' y usar solo letras, números, '_' o '-'",
                k
            );
            warn!("{warning}");
            out.warnings.push(warning);
            continue;
        }
        if let Some(old) = out.variables.insert(k.clone(), v.clone()) {
            let warning = format!(
                "El token '{}' colisiona con una variable generada; [tokens] gana (anterior: '{}')",
                k, old
            );
            warn!("{warning}");
            out.warnings.push(warning);
        }
    }

    out
}

pub fn flatten_variables(theme: &ThemeDef) -> HashMap<String, String> {
    flatten_with_diagnostics(theme).variables
}

#[cfg(test)]
#[path = "../../tests/commands/themes/v2.rs"]
mod tests;
