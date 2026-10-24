use super::FontFace;
use super::assets::{
    BgValidationError, extract_preview, resolve_background, resolve_fonts, resolve_theme_asset,
    validate_theme_icon,
};
use super::v1::{ThemeEntry, ThemePreview};
use super::{Theme, ZipImportable};
use crate::core::errors::{CoreError, FsError};
use crate::core::validate_identifier;
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
    /// Id de otro tema V2 instalado del que se heredan los tokens.
    #[serde(default)]
    pub extends: Option<String>,
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
    /// CSS opcional que reestructura el shell usando atributos `data-part`.
    #[serde(default)]
    pub layout_pack: Option<LayoutPackConfig>,
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
pub struct LayoutPackConfig {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
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
            layout_css: None,
        }
    }

    fn load_at(
        theme_base: &Path,
        id: &str,
        themes_root: &Path,
    ) -> Result<super::ThemeResponse, String> {
        let meta = read_meta(theme_base)?;
        let mut theme = load_definition_with_inheritance(theme_base, id, themes_root)?;

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
        let layout_css = resolve_layout_pack(theme_base, &theme, id)?;

        let mut response = Self { meta, theme }.to_theme_res();
        response.inject_css = inject_css;
        response.inject_layer = inject_layer;
        response.layout_css = layout_css;
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

pub(super) const MAX_EXTENDS_DEPTH: usize = 8;

/// Carga `Definition.toml` resolviendo `extends` recursivamente. La cadena se
/// busca siempre dentro de `themes_root`, nunca con rutas del manifest.
fn load_definition_with_inheritance(
    theme_base: &Path,
    id: &str,
    themes_root: &Path,
) -> Result<ThemeDef, String> {
    let mut chain: Vec<String> = Vec::new();
    resolve_definition(theme_base, id, themes_root, &mut chain, 0)
}

fn resolve_definition(
    theme_base: &Path,
    id: &str,
    themes_root: &Path,
    chain: &mut Vec<String>,
    depth: usize,
) -> Result<ThemeDef, String> {
    if depth > MAX_EXTENDS_DEPTH {
        return Err(format!(
            "Herencia demasiado profunda: '{}' supera el máximo de {MAX_EXTENDS_DEPTH} niveles",
            chain.join(" -> ")
        ));
    }
    if chain.iter().any(|seen| seen == id) {
        let mut cycle = chain.clone();
        cycle.push(id.to_string());
        return Err(format!("Ciclo de herencia: {}", cycle.join(" -> ")));
    }
    chain.push(id.to_string());

    let meta = read_meta(theme_base)?;
    let theme = read_definition(theme_base)?;

    let Some(base_id) = meta
        .extends
        .as_deref()
        .map(str::trim)
        .filter(|base| !base.is_empty())
    else {
        return Ok(theme);
    };

    let base_id = base_id.strip_prefix("user:").unwrap_or(base_id);
    validate_identifier(base_id).map_err(|e| format!("Meta.toml: 'extends' inválido: {e}"))?;

    let base_dir = themes_root.join(base_id);
    if !base_dir.join("Meta.toml").is_file() {
        return Err(format!(
            "Meta.toml: el tema base '{base_id}' no está instalado o no es V2"
        ));
    }

    let base = resolve_definition(&base_dir, base_id, themes_root, chain, depth + 1)?;
    info!("Theme '{}': hereda tokens de '{}'", id, base_id);
    Ok(merge_definitions(base, theme))
}

/// El hijo gana clave a clave. Assets, fuentes, iconos e `[inject]` no se
/// heredan porque sus rutas son relativas al tema que los declara.
fn merge_definitions(mut base: ThemeDef, child: ThemeDef) -> ThemeDef {
    base.colors.extend(child.colors);
    base.text.extend(child.text);
    base.borders.extend(child.borders);
    base.layout.extend(child.layout);
    base.shadows.extend(child.shadows);
    base.backgrounds.extend(child.backgrounds);
    base.backdrop.extend(child.backdrop);
    base.others.extend(child.others);
    base.tokens.extend(child.tokens);
    base.spacing.extend(child.spacing);
    base.motion.extend(child.motion);
    for (component, entries) in child.components {
        base.components
            .entry(component)
            .or_default()
            .extend(entries);
    }
    base.typography = merge_typography(base.typography, child.typography);
    base.background = merge_background(base.background, child.background);

    base.fonts = child.fonts;
    base.icons = child.icons;
    base.inject = child.inject;
    base.layout_pack = child.layout_pack;
    base.extra = child.extra;
    base
}

fn merge_typography(base: Typography, child: Typography) -> Typography {
    Typography {
        family: child.family.or(base.family),
        size_base: child.size_base.or(base.size_base),
        size_sm: child.size_sm.or(base.size_sm),
        size_lg: child.size_lg.or(base.size_lg),
        size_control: child.size_control.or(base.size_control),
        size_label: child.size_label.or(base.size_label),
        weight_normal: child.weight_normal.or(base.weight_normal),
        weight_medium: child.weight_medium.or(base.weight_medium),
        weight_bold: child.weight_bold.or(base.weight_bold),
        line_height: child.line_height.or(base.line_height),
        extra: child.extra,
    }
}

fn merge_background(base: Background, child: Background) -> Background {
    Background {
        // La imagen es un asset del tema base y no se hereda.
        reference_path: child.reference_path,
        image_blur: child.image_blur.or(base.image_blur),
        image_opacity: child.image_opacity.or(base.image_opacity),
        mode: child.mode.or(base.mode),
        position: child.position.or(base.position),
        brightness: child.brightness.or(base.brightness),
        extra: child.extra,
    }
}

fn resolve_layout_pack(
    theme_base: &Path,
    theme: &ThemeDef,
    id: &str,
) -> Result<Option<String>, String> {
    let Some(config) = theme.layout_pack.as_ref() else {
        return Ok(None);
    };
    if config.enabled == Some(false) {
        info!("Theme '{}': [layout_pack] deshabilitado", id);
        return Ok(None);
    }

    let path = config.path.as_deref().unwrap_or("Layout.css");
    let Some(resolved) = resolve_theme_asset(theme_base, path)? else {
        return Ok(None);
    };
    if !resolved.is_file() {
        warn!(
            "Theme '{}': [layout_pack].path '{}' no existe, ignorando",
            id, path
        );
        return Ok(None);
    }

    let content = read_css(&resolved)?;
    info!(
        "Theme '{}': layout pack leído ({} bytes)",
        id,
        content.len()
    );
    Ok(Some(content))
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
