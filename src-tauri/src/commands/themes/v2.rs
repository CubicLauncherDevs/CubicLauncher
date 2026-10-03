use super::FontFace;
use super::{Theme, ZipImportable};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::warn;

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
            bg_music: None,
            bg_image_blur: self.theme.background.image_blur,
            bg_image_opacity: self.theme.background.image_opacity,
            fonts: self.theme.fonts.clone(),
            icons,
            inject_css: None, // Not Implemented
        }
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
