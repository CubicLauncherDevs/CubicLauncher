use super::*;

fn build_theme_meta(name: &str) -> ThemeMeta {
    ThemeMeta {
        name: CompactString::new(name),
        author: CompactString::new("TestAuthor"),
        version: CompactString::new("1.0"),
        description: CompactString::new("Test description"),
        injects_css: None,
        format: 2,
        api_version: None,
        extends: None,
        extra: HashMap::new(),
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

    let mut ui = HashMap::new();
    ui.insert("play".into(), "ui/play.svg".into());
    let icons = Icons {
        preview: Some("preview.png".into()),
        groups: {
            let mut groups = HashMap::new();
            groups.insert("ui".into(), ui);
            groups
        },
    };

    ThemeDef {
        colors,
        background: Background {
            reference_path: Some("bg.webp".into()),
            image_blur: Some(10.0),
            image_opacity: Some(0.5),
            mode: None,
            position: None,
            brightness: None,
            extra: HashMap::new(),
        },
        text,
        fonts: vec![],
        icons,
        layout: HashMap::new(),
        borders,
        shadows,
        backgrounds: HashMap::new(),
        backdrop: HashMap::new(),
        others,
        tokens: HashMap::new(),
        typography: Typography::default(),
        spacing: HashMap::new(),
        motion: HashMap::new(),
        components: HashMap::new(),
        inject: None,
        layout_pack: None,
        extra: HashMap::new(),
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

            [theme.icons]
            preview = "preview.png"

            [theme.icons.ui]
            play = "ui/play.svg"
        "##;

    let theme: V2Theme = toml::from_str(toml_str).expect("failed to parse TOML");
    assert_eq!(theme.meta.name.as_str(), "Test Theme");
    assert_eq!(theme.meta.author.as_str(), "Author");
    assert_eq!(theme.meta.injects_css, Some(true));
    assert_eq!(theme.theme.colors.get("accent").unwrap(), "#abc123");
    assert_eq!(theme.theme.text.get("primary").unwrap(), "#000000");
    assert_eq!(
        theme.theme.background.reference_path.as_deref(),
        Some("image.png")
    );
    assert_eq!(theme.theme.icons.preview.as_deref(), Some("preview.png"));
    assert_eq!(
        theme
            .theme
            .icons
            .groups
            .get("ui")
            .and_then(|g| g.get("play"))
            .map(String::as_str),
        Some("ui/play.svg")
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
    assert_eq!(theme.meta.injects_css, None);
    assert!(theme.theme.background.reference_path.is_none());
}

#[test]
fn parse_additive_fields() {
    let toml_str = r##"
            [meta]
            name = "Additive"
            format = 2
            api_version = "2.1"
            injects_css = true

            [theme.tokens]
            "--sidebar-width" = "300px"

            [theme.typography]
            family = "Inter, sans-serif"
            size_base = "14px"
            weight_bold = "700"
            line_height = "1.6"

            [theme.spacing]
            sm = "8px"

            [theme.motion]
            fast = "0.15s"

            [theme.components.sidebar]
            width = "260px"

            [theme.background]
            mode = "contain"
            position = "top"
            brightness = 0.6

            [theme.inject]
            path = "custom.css"
            enabled = true
            layer = "theme"
        "##;

    let theme: V2Theme = toml::from_str(toml_str).expect("failed to parse additive fields");
    assert_eq!(theme.meta.format, 2);
    assert_eq!(theme.meta.api_version.as_deref(), Some("2.1"));
    assert_eq!(
        theme
            .theme
            .tokens
            .get("--sidebar-width")
            .map(String::as_str),
        Some("300px")
    );
    assert_eq!(
        theme.theme.typography.family.as_deref(),
        Some("Inter, sans-serif")
    );
    assert_eq!(theme.theme.typography.size_base.as_deref(), Some("14px"));
    assert_eq!(theme.theme.typography.weight_bold.as_deref(), Some("700"));
    assert_eq!(theme.theme.typography.line_height.as_deref(), Some("1.6"));
    assert_eq!(
        theme.theme.spacing.get("sm").map(String::as_str),
        Some("8px")
    );
    assert_eq!(
        theme.theme.motion.get("fast").map(String::as_str),
        Some("0.15s")
    );
    assert_eq!(
        theme
            .theme
            .components
            .get("sidebar")
            .and_then(|values| values.get("width"))
            .map(String::as_str),
        Some("260px")
    );
    assert_eq!(theme.theme.background.mode.as_deref(), Some("contain"));
    assert_eq!(theme.theme.background.position.as_deref(), Some("top"));
    assert_eq!(theme.theme.background.brightness, Some(0.6));
    let inject = theme.theme.inject.as_ref().expect("inject config");
    assert_eq!(inject.path.as_deref(), Some("custom.css"));
    assert_eq!(inject.enabled, Some(true));
    assert_eq!(inject.layer.as_deref(), Some("theme"));
}

#[test]
fn unknown_fields_are_collected_for_validation() {
    let toml_str = r##"
            [meta]
            name = "Unknown"
            unexpected = "value"

            [theme.background]
            reference_path = "bg.webp"
            mystery = 1

            [theme.typography]
            family = "Inter"
            size_xl = "20px"
        "##;

    let theme: V2Theme = toml::from_str(toml_str).expect("unknown fields should not fail");
    assert!(theme.meta.extra.contains_key("unexpected"));
    assert!(theme.theme.background.extra.contains_key("mystery"));
    assert!(theme.theme.typography.extra.contains_key("size_xl"));
}

#[test]
fn flatten_additive_sections() {
    let mut theme = build_theme_def();
    theme.colors.clear();
    theme.text.clear();
    theme.borders.clear();
    theme.shadows.clear();
    theme.others.clear();
    theme.typography = Typography {
        family: Some("Inter".into()),
        size_base: Some("14px".into()),
        size_sm: Some("12px".into()),
        size_lg: Some("16px".into()),
        size_control: Some("13px".into()),
        size_label: Some("11px".into()),
        weight_normal: Some("400".into()),
        weight_medium: Some("600".into()),
        weight_bold: Some("700".into()),
        line_height: Some("1.5".into()),
        extra: HashMap::new(),
    };
    theme.spacing.insert("md".into(), "12px".into());
    theme.motion.insert("normal".into(), "0.2s".into());
    theme.components.insert(
        "market".into(),
        HashMap::from([("card_min_width".into(), "280px".into())]),
    );

    let vars = flatten_variables(&theme);
    assert_eq!(vars.get("--font-family").unwrap(), "Inter");
    assert_eq!(vars.get("--font-size-base").unwrap(), "14px");
    assert_eq!(vars.get("--font-size-sm").unwrap(), "12px");
    assert_eq!(vars.get("--font-size-lg").unwrap(), "16px");
    assert_eq!(vars.get("--font-size-control").unwrap(), "13px");
    assert_eq!(vars.get("--font-size-label").unwrap(), "11px");
    assert_eq!(vars.get("--font-weight-normal").unwrap(), "400");
    assert_eq!(vars.get("--font-weight-medium").unwrap(), "600");
    assert_eq!(vars.get("--font-weight-bold").unwrap(), "700");
    assert_eq!(vars.get("--line-height").unwrap(), "1.5");
    assert_eq!(vars.get("--space-md").unwrap(), "12px");
    assert_eq!(vars.get("--transition-normal").unwrap(), "0.2s");
    assert_eq!(
        vars.get("--component-market-card_min_width").unwrap(),
        "280px"
    );
}

#[test]
fn tokens_win_and_report_collisions() {
    let mut theme = build_theme_def();
    theme.tokens.insert("--accent".into(), "#00ff00".into());

    let output = flatten_with_diagnostics(&theme);
    assert_eq!(output.variables.get("--accent").unwrap(), "#00ff00");
    assert!(
        output
            .warnings
            .iter()
            .any(|warning| warning.contains("colisiona") && warning.contains("--accent"))
    );
}

#[test]
fn invalid_token_names_are_skipped() {
    let mut theme = build_theme_def();
    theme.tokens.insert("accent".into(), "#00ff00".into());
    theme.tokens.insert("--valid-token".into(), "1".into());

    let output = flatten_with_diagnostics(&theme);
    assert!(!output.variables.contains_key("accent"));
    assert_eq!(output.variables.get("--valid-token").unwrap(), "1");
    assert!(
        output
            .warnings
            .iter()
            .any(|warning| warning.contains("inválido"))
    );
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
    assert!(
        !vars.contains_key("--bg-image-path"),
        "background fields should NOT be in variables (exposed via ThemeResponse fields)"
    );
    assert!(!vars.contains_key("--bg-image-blur"));
    assert!(!vars.contains_key("--bg-image-opacity"));
}

#[test]
fn flatten_background_none() {
    let mut theme = build_theme_def();
    theme.background = Background {
        reference_path: None,
        image_blur: None,
        image_opacity: None,
        mode: None,
        position: None,
        brightness: None,
        extra: HashMap::new(),
    };

    let vars = flatten_variables(&theme);
    assert!(!vars.contains_key("--bg-image-path"));
    assert!(!vars.contains_key("--bg-image-blur"));
    assert!(!vars.contains_key("--bg-image-opacity"));
}

#[test]
fn flatten_backgrounds_prefix() {
    let mut theme = build_theme_def();
    theme.colors.clear();
    theme.text.clear();
    theme.borders.clear();
    theme.shadows.clear();
    theme.others.clear();
    theme.backgrounds.insert("main".into(), "#000000".into());
    theme.backgrounds.insert("card".into(), "#111111".into());

    let vars = flatten_variables(&theme);
    assert_eq!(vars.get("--bg-main").unwrap(), "#000000");
    assert_eq!(vars.get("--bg-card").unwrap(), "#111111");
}

#[test]
fn flatten_backdrop_prefix_and_px() {
    let mut theme = build_theme_def();
    theme.colors.clear();
    theme.text.clear();
    theme.borders.clear();
    theme.shadows.clear();
    theme.others.clear();
    theme.backdrop.insert("dropdown".into(), 10.0);
    theme.backdrop.insert("modal".into(), 4.0);

    let vars = flatten_variables(&theme);
    assert_eq!(vars.get("--backdrop-blur-dropdown").unwrap(), "10px");
    assert_eq!(vars.get("--backdrop-blur-modal").unwrap(), "4px");
}

#[test]
fn flatten_empty_sections() {
    let theme = ThemeDef {
        colors: HashMap::new(),
        background: Background {
            reference_path: None,
            image_blur: None,
            image_opacity: None,
            mode: None,
            position: None,
            brightness: None,
            extra: HashMap::new(),
        },
        text: HashMap::new(),
        fonts: vec![],
        icons: Icons::default(),
        layout: HashMap::new(),
        borders: HashMap::new(),
        shadows: HashMap::new(),
        backgrounds: HashMap::new(),
        backdrop: HashMap::new(),
        others: HashMap::new(),
        tokens: HashMap::new(),
        typography: Typography::default(),
        spacing: HashMap::new(),
        motion: HashMap::new(),
        components: HashMap::new(),
        inject: None,
        layout_pack: None,
        extra: HashMap::new(),
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
    assert_eq!(v2.get_author().as_str(), "testauthor");
    assert_eq!(v2.get_version().as_str(), "1.0");
}

#[test]
fn collect_icons_preview_and_groups() {
    let mut ui = HashMap::new();
    ui.insert("play".into(), "ui/play.svg".into());
    ui.insert("trash".into(), "ui/trash.svg".into());

    let mut nav = HashMap::new();
    nav.insert("settings".into(), "nav/settings.svg".into());

    let icons = Icons {
        preview: Some("preview.png".into()),
        groups: {
            let mut groups = HashMap::new();
            groups.insert("ui".into(), ui);
            groups.insert("nav".into(), nav);
            groups
        },
    };

    let collected = collect_icons(&icons);
    assert_eq!(collected.len(), 4);
    assert_eq!(
        collected.get("preview").map(String::as_str),
        Some("preview.png")
    );
    assert_eq!(
        collected.get("ui:play").map(String::as_str),
        Some("ui/play.svg")
    );
    assert_eq!(
        collected.get("ui:trash").map(String::as_str),
        Some("ui/trash.svg")
    );
    assert_eq!(
        collected.get("nav:settings").map(String::as_str),
        Some("nav/settings.svg")
    );
}

#[test]
fn collect_icons_empty() {
    let icons = Icons::default();
    let collected = collect_icons(&icons);
    assert!(collected.is_empty());
}

#[test]
fn collect_icons_only_preview() {
    let icons = Icons {
        preview: Some("icon.png".into()),
        groups: HashMap::new(),
    };
    let collected = collect_icons(&icons);
    assert_eq!(collected.len(), 1);
    assert_eq!(
        collected.get("preview").map(String::as_str),
        Some("icon.png")
    );
}

#[test]
fn collect_icons_only_groups() {
    let mut group = HashMap::new();
    group.insert("puzzle".into(), "instance/puzzle.svg".into());

    let icons = Icons {
        preview: None,
        groups: {
            let mut groups = HashMap::new();
            groups.insert("instance".into(), group);
            groups
        },
    };

    let collected = collect_icons(&icons);
    assert_eq!(collected.len(), 1);
    assert_eq!(
        collected.get("instance:puzzle").map(String::as_str),
        Some("instance/puzzle.svg")
    );
}

#[test]
fn to_theme_res_fields() {
    let v2 = V2Theme {
        meta: build_theme_meta("ResTheme"),
        theme: build_theme_def(),
    };

    let res = v2.to_theme_res();
    assert_eq!(res.name, "ResTheme");
    assert_eq!(res.author, "testauthor");
    assert_eq!(res.version, "1.0");
    assert_eq!(res.r#type, "user");
    assert!(res.variables.contains_key("--accent"));
    assert_eq!(res.bg_image.as_deref(), Some("bg.webp"));
    assert_eq!(res.bg_image_blur, Some(10.0));
    assert_eq!(res.bg_image_opacity, Some(0.5));
    assert_eq!(
        res.icons.get("preview").map(String::as_str),
        Some("preview.png")
    );
    assert_eq!(
        res.icons.get("ui:play").map(String::as_str),
        Some("ui/play.svg")
    );
    assert!(res.inject_css.is_none());
    assert_eq!(res.format, 2);
    assert!(res.api_version.is_none());
    assert!(res.bg_image_size.is_none());
    assert!(res.bg_image_position.is_none());
    assert!(res.bg_image_brightness.is_none());
    assert!(res.inject_layer.is_none());
}


// ── Herencia (extends) y layout packs ───────────────────────────────────────

struct ThemeTree(std::path::PathBuf);

impl ThemeTree {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("cubic-theme-tree-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }

    fn theme(&self, id: &str) -> std::path::PathBuf {
        self.0.join(id)
    }

    fn write(&self, theme: &str, name: &str, content: &str) {
        let path = self.theme(theme).join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}

impl Drop for ThemeTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_meta(tree: &ThemeTree, id: &str, name: &str, extra: &str) {
    tree.write(
        id,
        "Meta.toml",
        &format!("name = '{name}'\nauthor = 'Author'\nversion = '1.0'\n{extra}"),
    );
}

#[test]
fn extends_merges_tokens_and_child_wins() {
    let tree = ThemeTree::new();
    write_meta(&tree, "base", "Base", "");
    tree.write(
        "base",
        "Definition.toml",
        "[colors]\naccent = '#ff0000'\nbg = '#101010'\n\n[text]\nprimary = '#ffffff'\n\n[typography]\nfamily = 'Inter, sans-serif'\n\n[tokens]\n'--base-token' = '1'\n",
    );

    write_meta(&tree, "child", "Child", "extends = 'base'\n");
    tree.write(
        "child",
        "Definition.toml",
        "[colors]\naccent = '#0000ff'\n",
    );

    let res = V2Theme::load(&tree.theme("child"), "child").unwrap();

    assert_eq!(res.variables.get("--accent").map(String::as_str), Some("#0000ff"));
    assert_eq!(res.variables.get("--bg").map(String::as_str), Some("#101010"));
    assert_eq!(
        res.variables.get("--text-primary").map(String::as_str),
        Some("#ffffff")
    );
    assert_eq!(
        res.variables.get("--font-family").map(String::as_str),
        Some("Inter, sans-serif")
    );
    assert_eq!(
        res.variables.get("--base-token").map(String::as_str),
        Some("1")
    );
}

#[test]
fn extends_does_not_inherit_assets() {
    let tree = ThemeTree::new();
    write_meta(&tree, "base", "Base", "");
    tree.write(
        "base",
        "Definition.toml",
        "[[fonts]]\nfamily = 'Base'\nsrc = 'base.woff2'\n\n[icons]\npreview = 'preview.png'\n\n[background]\nreference_path = 'bg.webp'\nimage_blur = 12\n",
    );

    write_meta(&tree, "child", "Child", "extends = 'base'\n");
    tree.write("child", "Definition.toml", "[colors]\naccent = '#0000ff'\n");

    let res = V2Theme::load(&tree.theme("child"), "child").unwrap();

    assert!(res.fonts.is_empty());
    assert!(res.icons.is_empty());
    assert!(res.bg_image.is_none());
    // Los escalares del fondo sí se heredan.
    assert_eq!(res.bg_image_blur, Some(12.0));
}

#[test]
fn extends_cycle_is_an_error() {
    let tree = ThemeTree::new();
    write_meta(&tree, "a", "A", "extends = 'b'\n");
    tree.write("a", "Definition.toml", "[colors]\naccent = '#ff0000'\n");
    write_meta(&tree, "b", "B", "extends = 'a'\n");
    tree.write("b", "Definition.toml", "[colors]\naccent = '#00ff00'\n");

    let error = V2Theme::load(&tree.theme("a"), "a").unwrap_err();

    assert!(error.contains("Ciclo de herencia"), "{error}");
    assert!(error.contains("a -> b -> a"), "{error}");
}

#[test]
fn extends_missing_base_is_an_error() {
    let tree = ThemeTree::new();
    write_meta(&tree, "child", "Child", "extends = 'ghost'\n");
    tree.write("child", "Definition.toml", "[colors]\naccent = '#0000ff'\n");

    let error = V2Theme::load(&tree.theme("child"), "child").unwrap_err();

    assert!(error.contains("no está instalado"), "{error}");
}

#[test]
fn extends_rejects_path_traversal() {
    let tree = ThemeTree::new();
    write_meta(&tree, "child", "Child", "extends = '../evil'\n");
    tree.write("child", "Definition.toml", "[colors]\naccent = '#0000ff'\n");

    let error = V2Theme::load(&tree.theme("child"), "child").unwrap_err();

    assert!(error.contains("extends"), "{error}");
}

#[test]
fn extends_depth_is_limited() {
    let tree = ThemeTree::new();
    for index in 0..10 {
        let extra = if index == 9 {
            String::new()
        } else {
            format!("extends = 'theme{}'\n", index + 1)
        };
        write_meta(&tree, &format!("theme{index}"), &format!("T{index}"), &extra);
        tree.write(
            &format!("theme{index}"),
            "Definition.toml",
            "[colors]\naccent = '#ff0000'\n",
        );
    }

    let error = V2Theme::load(&tree.theme("theme0"), "theme0").unwrap_err();

    assert!(error.contains("demasiado profunda"), "{error}");
}

#[test]
fn layout_pack_css_is_loaded() {
    let tree = ThemeTree::new();
    write_meta(&tree, "pack", "Pack", "");
    tree.write(
        "pack",
        "Definition.toml",
        "[layout]\nsidebar-width = '260px'\n\n[layout_pack]\npath = 'Layout.css'\n",
    );
    tree.write(
        "pack",
        "Layout.css",
        "[data-part='app'] { flex-direction: row-reverse; }\n",
    );

    let res = V2Theme::load(&tree.theme("pack"), "pack").unwrap();

    let css = res.layout_css.expect("layout css");
    assert!(css.contains("data-part"));
    assert_eq!(
        res.variables.get("--sidebar-width").map(String::as_str),
        Some("260px")
    );
}

#[test]
fn layout_pack_missing_or_disabled_is_ignored() {
    let tree = ThemeTree::new();
    write_meta(&tree, "missing", "Missing", "");
    tree.write(
        "missing",
        "Definition.toml",
        "[layout_pack]\npath = 'Nope.css'\n",
    );
    assert!(
        V2Theme::load(&tree.theme("missing"), "missing")
            .unwrap()
            .layout_css
            .is_none()
    );

    write_meta(&tree, "off", "Off", "");
    tree.write(
        "off",
        "Definition.toml",
        "[layout_pack]\npath = 'Layout.css'\nenabled = false\n",
    );
    tree.write("off", "Layout.css", "body { color: red; }");
    assert!(
        V2Theme::load(&tree.theme("off"), "off")
            .unwrap()
            .layout_css
            .is_none()
    );
}
