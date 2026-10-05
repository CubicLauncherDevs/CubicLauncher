use super::*;
use crate::services::instance_import::sanitize_instance_name;

#[test]
fn test_detect_cubic_format() {
    let temp =
        std::env::temp_dir().join(format!("cubic_import_detect_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp).unwrap();
    std::fs::write(temp.join("cubic-manifest.json"), r#"{"format_version":1,"exported_by":"CubicLauncher","uuid":"u","name":"Test","version_id":"1.21","mc_version":"1.21","loader":"Vanilla","loader_version":null,"min_memory":512,"max_memory":2048,"overrides":null}"#).unwrap();

    let provider = CubicProvider;
    assert!(provider.detect(&temp));

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_preview_cubic_instance() {
    let temp = std::env::temp_dir().join(format!(
        "cubic_import_preview_test_{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&temp).unwrap();
    std::fs::write(temp.join("cubic-manifest.json"), r#"{"format_version":1,"exported_by":"CubicLauncher","uuid":"u","name":"Mi Instancia","version_id":"fabric-loader-0.15.0-1.21","mc_version":"1.21","loader":"Fabric","loader_version":"0.15.0","min_memory":1024,"max_memory":4096,"overrides":null}"#).unwrap();

    let provider = CubicProvider;
    let plan = provider.preview(&temp).unwrap();

    assert_eq!(plan.format_id, "cubic");
    assert_eq!(plan.minecraft_version.as_deref(), Some("1.21"));
    assert_eq!(plan.loader.as_deref(), Some("Fabric"));
    assert_eq!(plan.loader_version.as_deref(), Some("0.15.0"));
    assert_eq!(plan.sanitized_name, sanitize_instance_name("Mi Instancia"));
    assert!(read_manifest(&temp).unwrap().modpack.is_none());

    let _ = std::fs::remove_dir_all(&temp);
}

fn manifest_with_pack(state: Option<&PackState>) -> serde_json::Value {
    serde_json::json!({
        "format_version": 1, "exported_by": "CubicLauncher", "uuid": "u",
        "name": "Pack", "version_id": "1.21", "mc_version": "1.21",
        "loader": "Vanilla", "loader_version": null,
        "min_memory": 512, "max_memory": 2048, "overrides": null,
        "modpack": state,
    })
}

fn inventoried_pack() -> PackState {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1, "source": "curseforge", "project_id": "42",
        "version_id": "99", "name": "My pack", "version": "v2",
        "game_version": "1.21", "locked": true,
        "files": {
            "mods/owned.jar": { "sha1": "0123456789abcdef0123456789abcdef01234567", "size": 123, "downloads": ["https://example.org/original.jar"] },
            "extra/nested/pack.txt": { "sha1": "abcdef0123456789abcdef0123456789abcdef0123", "size": 456 },
            "root-pack.txt": { "sha1": "abcdef0123456789abcdef0123456789abcdef0123", "size": 456 },
            "missing.txt": { "sha1": "abcdef0123456789abcdef0123456789abcdef0123", "size": 456 }
        },
        "identities": { "mods/owned.jar": ["original_mod"] },
        "needs_inventory": false,
    })).unwrap()
}

#[test]
fn backup_round_trip_restores_provenance_and_customized_pack_files() {
    use crate::services::instance_export::{ExportInput, export_to_zip};
    let source = tempfile::tempdir().unwrap();
    let extracted = tempfile::tempdir().unwrap();
    let restored = tempfile::tempdir().unwrap();
    let state = inventoried_pack();
    for relative in state.files.keys().filter(|path| *path != "missing.txt") {
        let path = source.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"custom content differs from original hash and size").unwrap();
    }
    let input = ExportInput {
        uuid: "uuid".into(),
        name: "Pack".into(),
        version_id: "1.21".into(),
        mc_version: "1.21".into(),
        loader_name: "Vanilla".into(),
        loader_version: None,
        loader_mmc_uid: None,
        instance_dir: source.path().to_owned(),
        min_memory: 512,
        max_memory: 2048,
        overrides: None,
        minecraft_jar: Default::default(),
        icon_src: None,
        modpack: Some(state.clone()),
    };
    let backup = source.path().join("backup.zip");
    export_to_zip(&input, &backup).unwrap();
    zip::ZipArchive::new(std::fs::File::open(backup).unwrap())
        .unwrap()
        .extract(extracted.path())
        .unwrap();
    let imported = read_manifest(extracted.path()).unwrap().modpack.unwrap();
    assert_eq!(imported, state);
    restore_modpack(
        &resolve_game_dir(extracted.path()),
        restored.path(),
        &imported,
    )
    .unwrap();
    assert_eq!(modpack::load(restored.path()).unwrap(), Some(state.clone()));
    for relative in state.files.keys().filter(|path| *path != "missing.txt") {
        assert_eq!(
            std::fs::read(restored.path().join(relative)).unwrap(),
            std::fs::read(source.path().join(relative)).unwrap()
        );
    }
    assert!(!restored.path().join("missing.txt").exists());
}

#[tokio::test]
async fn legacy_pack_keeps_metadata_and_needs_inventory() {
    let source = tempfile::tempdir().unwrap();
    let restored = tempfile::tempdir().unwrap();
    std::fs::write(
        source.path().join("upstream.json"),
        r#"{"type":"curseforge-modpack","projectId":42,"fileId":99}"#,
    )
    .unwrap();
    let state = modpack::read_state(source.path()).await.unwrap().unwrap();
    assert!(state.needs_inventory);
    assert!(state.locked);
    assert_eq!(state.project_id.as_deref(), Some("42"));
    assert_eq!(state.version_id.as_deref(), Some("99"));
    std::fs::write(
        source.path().join("cubic-manifest.json"),
        manifest_with_pack(Some(&state)).to_string(),
    )
    .unwrap();
    let imported = read_manifest(source.path()).unwrap().modpack.unwrap();
    restore_modpack(source.path(), restored.path(), &imported).unwrap();
    assert!(restored.path().join(modpack::STATE_FILE).is_file());
    assert_eq!(
        modpack::read_state(restored.path()).await.unwrap(),
        Some(state)
    );
}

#[test]
fn preview_rejects_invalid_pack_schema_and_paths() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = inventoried_pack();
    state.schema_version = 2;
    std::fs::write(
        temp.path().join("cubic-manifest.json"),
        manifest_with_pack(Some(&state)).to_string(),
    )
    .unwrap();
    assert!(matches!(
        CubicProvider.preview(temp.path()),
        Err(ImportError::InvalidArchive(_))
    ));
    state.schema_version = 1;
    let file = state.files.values().next().unwrap().clone();
    for relative in [
        "../outside",
        "/absolute",
        "mods\\escape.jar",
        "mods//empty",
        "instance.cub",
        "modpack.cub.json",
        "config/.modpack-staging/file",
        "config/cache.crep",
        "cubic-jar/client.jar",
    ] {
        state.files = [(relative.into(), file.clone())].into();
        std::fs::write(
            temp.path().join("cubic-manifest.json"),
            manifest_with_pack(Some(&state)).to_string(),
        )
        .unwrap();
        assert!(
            matches!(
                CubicProvider.preview(temp.path()),
                Err(ImportError::InvalidArchive(_))
            ),
            "{relative}"
        );
    }
}

#[cfg(unix)]
#[test]
fn restore_rejects_linked_pack_file_ancestors() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("pack.txt"), b"external").unwrap();
    std::os::unix::fs::symlink(outside.path(), source.path().join("extra")).unwrap();
    assert!(restore_modpack(source.path(), target.path(), &inventoried_pack()).is_err());
    assert!(!target.path().join(modpack::STATE_FILE).exists());
}
