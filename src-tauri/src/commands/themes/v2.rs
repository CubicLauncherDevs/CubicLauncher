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
    pub injects_css: bool,
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
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Background {
    #[serde(default)]
    pub reference_path: Option<String>,
    #[serde(default)]
    pub image_blur: Option<f64>,
    #[serde(default)]
    pub image_opacity: Option<f64>,
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
            inject_css: None, // Not Implemented
        }
    }

    fn load(theme_base: &Path, id: &str) -> Result<super::ThemeResponse, String> {
        let read_toml = |name: &str| -> Result<Vec<u8>, String> {
            let path = theme_base.join(name);
            std::fs::read(&path).map_err(|e| {
                FsError::ReadFile {
                    path: path.to_string_lossy().into_owned(),
                    source: e,
                }
                .to_string()
            })
        };
        let meta: ThemeMeta = toml::from_slice(&read_toml("Meta.toml")?)
            .map_err(|e| CoreError::Serialize(e.to_string()).to_string())?;
        let mut theme: ThemeDef = toml::from_slice(&read_toml("Definition.toml")?)
            .map_err(|e| CoreError::Serialize(e.to_string()).to_string())?;

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

        let inject_css_path = theme_base.join("Inject.css");
        let inject_css = if inject_css_path.exists() {
            let content = std::fs::read_to_string(&inject_css_path).map_err(|e| {
                FsError::ReadFile {
                    path: inject_css_path.to_string_lossy().into_owned(),
                    source: e,
                }
                .to_string()
            })?;
            info!("Inject.css leido, {} bytes", content.len());
            Some(content)
        } else {
            info!("Inject.css no encontrado en {:?}", inject_css_path);
            None
        };

        let mut response = Self { meta, theme }.to_theme_res();
        response.inject_css = inject_css;
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

fn insert_var(vars: &mut HashMap<String, String>, key: String, value: String) {
    if let Some(old) = vars.insert(key.clone(), value) {
        warn!(
            "Key collision en theme: '{}' fue sobrescrito (anterior: '{}')",
            key, old
        );
    }
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

pub fn flatten_variables(theme: &ThemeDef) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    for (k, v) in &theme.colors {
        insert_var(&mut vars, format!("--{}", k), v.clone());
    }
    for (k, v) in &theme.text {
        insert_var(&mut vars, format!("--text-{}", k), v.clone());
    }
    for (k, v) in &theme.borders {
        insert_var(&mut vars, format!("--border-{}", k), v.clone());
    }
    for (k, v) in &theme.layout {
        insert_var(&mut vars, format!("--{}", k), v.clone());
    }
    for (k, v) in &theme.shadows {
        insert_var(&mut vars, format!("--{}", k), v.clone());
    }
    for (k, v) in &theme.backgrounds {
        insert_var(&mut vars, format!("--bg-{}", k), v.clone());
    }
    for (k, v) in &theme.backdrop {
        insert_var(
            &mut vars,
            format!("--backdrop-blur-{}", k),
            format!("{}px", v),
        );
    }
    for (k, v) in &theme.others {
        insert_var(&mut vars, format!("--{}", k), v.clone());
    }

    vars
}

#[cfg(test)]
#[path = "../../tests/commands/themes/v2.rs"]
mod tests;
