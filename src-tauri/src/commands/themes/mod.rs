pub(crate) mod archive;
mod assets;
pub(crate) mod commands;
pub(crate) mod install;
mod v1;
mod v2;

use self::v1::{ThemeEntry, ThemeFile};
use self::v2::V2Theme;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

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

#[derive(Debug, Serialize, Clone)]
pub struct ThemeResponse {
    pub name: String,
    pub author: String,
    pub version: String,
    pub r#type: String,
    pub variables: HashMap<String, String>,
    pub bg_image: Option<String>,
    pub bg_image_blur: Option<f64>,
    pub bg_image_opacity: Option<f64>,
    pub fonts: Vec<FontFace>,
    pub icons: HashMap<String, String>,
    pub inject_css: Option<String>,
}

trait Theme {
    fn get_name(&self) -> CompactString;
    fn get_author(&self) -> CompactString;
    fn get_version(&self) -> CompactString; // semver
    fn to_theme_res(&self) -> ThemeResponse;
    fn load(theme_base: &Path, id: &str) -> Result<ThemeResponse, String>;
    fn list_entry(theme_base: &Path, id: &str) -> Option<ThemeEntry>;
}

pub(crate) trait ZipImportable: Sized {
    const ZIP_TARGET_FILE: &'static str;
    fn parse_import(content: &str) -> Result<Self, String>;
    fn import_name(&self) -> &str;
    fn import_author(&self) -> &str;
    fn import_version(&self) -> &str;
}

fn load_user_theme(theme_base: &Path, id: &str) -> Result<ThemeResponse, String> {
    let meta_path = theme_base.join("Meta.toml");
    if std::fs::exists(&meta_path).map_err(|e| e.to_string())? {
        info!("El theme {id} tiene Meta.toml; se cargará como V2");
        V2Theme::load(theme_base, id)
    } else {
        ThemeFile::load(theme_base, id)
    }
}
