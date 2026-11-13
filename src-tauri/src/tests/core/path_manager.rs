use super::*;

fn dirs() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let settings = root.path().join("settings");
    let cache = root.path().join("cache");
    std::fs::create_dir_all(&settings).unwrap();
    (root, settings, cache)
}

#[test]
fn moves_legacy_manifest_caches_out_of_settings() {
    let (_root, settings, cache) = dirs();
    std::fs::write(settings.join("manifest.crep"), b"manifest").unwrap();
    std::fs::write(settings.join("quilt.crep"), b"quilt").unwrap();

    migrate_legacy_cache(&settings, &cache);

    assert!(!settings.join("manifest.crep").exists());
    assert_eq!(
        std::fs::read(cache.join("manifests").join("manifest.crep")).unwrap(),
        b"manifest"
    );
    assert_eq!(
        std::fs::read(cache.join("manifests").join("quilt.crep")).unwrap(),
        b"quilt"
    );
}

#[test]
fn moves_the_locale_directory() {
    let (_root, settings, cache) = dirs();
    std::fs::create_dir_all(settings.join("i18n")).unwrap();
    std::fs::write(settings.join("i18n").join("es-ES.json"), b"{}").unwrap();

    migrate_legacy_cache(&settings, &cache);

    assert!(!settings.join("i18n").exists());
    assert!(cache.join("i18n").join("es-ES.json").exists());
}

#[test]
fn never_overwrites_a_cache_that_is_already_in_place() {
    let (_root, settings, cache) = dirs();
    std::fs::create_dir_all(cache.join("manifests")).unwrap();
    std::fs::write(settings.join("manifest.crep"), b"stale").unwrap();
    std::fs::write(cache.join("manifests").join("manifest.crep"), b"current").unwrap();

    migrate_legacy_cache(&settings, &cache);

    assert_eq!(
        std::fs::read(cache.join("manifests").join("manifest.crep")).unwrap(),
        b"current"
    );
    assert!(settings.join("manifest.crep").exists());
}

#[test]
fn ignores_missing_legacy_files() {
    let (_root, settings, cache) = dirs();

    migrate_legacy_cache(&settings, &cache);

    assert!(!cache.join("manifests").exists());
}
