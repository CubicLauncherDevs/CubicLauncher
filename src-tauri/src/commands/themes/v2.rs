use super::FontFace;
use super::Theme;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub layout: HashMap<String, String>,
    #[serde(default)]
    pub borders: HashMap<String, String>,
    #[serde(default)]
    pub shadows: HashMap<String, String>,
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

impl Theme for V2Theme {
    fn get_name(&self) -> CompactString {
        self.meta.name.clone()
    }
    fn get_author(&self) -> CompactString {
        self.meta.author.clone()
    }
    fn get_version(&self) -> CompactString {
        self.meta.version.clone()
    }
    fn to_theme_res(&self) -> super::ThemeResponse {
        let vars = flatten_variables(&self.theme);
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
            inject_css: None, // Not Implemented
        }
    }
}

pub fn flatten_variables(theme: &ThemeDef) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    for (k, v) in &theme.colors {
        vars.insert(format!("--{}", k), v.clone());
    }
    for (k, v) in &theme.text {
        vars.insert(format!("--text-{}", k), v.clone());
    }
    for (k, v) in &theme.borders {
        vars.insert(format!("--border-{}", k), v.clone());
    }
    for (k, v) in &theme.layout {
        vars.insert(format!("--{}", k), v.clone());
    }
    for (k, v) in &theme.shadows {
        vars.insert(format!("--{}", k), v.clone());
    }
    for (k, v) in &theme.others {
        vars.insert(format!("--{}", k), v.clone());
    }

    if let Some(path) = &theme.background.reference_path {
        vars.insert("--bg-image-path".into(), path.clone());
    }
    if let Some(blur) = theme.background.image_blur {
        vars.insert("--bg-image-blur".into(), format!("{}px", blur));
    }
    if let Some(opacity) = theme.background.image_opacity {
        vars.insert("--bg-image-opacity".into(), opacity.to_string());
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_theme_meta(name: &str) -> ThemeMeta {
        ThemeMeta {
            name: CompactString::new(name),
            author: CompactString::new("TestAuthor"),
            version: CompactString::new("1.0"),
            description: CompactString::new("Test description"),
            injects_css: false,
        }
    }

    fn build_theme_def() -> ThemeDef {
        let mut colors = HashMap::new();
        colors.insert("accent".into(), "#ff0000".into());
        colors.insert("accent-rgb".into(), "255, 0, 0".into());

        let mut text = HashMap::new();
        text.insert("primary".into(), "#ffffff".into());

        let mut borders = HashMap::new();
        borders.insert("color".into(), "#333333".into());
        borders.insert("radius".into(), "8px".into());

        let mut shadows = HashMap::new();
        shadows.insert("shadow-sm".into(), "0 1px 3px rgba(0,0,0,0.5)".into());
        shadows.insert("glow-accent".into(), "0 0 12px rgba(255,0,0,0.3)".into());

        let mut others = HashMap::new();
        others.insert("icon-filter".into(), "invert(1)".into());

        ThemeDef {
            colors,
            background: Background {
                reference_path: Some("bg.webp".into()),
                image_blur: Some(10.0),
                image_opacity: Some(0.5),
            },
            text,
            fonts: vec![],
            layout: HashMap::new(),
            borders,
            shadows,
            others,
        }
    }

    #[test]
    fn parse_v2_theme_from_toml() {
        let toml_str = r##"
            [meta]
            name = "Test Theme"
            author = "Author"
            version = "2.0"
            description = "A test"
            injects_css = true

            [theme.background]
            reference_path = "image.png"
            image_blur = 5.0
            image_opacity = 0.8

            [theme.colors]
            accent = "#abc123"

            [theme.text]
            primary = "#000000"
        "##;

        let theme: V2Theme = toml::from_str(toml_str).expect("failed to parse TOML");
        assert_eq!(theme.meta.name.as_str(), "Test Theme");
        assert_eq!(theme.meta.author.as_str(), "Author");
        assert!(theme.meta.injects_css);
        assert_eq!(theme.theme.colors.get("accent").unwrap(), "#abc123");
        assert_eq!(theme.theme.text.get("primary").unwrap(), "#000000");
        assert_eq!(
            theme.theme.background.reference_path.as_deref(),
            Some("image.png")
        );
    }

    #[test]
    fn parse_minimal_theme() {
        let toml_str = r##"
            [meta]
            name = "Minimal"

            [theme.background]
        "##;

        let theme: V2Theme = toml::from_str(toml_str).expect("failed to parse minimal TOML");
        assert_eq!(theme.meta.name.as_str(), "Minimal");
        assert!(theme.meta.author.is_empty());
        assert!(theme.meta.version.is_empty());
        assert!(!theme.meta.injects_css);
        assert!(theme.theme.background.reference_path.is_none());
    }

    #[test]
    fn flatten_colors_prefix() {
        let mut theme = build_theme_def();
        theme.text.clear();
        theme.borders.clear();
        theme.shadows.clear();
        theme.others.clear();

        let vars = flatten_variables(&theme);
        assert_eq!(vars.get("--accent").unwrap(), "#ff0000");
        assert_eq!(vars.get("--accent-rgb").unwrap(), "255, 0, 0");
    }

    #[test]
    fn flatten_text_prefix() {
        let mut theme = build_theme_def();
        theme.colors.clear();
        theme.borders.clear();
        theme.shadows.clear();
        theme.others.clear();

        let vars = flatten_variables(&theme);
        assert_eq!(vars.get("--text-primary").unwrap(), "#ffffff");
    }

    #[test]
    fn flatten_borders_prefix() {
        let mut theme = build_theme_def();
        theme.colors.clear();
        theme.text.clear();
        theme.shadows.clear();
        theme.others.clear();

        let vars = flatten_variables(&theme);
        assert_eq!(vars.get("--border-color").unwrap(), "#333333");
        assert_eq!(vars.get("--border-radius").unwrap(), "8px");
    }

    #[test]
    fn flatten_shadows_no_prefix() {
        let mut theme = build_theme_def();
        theme.colors.clear();
        theme.text.clear();
        theme.borders.clear();
        theme.others.clear();

        let vars = flatten_variables(&theme);
        assert_eq!(
            vars.get("--shadow-sm").unwrap(),
            "0 1px 3px rgba(0,0,0,0.5)"
        );
        assert_eq!(
            vars.get("--glow-accent").unwrap(),
            "0 0 12px rgba(255,0,0,0.3)"
        );
    }

    #[test]
    fn flatten_others_no_prefix() {
        let mut theme = build_theme_def();
        theme.colors.clear();
        theme.text.clear();
        theme.borders.clear();
        theme.shadows.clear();

        let vars = flatten_variables(&theme);
        assert_eq!(vars.get("--icon-filter").unwrap(), "invert(1)");
    }

    #[test]
    fn flatten_background_fields() {
        let theme = build_theme_def();
        let vars = flatten_variables(&theme);
        assert_eq!(vars.get("--bg-image-path").unwrap(), "bg.webp");
        assert_eq!(vars.get("--bg-image-blur").unwrap(), "10px");
        assert_eq!(vars.get("--bg-image-opacity").unwrap(), "0.5");
    }

    #[test]
    fn flatten_background_none() {
        let mut theme = build_theme_def();
        theme.background = Background {
            reference_path: None,
            image_blur: None,
            image_opacity: None,
        };

        let vars = flatten_variables(&theme);
        assert!(!vars.contains_key("--bg-image-path"));
        assert!(!vars.contains_key("--bg-image-blur"));
        assert!(!vars.contains_key("--bg-image-opacity"));
    }

    #[test]
    fn flatten_empty_sections() {
        let theme = ThemeDef {
            colors: HashMap::new(),
            background: Background {
                reference_path: None,
                image_blur: None,
                image_opacity: None,
            },
            text: HashMap::new(),
            fonts: vec![],
            layout: HashMap::new(),
            borders: HashMap::new(),
            shadows: HashMap::new(),
            others: HashMap::new(),
        };

        let vars = flatten_variables(&theme);
        assert!(vars.is_empty());
    }

    #[test]
    fn trait_getters() {
        let v2 = V2Theme {
            meta: build_theme_meta("MyTheme"),
            theme: build_theme_def(),
        };

        assert_eq!(v2.get_name().as_str(), "MyTheme");
        assert_eq!(v2.get_author().as_str(), "TestAuthor");
        assert_eq!(v2.get_version().as_str(), "1.0");
    }

    #[test]
    fn to_theme_res_fields() {
        let v2 = V2Theme {
            meta: build_theme_meta("ResTheme"),
            theme: build_theme_def(),
        };

        let res = v2.to_theme_res();
        assert_eq!(res.name, "ResTheme");
        assert_eq!(res.author, "TestAuthor");
        assert_eq!(res.version, "1.0");
        assert_eq!(res.r#type, "user");
        assert!(res.variables.contains_key("--accent"));
        assert_eq!(res.bg_image.as_deref(), Some("bg.webp"));
        assert_eq!(res.bg_image_blur, Some(10.0));
        assert_eq!(res.bg_image_opacity, Some(0.5));
        assert!(res.inject_css.is_none());
    }
}
