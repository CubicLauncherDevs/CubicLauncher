use super::*;
use std::path::{Path, PathBuf};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("cubic-theme-validate-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, name: &str, content: &str) {
        let path = self.0.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const META: &str = "name = 'Valid'\nauthor = 'Author'\nversion = '1.0'\ninjects_css = true\n";
const DEFINITION: &str = "[colors]\naccent = '#ff0000'\n";

#[test]
fn valid_v2_theme_has_no_errors() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write("Definition.toml", DEFINITION);
    dir.write("Inject.css", "body { color: red; }");

    let report = validate_theme_at(dir.path(), "valid_author");
    assert_eq!(report.format, "v2");
    assert_eq!(report.name.as_deref(), Some("Valid"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

#[test]
fn builtin_themes_validate() {
    let themes_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../static/themes");
    let mut checked = 0;

    for entry in std::fs::read_dir(&themes_dir).expect("built-in themes dir") {
        let path = entry.expect("theme entry").path();
        if !path.is_dir() {
            continue;
        }
        let id = path.file_name().unwrap().to_string_lossy().into_owned();
        let manifest = path.join(format!("{id}.json"));
        let mut report = ThemeValidationReport::new(&id);
        report.format = "v1".to_string();
        report.warning("Formato V1 legacy.");
        validate_v1(&path, &manifest, &mut report);
        assert!(report.errors.is_empty(), "tema {id}: {:?}", report.errors);
        checked += 1;
    }

    assert!(checked > 0, "no se encontraron temas built-in");
}

#[test]
fn invalid_definition_reports_error() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write("Definition.toml", "[colors\n");

    let report = validate_theme_at(dir.path(), "valid_author");
    assert_eq!(report.format, "v2");
    assert!(!report.errors.is_empty());
}

#[test]
fn missing_manifests_reports_error() {
    let dir = TestDir::new();
    let report = validate_theme_at(dir.path(), "empty_author");
    assert_eq!(report.format, "unknown");
    assert!(!report.errors.is_empty());
}

#[test]
fn v1_theme_is_flagged_as_legacy() {
    let dir = TestDir::new();
    dir.write(
        "theme.json",
        r#"{"name":"Legacy","author":"Author","variables":{}}"#,
    );

    let report = validate_theme_at(dir.path(), "legacy_author");
    assert_eq!(report.format, "v1");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.to_lowercase().contains("legacy"))
    );
}

#[test]
fn missing_assets_produce_warnings() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write(
        "Definition.toml",
        r#"
[background]
reference_path = "missing-bg.webp"

[[fonts]]
family = "Missing"
src = "missing-font.woff2"

[icons]
preview = "missing-preview.png"

[icons.ui]
play = "missing-play.svg"
"#,
    );

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(report.warnings.len() >= 4, "{:?}", report.warnings);
}

#[test]
fn injects_css_false_warns_when_css_exists() {
    let dir = TestDir::new();
    dir.write(
        "Meta.toml",
        "name = 'Valid'\nauthor = 'Author'\nversion = '1.0'\ninjects_css = false\n",
    );
    dir.write("Definition.toml", DEFINITION);
    dir.write("Inject.css", "body { color: red; }");

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("[inject]"))
    );
}

#[test]
fn unknown_fields_produce_warnings() {
    let dir = TestDir::new();
    dir.write(
        "Meta.toml",
        "name = 'Valid'\nauthor = 'Author'\nunexpected = 'value'\n",
    );
    dir.write(
        "Definition.toml",
        "[colors]\naccent = '#ff0000'\n\n[background]\nmystery = 1\n\n[typography]\nsize_xl = '20px'\n\n[inject]\nunknown = true\n",
    );

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    for field in [
        "unexpected",
        "background.mystery",
        "typography.size_xl",
        "inject.unknown",
    ] {
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains(field)),
            "missing warning for {field}: {:?}",
            report.warnings
        );
    }
}

#[test]
fn invalid_token_names_are_errors() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write(
        "Definition.toml",
        "[tokens]\naccent = '#ff0000'\n'--valid' = '1'\n",
    );

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("token 'accent'"))
    );
}

#[test]
fn token_collisions_produce_warnings() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write(
        "Definition.toml",
        "[colors]\naccent = '#ff0000'\n\n[tokens]\n'--accent' = '#00ff00'\n",
    );

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("colisiona") && warning.contains("--accent"))
    );
}

