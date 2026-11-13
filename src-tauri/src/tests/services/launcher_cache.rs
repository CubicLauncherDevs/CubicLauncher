use super::*;

fn cache_file(root: &Path, kind: &str, name: &str) -> PathBuf {
    let dir = root.join(kind);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, b"0123456789").unwrap();
    path
}

#[test]
fn breakdown_reports_every_cache_kind() {
    let root = tempfile::tempdir().unwrap();
    cache_file(root.path(), "manifests", "manifest.crep");
    cache_file(root.path(), "i18n", "es-ES.json");
    cache_file(root.path(), "market-icons", "abc");

    let (total, items) = breakdown(root.path());

    let names: Vec<&str> = items.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["i18n", "manifests", "market-icons"]);
    assert_eq!(total.files, 3);
    assert_eq!(total.bytes, 30);
    assert!(!total.blocked);
}

#[test]
fn breakdown_ignores_a_missing_cache_directory() {
    let root = tempfile::tempdir().unwrap();
    let (total, items) = breakdown(&root.path().join("missing"));
    assert_eq!(items.len(), 0);
    assert_eq!((total.bytes, total.files), (0, 0));
}

#[test]
fn purge_removes_every_kind_and_reports_what_it_freed() {
    let root = tempfile::tempdir().unwrap();
    cache_file(root.path(), "manifests", "manifest.crep");
    cache_file(root.path(), "market-icons", "abc");

    let usage = clear_cache_at(root.path().to_path_buf()).unwrap();

    assert_eq!(usage.files, 2);
    assert_eq!(usage.bytes, 20);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    // La carpeta en sí se conserva: el protocolo la vuelve a usar.
    assert!(root.path().is_dir());
}

#[test]
fn purging_an_empty_cache_is_a_noop() {
    let root = tempfile::tempdir().unwrap();
    let usage = clear_cache_at(root.path().to_path_buf()).unwrap();
    assert_eq!((usage.bytes, usage.files), (0, 0));
}
