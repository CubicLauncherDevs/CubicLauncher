use super::*;
use std::io::Write;

#[test]
fn sodium_beta_for_mc_26_2_accepts_fabric_despite_display_name_casing() {
    let game = zellkern::GameVersion {
        mc_version: "26.2".into(),
        loader: zellkern::Loader::Fabric("0.19.0".into()),
    };
    let version = serde_json::json!({
        "name": "Sodium Fabric 0.9.2 beta 1 mc 26.2",
        "version_type": "beta",
        "game_versions": ["26.2"],
        "loaders": ["fabric"]
    });
    assert!(modrinth_compatible(&version, &game));
}

#[test]
fn modrinth_loader_matching_ignores_case_but_keeps_game_and_loader_requirements() {
    for (loader, name) in [
        (zellkern::Loader::Fabric("1".into()), "fabric"),
        (zellkern::Loader::Forge("1".into()), "forge"),
        (zellkern::Loader::NeoForge("1".into()), "neoforge"),
        (zellkern::Loader::Quilt("1".into()), "quilt"),
    ] {
        let game = zellkern::GameVersion {
            mc_version: "26.2".into(),
            loader,
        };
        for published_name in [
            name.to_string(),
            name.to_uppercase(),
            game.loader.name().to_string(),
        ] {
            assert!(modrinth_compatible(
                &serde_json::json!({
                    "game_versions": ["26.2"], "loaders": [published_name]
                }),
                &game
            ));
        }
        assert!(!modrinth_compatible(
            &serde_json::json!({
                "game_versions": ["26.1"], "loaders": [name]
            }),
            &game
        ));
        assert!(!modrinth_compatible(
            &serde_json::json!({
                "game_versions": ["26.2"], "loaders": ["unrelated"]
            }),
            &game
        ));
        assert!(!modrinth_compatible(
            &serde_json::json!({ "game_versions": ["26.2"] }),
            &game
        ));
        assert!(!modrinth_compatible(
            &serde_json::json!({ "loaders": [name] }),
            &game
        ));
    }
}

fn jar(path: &Path, id: &str, version: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    zip.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    write!(
        zip,
        "{}",
        serde_json::json!({"schemaVersion":1,"id":id,"name":id,"version":version})
    )
    .unwrap();
    zip.finish().unwrap();
}

fn fixture(
    disabled: bool,
) -> (
    tempfile::TempDir,
    ReplaceRequest,
    Vec<RemoteFile>,
    Vec<InstalledFile>,
) {
    let temp = tempfile::tempdir().unwrap();
    let filename = if disabled {
        "old.jar.disabled"
    } else {
        "old.jar"
    };
    jar(&temp.path().join("mods").join(filename), "sample", "1");
    jar(&temp.path().join("stage/new.jar"), "sample", "2");
    jar(&temp.path().join("mods/user.jar"), "other", "1");
    let target = VersionRef {
        source: "modrinth".into(),
        project_id: "sample".into(),
        version_id: "v2".into(),
    };
    let request = ReplaceRequest {
        filename: filename.into(),
        expected_sha1: compute_file_sha1(&temp.path().join("mods").join(filename)).unwrap(),
        target: target.clone(),
        downloads: vec![target.clone()],
    };
    let remote = RemoteFile {
        version: target,
        filename: "new.jar".into(),
        url: "https://example.invalid/new.jar".into(),
        sha1: compute_file_sha1(&temp.path().join("stage/new.jar")).unwrap(),
        size: std::fs::metadata(temp.path().join("stage/new.jar"))
            .unwrap()
            .len(),
    };
    let installed = scan(temp.path()).unwrap();
    (temp, request, vec![remote], installed)
}

#[test]
fn replacement_preserves_disabled_state_and_unrelated_mods_and_can_roll_back() {
    for disabled in [false, true] {
        let (temp, request, files, installed) = fixture(disabled);
        let (name, operations) = plan_changes(
            temp.path(),
            &temp.path().join("stage"),
            &request,
            &files,
            &installed,
        )
        .unwrap();
        assert_eq!(
            name,
            if disabled {
                "new.jar.disabled"
            } else {
                "new.jar"
            }
        );
        let old = std::fs::read(temp.path().join("mods").join(&request.filename)).unwrap();
        let user = std::fs::read(temp.path().join("mods/user.jar")).unwrap();
        modpack_update::stage_file_changes(temp.path(), &operations).unwrap();
        assert!(!temp.path().join("mods").join(&request.filename).exists());
        assert_eq!(
            compute_file_sha1(&temp.path().join("mods").join(&name)).unwrap(),
            files[0].sha1
        );
        assert_eq!(
            std::fs::read(temp.path().join("mods/user.jar")).unwrap(),
            user
        );
        modpack_update::recover(temp.path()).unwrap();
        assert_eq!(
            std::fs::read(temp.path().join("mods").join(&request.filename)).unwrap(),
            old
        );
        assert!(!temp.path().join("mods").join(&name).exists());
    }
}

