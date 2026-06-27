use super::FontFace;
use super::Theme;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Estructura que representa la primera version del sistema de Themes
/// Implementa el Trait `Theme` el cual es el que lo permite ser parseado
/// como tal. Estos se traducen a un struct intermedio el cual el
/// frontend carga.
///
pub struct ThemeFile {
    pub name: CompactString,
    #[serde(default)]
    pub author: CompactString,
    #[serde(default)]
    pub version: CompactString,
    #[serde(default)]
    pub r#type: CompactString,
    pub variables: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub bg_image: Option<String>,
    #[serde(default)]
    pub bg_image_blur: Option<String>,
    #[serde(default)]
    pub bg_image_opacity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg_image_warning_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<FontFace>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ThemeEntry {
    pub id: CompactString,
    pub name: CompactString,
    pub author: CompactString,
    pub version: CompactString,
    pub r#type: CompactString,
}

impl Theme for ThemeFile {
    fn get_author(&self) -> CompactString {
        self.author.clone()
    }
    fn get_name(&self) -> CompactString {
        self.name.clone()
    }
    fn get_version(&self) -> CompactString {
        self.version.clone()
    }

    fn to_theme_res(&self) -> super::ThemeResponse {
        super::ThemeResponse {
            name: self.name.clone().into(),
            author: self.author.clone().into(),
            version: self.version.clone().into(),
            r#type: self.r#type.clone().into(),
            variables: self.variables.clone(),
            bg_image: self.bg_image.clone(),
            bg_image_blur: Some(parse_f64_with_default(self.bg_image_blur.as_deref(), 0.0)),
            bg_image_opacity: self.bg_image_opacity.clone(),
            fonts: self.fonts.clone(),
            inject_css: None, // Not implemented
        }
    }
}

fn parse_f64_with_default(value: Option<&str>, default: f64) -> f64 {
    match value {
        Some(s) if !s.is_empty() => s.parse::<f64>().unwrap_or(default),
        _ => default,
    }
}
