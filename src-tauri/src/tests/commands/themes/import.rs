use super::*;
use std::io::Write;

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("cubic-theme-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn zip(&self, entries: &[(&str, &str)]) -> String {
        let path = self.0.join("input.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (name, content) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        path.to_str().unwrap().to_owned()
    }

    fn existing(&self) -> PathBuf {
        let destination = self.0.join("themes/test_author");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(destination.join("old.txt"), "keep me").unwrap();
        destination
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const V1: &str = r#"{"name":"Test","author":"Author","variables":{}}"#;
const V2: &str = "name = 'Test'\nauthor = 'Author'\ninjects_css = false\n";

fn imports_layout<T: ZipImportable>(prefix: &str, metadata: &str) {
    let dir = TestDir::new();
    let themes = dir.0.join("themes");
    let destination = dir.existing();
    let zip = dir.zip(&[
        (&format!("{prefix}{}", T::ZIP_TARGET_FILE), metadata),
        (
            &format!("{prefix}Definition.toml"),
            "[colors]\naccent = 'red'",
        ),
        (&format!("{prefix}Inject.css"), "body { color: red; }"),
        (&format!("{prefix}assets/nested/font.woff2"), "font bytes"),
        (&format!("{prefix}assets/BG.mp3"), "music bytes"),
    ]);
    let (entry, staged) = stage_zip::<T>(&zip, &themes).unwrap().unwrap();
    assert_eq!(entry.id, "test_author");
    assert_eq!(staged.root.parent(), Some(themes.as_path()));
    assert!(!staged.root.join("theme.json").exists());
    assert!(!staged.root.join("Meta.toml").exists());
    assert_eq!(std::fs::read_dir(&themes).unwrap().count(), 2);
    assert!(destination.join("old.txt").exists());
    let staging_root = staged.root.clone();
    staged.install(&destination).unwrap();
    drop(staged);
    assert!(!staging_root.exists());
    assert!(!destination.join("old.txt").exists());
    assert_eq!(
        std::fs::read_to_string(destination.join(T::ZIP_TARGET_FILE)).unwrap(),
        metadata
    );
    assert_eq!(
        std::fs::read_to_string(destination.join("Inject.css")).unwrap(),
        "body { color: red; }"
    );
    assert_eq!(
        std::fs::read_to_string(destination.join("assets/nested/font.woff2")).unwrap(),
        "font bytes"
    );
    assert_eq!(
        std::fs::read_to_string(destination.join("assets/BG.mp3")).unwrap(),
        "music bytes"
    );
}

#[test]
fn imports_v1_root() {
    imports_layout::<ThemeFile>("", V1);
}

#[test]
fn imports_v1_wrapped() {
    imports_layout::<ThemeFile>("wrapper/", V1);
    imports_layout::<ThemeFile>("./wrapper/", V1);
}

#[test]
fn imports_v2_root() {
    imports_layout::<ThemeMeta>("", V2);
}

#[test]
fn imports_v2_wrapped() {
    imports_layout::<ThemeMeta>("wrapper/nested/", V2);
}

#[test]
fn imports_legacy_windows_root_and_wrapped_archives() {
    for prefix in ["", "wrapper\\nested\\"] {
        for (manifest, content) in [("theme.json", V1), ("Meta.toml", V2)] {
            let dir = TestDir::new();
            let themes = dir.0.join("themes");
            let zip = dir.zip(&[
                (&format!("{prefix}{manifest}"), content),
                (&format!("{prefix}Definition.toml"), ""),
                (&format!("{prefix}assets\\nested\\"), ""),
                (&format!("{prefix}assets\\nested\\font.woff2"), "font bytes"),
            ]);
            let staged = if manifest == "theme.json" {
                stage_zip::<ThemeFile>(&zip, &themes)
            } else {
                stage_zip::<ThemeMeta>(&zip, &themes)
            }
            .unwrap()
            .unwrap()
            .1;
            assert_eq!(
                std::fs::read_to_string(staged.root.join("theme/assets/nested/font.woff2"))
                    .unwrap(),
                "font bytes"
            );
        }
    }
}

#[test]
fn exports_canonical_zip_names_and_roundtrips() {
    let dir = TestDir::new();
    let theme = dir.0.join("source");
    std::fs::create_dir_all(theme.join("assets/nested")).unwrap();
    std::fs::write(theme.join("theme.json"), V1).unwrap();
    std::fs::write(theme.join("assets/nested/font.woff2"), "font bytes").unwrap();
    let output = dir.0.join("export.zip");
    export_theme_directory(&theme, &output).unwrap();
    let archive = zip::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
    assert!(archive.file_names().all(|name| !name.contains('\\')));
    assert!(
        archive
            .file_names()
            .any(|name| name == "assets/nested/font.woff2")
    );
    let (_, staged) = stage_zip::<ThemeFile>(output.to_str().unwrap(), &dir.0.join("themes"))
        .unwrap()
        .unwrap();
    assert!(staged.root.join("theme/assets/nested/font.woff2").exists());
}

#[test]
fn runtime_metadata_precedence_preserves_existing_on_invalid_v2() {
    let dir = TestDir::new();
    let destination = dir.existing();
    let zip = dir.zip(&[
        ("theme.json", V1),
        ("Meta.toml", "[invalid"),
        ("Definition.toml", ""),
    ]);
    assert!(stage_zip::<ThemeFile>(&zip, &dir.0.join("themes")).is_err());
    assert_eq!(
        std::fs::read_to_string(destination.join("old.txt")).unwrap(),
        "keep me"
    );
}

#[test]
fn runtime_fatal_asset_checks_preserve_existing() {
    let dir = TestDir::new();
    let destination = dir.existing();
    for definition in [
        "[[fonts]]\nfamily = 'Unsafe'\nsrc = '../outside'",
        "[background]\nreference_path = '../outside'",
        "[icons]\npreview = '../outside'",
        "[icons.ui]\nplay = '../outside'",
    ] {
        let zip = dir.zip(&[("Meta.toml", V2), ("Definition.toml", definition)]);
        assert!(stage_zip::<ThemeMeta>(&zip, &dir.0.join("themes")).is_err());
        assert!(destination.join("old.txt").exists());
    }
    let zip = dir.zip(&[("theme.json", r#"{"name":"Test","author":"Author","variables":{},"fonts":[{"family":"Unsafe","src":"../outside"}]}"#)]);
    assert!(stage_zip::<ThemeFile>(&zip, &dir.0.join("themes")).is_err());
    assert!(destination.join("old.txt").exists());
}

#[test]
fn runtime_tolerates_missing_assets_and_keeps_inject_css_without_flag() {
    let dir = TestDir::new();
    let themes = dir.0.join("themes");
    let zip = dir.zip(&[
        ("theme.json", V1),
        ("Meta.toml", V2),
        ("Definition.toml", "[[fonts]]\nfamily = 'Missing'\nsrc = 'missing.woff2'\n[background]\nreference_path = 'missing.png'\n[icons]\npreview = 'missing.png'\n[icons.ui]\nplay = 'missing.svg'"),
        ("Inject.css", "body { color: red; }"),
    ]);
    // V1 discovery must still validate/load using runtime V2 precedence.
    let (entry, staged) = stage_zip::<ThemeFile>(&zip, &themes).unwrap().unwrap();
    let loaded = load_user_theme(&staged.root.join("theme"), &entry.id).unwrap();
    assert!(loaded.bg_image.is_none());
    assert!(loaded.icons.is_empty());
    assert_eq!(loaded.fonts.len(), 1);
    assert_eq!(loaded.inject_css.as_deref(), Some("body { color: red; }"));
    staged.install(&themes.join(entry.id.as_str())).unwrap();
    assert!(load_user_theme(&themes.join(entry.id.as_str()), &entry.id).is_ok());
}

#[test]
fn runtime_discovers_bg_mp3_without_manifest_configuration() {
    for (manifest, metadata, definition, location) in [
        ("theme.json", V1, None, "assets/BG.mp3"),
        (
            "Meta.toml",
            V2,
            Some("[background]\nreference_path = 'visuals/BG.png'"),
            "visuals/BG.mp3",
        ),
    ] {
        let dir = TestDir::new();
        let themes = dir.0.join("themes");
        let mut entries = vec![(manifest, metadata), (location, "music bytes")];
        if let Some(definition) = definition {
            entries.push(("Definition.toml", definition));
            entries.push(("visuals/BG.png", "GIF89a0000000000"));
        }
        let zip = dir.zip(&entries);
        let staged = if manifest == "theme.json" {
            stage_zip::<ThemeFile>(&zip, &themes)
        } else {
            stage_zip::<ThemeMeta>(&zip, &themes)
        }
        .unwrap()
        .unwrap()
        .1;
        let loaded = load_user_theme(&staged.root.join("theme"), "test_author").unwrap();
        let expected = staged
            .root
            .join("theme")
            .join(location)
            .to_string_lossy()
            .into_owned();
        assert_eq!(loaded.bg_music.as_deref(), Some(expected.as_str()));
    }
}

#[test]
fn runtime_ignores_other_music_names_and_bg_mp3_directories() {
    let dir = TestDir::new();
    let theme = dir.0.join("theme");
    std::fs::create_dir_all(theme.join("assets/BG.mp3")).unwrap();
    std::fs::write(theme.join("theme.json"), V1).unwrap();
    std::fs::write(theme.join("assets/music.mp3"), "music bytes").unwrap();
    let loaded = load_user_theme(&theme, "test_author").unwrap();
    assert!(loaded.bg_music.is_none());
}

#[test]
fn failed_extraction_preserves_existing_and_cleans_staging() {
    let dir = TestDir::new();
    let destination = dir.existing();
    let zip = dir.zip(&[
        ("theme.json", V1),
        ("assets", "not a directory"),
        ("assets/font", "font"),
    ]);
    assert!(stage_zip::<ThemeFile>(&zip, &dir.0.join("themes")).is_err());
    assert_eq!(
        std::fs::read_to_string(destination.join("old.txt")).unwrap(),
        "keep me"
    );
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 2);
    assert_eq!(std::fs::read_dir(dir.0.join("themes")).unwrap().count(), 1);
}

#[test]
fn corrupt_asset_preserves_existing() {
    let dir = TestDir::new();
    let destination = dir.existing();
    let path = dir.0.join("corrupt.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("theme.json", options).unwrap();
    zip.write_all(V1.as_bytes()).unwrap();
    zip.start_file("asset.bin", options).unwrap();
    zip.write_all(b"unique asset contents").unwrap();
    zip.finish().unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    let offset = bytes
        .windows(b"unique asset contents".len())
        .position(|window| window == b"unique asset contents")
        .unwrap();
    bytes[offset] ^= 1;
    std::fs::write(&path, bytes).unwrap();
    assert!(stage_zip::<ThemeFile>(path.to_str().unwrap(), &dir.0.join("themes")).is_err());
    assert_eq!(
        std::fs::read_to_string(destination.join("old.txt")).unwrap(),
        "keep me"
    );
}

#[test]
fn ambiguous_metadata_preserves_existing() {
    let dir = TestDir::new();
    let destination = dir.existing();
    for paths in [
        ["theme.json", "wrapped/theme.json"],
        ["a/theme.json", "b/theme.json"],
    ] {
        let zip = dir.zip(&[(paths[0], V1), (paths[1], V1)]);
        assert!(stage_zip::<ThemeFile>(&zip, &dir.0.join("themes")).is_err());
        assert!(destination.join("old.txt").exists());
    }
}

#[test]
fn invalid_definition_preserves_existing() {
    let dir = TestDir::new();
    let destination = dir.existing();
    for definition in [None, Some("[invalid")] {
        let mut entries = vec![("Meta.toml", V2)];
        if let Some(definition) = definition {
            entries.push(("Definition.toml", definition));
        }
        let zip = dir.zip(&entries);
        assert!(stage_zip::<ThemeMeta>(&zip, &dir.0.join("themes")).is_err());
        assert!(destination.join("old.txt").exists());
    }
}

#[test]
fn failed_replacement_rolls_back_existing() {
    let dir = TestDir::new();
    let destination = dir.existing();
    let staged = StagedTheme::new(&dir.0.join("themes")).unwrap();
    // Force the second rename to fail after the old installation is backed up.
    std::fs::remove_dir(staged.root.join("theme")).unwrap();
    assert!(staged.install(&destination).is_err());
    assert_eq!(
        std::fs::read_to_string(destination.join("old.txt")).unwrap(),
        "keep me"
    );
    assert!(!staged.root.join("backup").exists());
}

#[test]
fn unsafe_paths_and_ids_never_replace_existing() {
    let dir = TestDir::new();
    let destination = dir.existing();
    for path in [
        "../escaped",
        "/absolute",
        "assets/../../escaped",
        "assets/../escaped",
        "..\\escaped",
        "assets\\..\\escaped",
        "assets/..\\escaped",
        "\\absolute",
        "\\\\server\\share\\escaped",
        "C:/escaped",
        "C:\\escaped",
        "C:escaped",
    ] {
        let zip = dir.zip(&[("theme.json", V1), (path, "unsafe")]);
        assert!(
            stage_zip::<ThemeFile>(&zip, &dir.0.join("themes")).is_err(),
            "{path}"
        );
        assert!(destination.join("old.txt").exists());
        assert!(!dir.0.join("escaped").exists());
    }
    for manifest in [
        "../theme.json",
        "..\\theme.json",
        "wrapper\\..\\theme.json",
        "C:\\wrapper\\theme.json",
    ] {
        let zip = dir.zip(&[(manifest, V1)]);
        assert!(stage_zip::<ThemeFile>(&zip, &dir.0.join("themes")).is_err());
    }
    for name in ["../escape", "/absolute", "a\\b", ".."] {
        assert!(build_theme_id(name, "").is_err());
    }
}

#[test]
fn non_theme_zip_allows_v1_v2_fallback() {
    let dir = TestDir::new();
    let zip = dir.zip(&[("readme.txt", "not a theme")]);
    assert!(
        stage_zip::<ThemeFile>(&zip, &dir.0.join("themes"))
            .unwrap()
            .is_none()
    );
    assert!(
        stage_zip::<ThemeMeta>(&zip, &dir.0.join("themes"))
            .unwrap()
            .is_none()
    );
    assert!(!dir.0.join("themes").exists());
}

#[cfg(unix)]
#[test]
fn stages_on_symlinked_theme_filesystem_without_hiding_dot_themes() {
    let dir = TestDir::new();
    let storage = TestDir::new();
    let themes = dir.0.join("themes");
    std::os::unix::fs::symlink(&storage.0, &themes).unwrap();
    let zip = dir.zip(&[("theme.json", r#"{"name":".Hidden","variables":{}}"#)]);
    let (entry, staged) = stage_zip::<ThemeFile>(&zip, &themes).unwrap().unwrap();
    assert!(
        std::fs::canonicalize(&staged.root)
            .unwrap()
            .starts_with(std::fs::canonicalize(&storage.0).unwrap())
    );
    assert!(!staged.root.join("theme.json").exists());
    assert!(!staged.root.join("Meta.toml").exists());
    assert_eq!(entry.id, ".hidden");
    staged.install(&themes.join(entry.id.as_str())).unwrap();
    drop(staged);
    assert!(themes.join(".hidden/theme.json").exists());
    assert_eq!(std::fs::read_dir(&themes).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn refuses_symlink_destination_without_touching_target() {
    let dir = TestDir::new();
    let destination = dir.existing();
    let link = dir.0.join("themes/link");
    std::os::unix::fs::symlink(&destination, &link).unwrap();
    let staged = StagedTheme::new(&dir.0.join("themes")).unwrap();
    assert!(staged.install(&link).is_err());
    assert!(destination.join("old.txt").exists());
    assert!(link.is_symlink());
}
