use compact_str::CompactString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontFace {
    pub family: CompactString,
    pub src: CompactString,
    #[serde(default)]
    pub format: Option<CompactString>,
    #[serde(default)]
    pub weight: Option<CompactString>,
    #[serde(default)]
    pub style: Option<CompactString>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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