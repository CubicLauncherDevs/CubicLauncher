use super::*;
use std::path::{Path, PathBuf};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("cubic-theme-list-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn list_prefers_v2_when_both_manifests_exist() {
    let dir = TestDir::new();
    std::fs::write(
        dir.path().join("Meta.toml"),
        "name = 'V2'\nauthor = 'Author'\nversion = '2.0'\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Definition.toml"),
        "[colors]\naccent = '#ff0000'\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("theme.json"),
        r#"{"name":"V1","author":"Author","variables":{}}"#,
    )
    .unwrap();

    let entry = list_theme_entry(dir.path(), "test_author").expect("theme entry");
    assert_eq!(entry.r#type.as_str(), "v2");
    assert_eq!(entry.name.as_str(), "V2");
}

#[test]
fn list_falls_back_to_v1() {
    let dir = TestDir::new();
    std::fs::write(
        dir.path().join("theme.json"),
        r##"{"name":"V1","author":"Author","type":"user","variables":{"--accent":"#fff"}}"##,
    )
    .unwrap();

    let entry = list_theme_entry(dir.path(), "test_author").expect("theme entry");
    assert_eq!(entry.r#type.as_str(), "user");
    assert_eq!(entry.name.as_str(), "v1");
}

#[test]
fn list_ignores_directories_without_manifest() {
    let dir = TestDir::new();
    assert!(list_theme_entry(dir.path(), "test_author").is_none());
}
