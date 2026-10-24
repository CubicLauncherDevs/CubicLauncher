use super::FontFace;
use super::assets::{BgValidationError, extract_preview, resolve_background, resolve_fonts};
use super::{Theme, ZipImportable};
use crate::core::errors::{CoreError, FsError};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::warn;

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    Number(f64),
    String(String),
}

fn deserialize_optional_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<StringOrNumber>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(StringOrNumber::Number(value)) => Ok(Some(value)),
        Some(StringOrNumber::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<f64>()
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
    }
}

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
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub bg_image_blur: Option<f64>,
    #[serde(default)]
    pub bg_image_opacity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg_image_warning_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<FontFace>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ThemePreview {
    pub bg: String,
    pub accent: String,
    pub text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ThemeEntry {
    pub id: CompactString,
    pub name: CompactString,
    pub author: CompactString,
    pub version: CompactString,
    pub r#type: CompactString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<ThemePreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl Theme for ThemeFile {
    fn get_author(&self) -> CompactString {
        CompactString::new(self.author.to_lowercase())
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
            author: self.author.to_lowercase().to_string(),
            version: self.version.clone().into(),
            r#type: self.r#type.clone().into(),
            variables: self.variables.clone(),
            bg_image: self.bg_image.clone(),
            bg_image_blur: self.bg_image_blur,
            bg_image_opacity: self.bg_image_opacity,
            fonts: self.fonts.clone(),
            icons: HashMap::new(),
            inject_css: None,
            format: 1,
            api_version: None,
            bg_image_size: None,
            bg_image_position: None,
            bg_image_brightness: None,
            inject_layer: None,
        }
    }

    fn load(theme_base: &Path, id: &str) -> Result<super::ThemeResponse, String> {
        let path = theme_base.join("theme.json");
        let content = std::fs::read_to_string(&path).map_err(|e| {
            FsError::ReadFile {
                path: path.to_string_lossy().into_owned(),
                source: e,
            }
            .to_string()
        })?;
        let mut theme: Self = serde_json::from_str(&content)
            .map_err(|e| CoreError::Other(format!("Theme '{}' inválido: {}", id, e)).to_string())?;

        match resolve_background(theme_base, &mut theme.bg_image)? {
            Some(BgValidationError::TooLarge) => {
                warn!("Theme '{}': bg_image demasiado grande, ignorando", id);
                theme.bg_image_warning_key = Some("themes.warning.largeFile".into());
            }
            Some(BgValidationError::InvalidImage) => {
                warn!("Theme '{}': bg_image no es una imagen válida", id);
                theme.bg_image_warning_key = Some("themes.warning.notAnImage".into());
            }
            Some(BgValidationError::Io) | None => {}
        }
        resolve_fonts(theme_base, &mut theme.fonts)?;
        Ok(theme.to_theme_res())
    }

    fn list_entry(theme_base: &Path, id: &str) -> Option<ThemeEntry> {
        let content = std::fs::read_to_string(theme_base.join("theme.json")).ok()?;
        let theme: Self = serde_json::from_str(&content).ok()?;
        Some(ThemeEntry {
            id: id.into(),
            name: theme.name.to_lowercase(),
            author: theme.author.to_lowercase(),
            version: theme.version,
            r#type: theme.r#type,
            preview: Some(extract_preview(&theme.variables)),
            icon: None,
        })
    }
}

impl ZipImportable for ThemeFile {
    const ZIP_TARGET_FILE: &'static str = "theme.json";

    fn parse_import(content: &str) -> Result<Self, String> {
        serde_json::from_str(content).map_err(|e| format!("theme.json inválido: {}", e))
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