#[test]
fn unsupported_format_is_an_error() {
    let dir = TestDir::new();
    dir.write(
        "Meta.toml",
        "name = 'Valid'\nauthor = 'Author'\nformat = 3\n",
    );
    dir.write("Definition.toml", DEFINITION);

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(report.errors.iter().any(|error| error.contains("format")));
}

#[test]
fn inject_disabled_skips_css_validation() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write(
        "Definition.toml",
        "[inject]\npath = 'missing.css'\nenabled = false\n",
    );

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(
        !report
            .warnings
            .iter()
            .any(|warning| warning.contains("missing.css"))
    );
}

#[test]
fn inject_custom_path_and_layer_are_validated() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write(
        "Definition.toml",
        "[inject]\npath = 'assets/custom.css'\nenabled = true\nlayer = 'unlayered'\n",
    );
    dir.write("assets/custom.css", "body { color: red; }");

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(
        !report
            .warnings
            .iter()
            .any(|warning| warning.contains("layer"))
    );
}

#[test]
fn inject_invalid_layer_warns() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write(
        "Definition.toml",
        "[inject]\nenabled = true\nlayer = 'component'\n",
    );

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("layer 'component'"))
    );
}

#[test]
fn injects_css_false_disables_inject_when_enabled_is_absent() {
    let dir = TestDir::new();
    dir.write(
        "Meta.toml",
        "name = 'Valid'\nauthor = 'Author'\ninjects_css = false\n",
    );
    dir.write("Definition.toml", "[inject]\npath = 'missing.css'\n");

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(
        !report
            .warnings
            .iter()
            .any(|warning| warning.contains("missing.css"))
    );
}

#[test]
fn inject_enabled_overrides_injects_css_false() {
    let dir = TestDir::new();
    dir.write(
        "Meta.toml",
        "name = 'Valid'\nauthor = 'Author'\ninjects_css = false\n",
    );
    dir.write(
        "Definition.toml",
        "[inject]\npath = 'missing.css'\nenabled = true\n",
    );

    let report = validate_theme_at(dir.path(), "valid_author");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("missing.css"))
    );
}

// ── Herencia (extends) y layout packs ───────────────────────────────────────

fn write_theme(dir: &TestDir, theme: &str, meta: &str, definition: &str) {
    dir.write(&format!("{theme}/Meta.toml"), meta);
    dir.write(&format!("{theme}/Definition.toml"), definition);
}

#[test]
fn valid_extends_has_no_errors() {
    let dir = TestDir::new();
    write_theme(
        &dir,
        "base",
        "name = 'Base'\nauthor = 'Author'\n",
        "[colors]\naccent = '#ff0000'\n",
    );
    write_theme(
        &dir,
        "child",
        "name = 'Child'\nauthor = 'Author'\nextends = 'base'\n",
        "[colors]\naccent = '#0000ff'\n",
    );

    let report = validate_theme_at(&dir.path().join("child"), "child");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

#[test]
fn extends_cycle_reports_error() {
    let dir = TestDir::new();
    write_theme(
        &dir,
        "a",
        "name = 'A'\nauthor = 'Author'\nextends = 'b'\n",
        "[colors]\naccent = '#ff0000'\n",
    );
    write_theme(
        &dir,
        "b",
        "name = 'B'\nauthor = 'Author'\nextends = 'a'\n",
        "[colors]\naccent = '#00ff00'\n",
    );

    let report = validate_theme_at(&dir.path().join("a"), "a");
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("Ciclo de herencia")),
        "{:?}",
        report.errors
    );
}

#[test]
fn extends_missing_base_reports_error() {
    let dir = TestDir::new();
    write_theme(
        &dir,
        "child",
        "name = 'Child'\nauthor = 'Author'\nextends = 'ghost'\n",
        "[colors]\naccent = '#0000ff'\n",
    );

    let report = validate_theme_at(&dir.path().join("child"), "child");
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("'ghost' no está instalado")),
        "{:?}",
        report.errors
    );
}

#[test]
fn layout_pack_missing_reports_warning() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write("Definition.toml", "[layout_pack]\npath = 'Nope.css'\n");

    let report = validate_theme_at(dir.path(), "pack_author");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("Nope.css")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn valid_layout_pack_has_no_errors() {
    let dir = TestDir::new();
    dir.write("Meta.toml", META);
    dir.write("Definition.toml", "[layout_pack]\npath = 'Layout.css'\n");
    dir.write(
        "Layout.css",
        "[data-part='app'] { flex-direction: row-reverse; }\n",
    );

    let report = validate_theme_at(dir.path(), "pack_author");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}