#[test]
fn same_filename_is_replaced_and_cache_identifies_the_exact_version() {
    let (temp, request, mut files, installed) = fixture(false);
    std::fs::rename(
        temp.path().join("stage/new.jar"),
        temp.path().join("stage/old.jar"),
    )
    .unwrap();
    files[0].filename = "old.jar".into();
    let (name, operations) = plan_changes(
        temp.path(),
        &temp.path().join("stage"),
        &request,
        &files,
        &installed,
    )
    .unwrap();
    assert_eq!(operations.len(), 1);
    modpack_update::stage_file_changes(temp.path(), &operations).unwrap();
    modpack_update::commit_file_changes(temp.path()).unwrap();
    cache_versions(temp.path(), &files, &name, &request.target).unwrap();
    let fresh = scan(temp.path()).unwrap();
    let updated = fresh.iter().find(|f| f.filename == name).unwrap();
    assert_eq!(updated.source.version_id(), Some("v2"));
    assert_eq!(updated.source.project_id(), Some("sample"));
    assert_eq!(updated.sha1, files[0].sha1);
    assert!(!temp.path().join(modpack_update::JOURNAL_DIR).exists());
}

#[test]
fn stale_original_collision_and_duplicate_versions_fail_before_mutating() {
    let (temp, mut request, files, installed) = fixture(false);
    request.expected_sha1 = "0".repeat(40);
    assert!(
        plan_changes(
            temp.path(),
            &temp.path().join("stage"),
            &request,
            &files,
            &installed
        )
        .is_err()
    );
    request.expected_sha1 = installed
        .iter()
        .find(|f| f.filename == "old.jar")
        .unwrap()
        .sha1
        .clone();
    jar(&temp.path().join("mods/new.jar"), "unrelated", "1");
    assert!(
        plan_changes(
            temp.path(),
            &temp.path().join("stage"),
            &request,
            &files,
            &scan(temp.path()).unwrap()
        )
        .is_err()
    );
    jar(&temp.path().join("mods/new.jar"), "sample", "0");
    assert!(
        plan_changes(
            temp.path(),
            &temp.path().join("stage"),
            &request,
            &files,
            &scan(temp.path()).unwrap()
        )
        .is_err()
    );
    assert!(temp.path().join("mods/old.jar").is_file());
    assert!(!temp.path().join(modpack_update::JOURNAL_DIR).exists());
}

#[test]
fn own_mod_updates_never_replace_pack_files_even_when_unlocked() {
    let (temp, request, files, installed) = fixture(false);
    let state = modpack::PackState {
        schema_version: 1,
        source: "local".into(),
        project_id: None,
        version_id: None,
        name: "Pack".into(),
        author: None,
        version: "1".into(),
        game_version: "1.21.1".into(),
        locked: false,
        needs_inventory: false,
        files: BTreeMap::from([(
            "mods/old.jar".into(),
            modpack::PackFile {
                sha1: request.expected_sha1.clone(),
                size: 0,
                downloads: vec![],
            },
        )]),
        identities: BTreeMap::new(),
    };
    modpack::save(temp.path(), &state).unwrap();
    assert!(
        plan_changes(
            temp.path(),
            &temp.path().join("stage"),
            &request,
            &files,
            &installed
        )
        .unwrap_err()
        .contains("Pack")
    );
    assert!(!temp.path().join(modpack_update::JOURNAL_DIR).exists());
}

#[test]
fn required_and_selected_optional_dependencies_are_included_but_embedded_are_not() {
    let node = |name: &str, kind| ResolvedDependency {
        source: DependencySource::Modrinth,
        project_id: name.into(),
        version_id: Some("1".into()),
        title: name.into(),
        icon_url: None,
        filename: format!("{name}.jar"),
        download_url: None,
        kind,
        depth: 0,
        children: vec![],
    };
    let optional = node("optional", DependencyKind::Optional);
    let nodes = vec![
        node("required", DependencyKind::Required),
        optional.clone(),
        node("embedded", DependencyKind::Embedded),
        node("incompatible", DependencyKind::Incompatible),
    ];
    let mut needed = BTreeSet::new();
    required_versions(&nodes, &BTreeSet::new(), &mut needed).unwrap();
    assert_eq!(needed.len(), 1);
    required_versions(
        &nodes,
        &BTreeSet::from([node_ref(&optional).unwrap()]),
        &mut needed,
    )
    .unwrap();
    assert_eq!(needed.len(), 2);
}
