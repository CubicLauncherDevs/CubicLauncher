use super::*;
use serde_json::json;
use zip::write::SimpleFileOptions;

fn fixture() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(".modpack-test-")
        .tempdir_in(env!("CARGO_MANIFEST_DIR"))
        .unwrap()
}

fn put(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn file(bytes: &[u8]) -> PackFile {
    PackFile {
        sha1: Sha1::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        downloads: vec![],
        size: bytes.len() as u64,
    }
}

fn state(entries: &[(&str, &[u8])]) -> PackState {
    PackState {
        schema_version: 1,
        source: "local".into(),
        project_id: None,
        version_id: None,
        name: "Inventory fixture".into(),
        author: None,
        version: "1.0".into(),
        game_version: "1.21.1".into(),
        locked: true,
        files: entries
            .iter()
            .map(|(p, b)| ((*p).into(), file(b)))
            .collect(),
        needs_inventory: false,
        identities: BTreeMap::new(),
    }
}

fn mrpack(root: &Path, files: serde_json::Value, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = root.join("fixture.mrpack");
    let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("modrinth.index.json", options).unwrap();
    serde_json::to_writer(
        &mut zip,
        &json!({
            "game": "minecraft", "formatVersion": 1, "versionId": "2.0",
            "name": "Archive fixture", "dependencies": {"minecraft": "1.21.1"},
            "files": files
        }),
    )
    .unwrap();
    for (name, bytes) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
    path
}

fn index_file(path: &str, bytes: &[u8], client: &str) -> serde_json::Value {
    json!({
        "path": path, "hashes": {"sha1": file(bytes).sha1},
        // inspect_archive must only inventory these URLs, never request them.
        "downloads": ["https://example.invalid/fixture.jar"],
        "fileSize": bytes.len(), "env": {"client": client, "server": "required"}
    })
}

fn mod_dto(filename: &str, sha1: &str) -> ModDto {
    ModDto {
        icon_revision: Some("local-icon-revision".into()),
        name: "Local metadata".into(),
        filename: filename.into(),
        version: Some("1.0".into()),
        description: Some("Local description".into()),
        authors: Some(vec!["Author".into()]),
        icon: None,
        enabled: !filename.ends_with(".disabled"),
        sha1: sha1.into(),
        file_size: 3,
        source: "local".into(),
        project_id: None,
        version_id: None,
        slug: None,
        pack_name: None,
        pack_locked: false,
        pack_modified: false,
    }
}

#[tokio::test]
async fn annotation_tracks_ownership_modifications_and_disabled_aliases_without_losing_metadata() {
    let temp = fixture();
    let mut inventory = state(&[
        ("mods/clean.jar", b"abc"),
        ("mods/edited.jar", b"abc"),
        ("mods/off.jar", b"abc"),
    ]);
    for locked in [true, false] {
        inventory.locked = locked;
        save(temp.path(), &inventory).unwrap();
        let hash = file(b"abc").sha1;
        let mut mods = vec![
            mod_dto("clean.jar", &hash.to_ascii_uppercase()),
            mod_dto("edited.jar", &file(b"xyz").sha1),
            mod_dto("off.jar.disabled", &hash),
            mod_dto("user.jar", &hash),
        ];
        let before = serde_json::to_value(&mods).unwrap();
        annotate_mods(temp.path(), "mods", &mut mods).await.unwrap();
        for (i, item) in mods.iter().enumerate() {
            assert_eq!(
                item.pack_name.as_deref(),
                (i < 3).then_some("Inventory fixture")
            );
            assert_eq!(item.pack_locked, i < 3 && locked);
            assert_eq!(item.pack_modified, i == 1 || i == 2);
            let mut expected = before[i].clone();
            expected["pack_name"] = serde_json::to_value(&item.pack_name).unwrap();
            expected["pack_locked"] = item.pack_locked.into();
            expected["pack_modified"] = item.pack_modified.into();
            assert_eq!(serde_json::to_value(item).unwrap(), expected);
        }
        let mut other_folder = vec![mod_dto("clean.jar", &hash)];
        annotate_mods(temp.path(), "resourcepacks", &mut other_folder)
            .await
            .unwrap();
        assert!(other_folder[0].pack_name.is_none());
        assert!(!other_folder[0].pack_locked);
    }
}

#[tokio::test]
async fn mrpack_inventory_uses_client_overrides_regardless_of_zip_order() {
    let temp = fixture();
    for client_first in [true, false] {
        let mut entries: Vec<(&str, &[u8])> = vec![
            ("client-overrides/config/common.json", b"client settings"),
            ("overrides/config/common.json", b"common settings"),
            ("overrides/mods/download.jar", b"override beats index"),
            ("client-overrides/mods/client.jar", b"client mod"),
            ("server-overrides/mods/server.jar", b"server mod"),
            ("README.txt", b"not installed"),
        ];
        if !client_first {
            entries.reverse();
        }
        let archive = mrpack(
            temp.path(),
            json!([
                index_file("mods/download.jar", b"download", "required"),
                index_file("mods/optional.jar", b"optional", "optional"),
                index_file("mods/server-only.jar", b"server", "unsupported")
            ]),
            &entries,
        );
        let inventory = inspect_archive(&archive).await.unwrap();
        assert_eq!(inventory.name, "Archive fixture");
        assert_eq!(inventory.version, "2.0");
        assert_eq!(inventory.game_version, "1.21.1");
        assert!(inventory.locked);
        assert!(!inventory.needs_inventory);
        assert_eq!(inventory.files.len(), 4);
        assert_eq!(
            inventory.files["config/common.json"],
            file(b"client settings")
        );
        assert_eq!(
            inventory.files["mods/download.jar"],
            file(b"override beats index")
        );
        assert_eq!(inventory.files["mods/client.jar"], file(b"client mod"));
        let optional = &inventory.files["mods/optional.jar"];
        assert_eq!(optional.sha1, file(b"optional").sha1);
        assert_eq!(optional.size, 8);
        assert_eq!(optional.downloads, ["https://example.invalid/fixture.jar"]);
    }
}

#[tokio::test]
async fn record_install_verifies_bytes_and_preserves_previous_state_on_failure() {
    let temp = fixture();
    let root = temp.path().join("instance");
    put(&root, "config/a.json", b"expected");
    put(&root, "mods/user.jar", b"user addition");
    let archive = mrpack(
        temp.path(),
        json!([]),
        &[("overrides/config/a.json", b"expected")],
    );
    record_install(
        &root,
        &archive,
        "modrinth",
        Some("project".into()),
        Some("release".into()),
    )
    .await
    .unwrap();
    let installed = load(&root).unwrap().unwrap();
    assert_eq!(installed.source, "modrinth");
    assert_eq!(installed.project_id.as_deref(), Some("project"));
    assert_eq!(installed.version_id.as_deref(), Some("release"));
    assert_eq!(
        installed.files,
        BTreeMap::from([("config/a.json".into(), file(b"expected"))])
    );
    let metadata = fs::read(root.join(STATE_FILE)).unwrap();
    for bytes in [b"tampered".as_slice(), b"wrong length".as_slice()] {
        put(&root, "config/a.json", bytes);
        assert!(
            record_install(&root, &archive, "local", None, None)
                .await
                .is_err()
        );
        assert_eq!(fs::read(root.join(STATE_FILE)).unwrap(), metadata);
        assert_eq!(fs::read(root.join("config/a.json")).unwrap(), bytes);
    }
    fs::remove_file(root.join("config/a.json")).unwrap();
    assert!(
        record_install(&root, &archive, "local", None, None)
            .await
            .is_err()
    );
    assert_eq!(load(&root).unwrap(), Some(installed));
}

#[tokio::test]
async fn record_inventory_persists_original_mod_ids_after_the_installed_jar_is_deleted() {
    let temp = fixture();
    let root = temp.path().join("instance");
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("mcmod.info", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(
        br#"{"modList":[{"modid":"first","version":"1"},{"modid":"second","version":"1"}]}"#,
    )
    .unwrap();
    let jar = zip.finish().unwrap().into_inner();
    put(&root, "mods/original.jar", &jar);
    put(&root, "mods/user.jar", &jar);
    put(&root, "config/settings", b"config");
    let archive = mrpack(
        temp.path(),
        json!([]),
        &[
            ("overrides/mods/original.jar", &jar),
            ("overrides/config/settings", b"config"),
        ],
    );
    record_install(&root, &archive, "local", None, None)
        .await
        .unwrap();
    let expected = BTreeMap::from([(
        "mods/original.jar".into(),
        std::collections::BTreeSet::from(["first".into(), "second".into()]),
    )]);
    let installed = load(&root).unwrap().unwrap();
    assert_eq!(installed.identities, expected);
    assert_eq!(installed.files["mods/original.jar"], file(&jar));
    assert!(!installed.files.contains_key("mods/user.jar"));
    fs::remove_file(root.join("mods/original.jar")).unwrap();
    let loaded = read_state(&root).await.unwrap().unwrap();
    assert_eq!(loaded, installed);
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(STATE_FILE)).unwrap()).unwrap();
    assert_eq!(
        json["identities"]["mods/original.jar"],
        json!(["first", "second"])
    );
}

#[test]
fn pre_identity_inventory_still_loads_and_preserves_file_ownership() {
    let temp = fixture();
    let expected = state(&[("mods/old.jar", b"original")]);
    let mut json = serde_json::to_value(&expected).unwrap();
    json.as_object_mut().unwrap().remove("identities");
    put(temp.path(), STATE_FILE, &serde_json::to_vec(&json).unwrap());
    let loaded = load(temp.path()).unwrap().unwrap();
    assert_eq!(loaded, expected);
    assert!(loaded.identities.is_empty());
    assert!(owned(&loaded, "mods/old.jar.disabled").is_some());
}

#[test]
fn verification_accepts_uppercase_hashes_fills_missing_hashes_and_checks_size() {
    let temp = fixture();
    put(temp.path(), "config/a", b"abc");
    let mut inventory = state(&[("config/a", b"abc")]);
    inventory
        .files
        .get_mut("config/a")
        .unwrap()
        .sha1
        .make_ascii_uppercase();
    verify_files(temp.path(), &mut inventory).unwrap();
    assert_eq!(
        inventory.files["config/a"].sha1,
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    inventory.files.get_mut("config/a").unwrap().sha1.clear();
    verify_files(temp.path(), &mut inventory).unwrap();
    assert_eq!(inventory.files["config/a"], file(b"abc"));
    inventory.files.get_mut("config/a").unwrap().size += 1;
    assert!(verify_files(temp.path(), &mut inventory).is_err());
}

#[tokio::test]
async fn protection_distinguishes_pack_and_user_files_including_disabled_aliases() {
    let temp = fixture();
    let mut inventory = state(&[
        ("mods/pack.jar", b"pack"),
        ("mods/off.jar.disabled", b"off"),
        ("resourcepacks/pack.zip", b"resources"),
        ("shaderpacks/pack.zip", b"shaders"),
    ]);
    save(temp.path(), &inventory).unwrap();
    for path in [
        "mods/pack.jar",
        "mods/pack.jar.disabled",
        "mods/off.jar",
        "mods/off.jar.disabled",
        "resourcepacks/pack.zip",
        "shaderpacks/pack.zip",
    ] {
        assert!(
            ensure_paths_mutable(temp.path(), &[path.into()])
                .await
                .is_err(),
            "{path}"
        );
    }
    for path in [
        "mods/user.jar",
        "mods/user.jar.disabled",
        "mods/pack.jar.backup",
        "config/user.json",
    ] {
        ensure_paths_mutable(temp.path(), &[path.into()])
            .await
            .unwrap();
    }
    assert!(
        ensure_paths_mutable(
            temp.path(),
            &["mods/user.jar".into(), "mods/pack.jar".into()]
        )
        .await
        .is_err()
    );
    inventory.locked = false;
    save(temp.path(), &inventory).unwrap();
    ensure_paths_mutable(temp.path(), &["mods/pack.jar.disabled".into()])
        .await
        .unwrap();
    assert!(
        ensure_paths_mutable(temp.path(), &["instance.cub".into()])
            .await
            .is_err()
    );
}

#[tokio::test]
async fn corrupt_or_unsupported_metadata_fails_closed_even_with_valid_legacy_metadata() {
    let temp = fixture();
    put(
        temp.path(),
        "upstream.json",
        br#"{"type":"modrinth-modpack","projectId":"p","versionId":"v"}"#,
    );
    let mut unsupported = state(&[]);
    unsupported.schema_version = 999;
    let mut reserved = state(&[]);
    reserved
        .files
        .insert("config/../instance.cub".into(), file(b"bad"));
    for bytes in [
        b"{truncated".to_vec(),
        b"null".to_vec(),
        serde_json::to_vec(&unsupported).unwrap(),
        serde_json::to_vec(&reserved).unwrap(),
    ] {
        put(temp.path(), STATE_FILE, &bytes);
        assert!(load(temp.path()).is_err());
        assert!(
            ensure_paths_mutable(temp.path(), &["mods/user.jar".into()])
                .await
                .is_err()
        );
        assert!(annotate_mods(temp.path(), "mods", &mut []).await.is_err());
        assert_eq!(fs::read(temp.path().join(STATE_FILE)).unwrap(), bytes);
    }
}

#[tokio::test]
async fn legacy_inventory_is_conservative_and_missing_metadata_is_not_a_pack() {
    let temp = fixture();
    assert_eq!(load(temp.path()).unwrap(), None);
    ensure_paths_mutable(temp.path(), &["mods/user.jar".into()])
        .await
        .unwrap();
    for (upstream, source, project, version) in [
        (
            json!({"type":"modrinth-modpack", "projectId":"project", "versionId":"version"}),
            "modrinth",
            "project",
            "version",
        ),
        (
            json!({"type":"curseforge-modpack", "projectId":123, "fileId":456}),
            "curseforge",
            "123",
            "456",
        ),
    ] {
        put(
            temp.path(),
            "upstream.json",
            &serde_json::to_vec(&upstream).unwrap(),
        );
        let inventory = load(temp.path()).unwrap().unwrap();
        assert_eq!(inventory.source, source);
        assert_eq!(inventory.project_id.as_deref(), Some(project));
        assert_eq!(inventory.version_id.as_deref(), Some(version));
        assert!(inventory.needs_inventory && inventory.locked);
        assert!(inventory.files.is_empty());
        assert!(
            ensure_paths_mutable(temp.path(), &["mods/unknown.jar".into()])
                .await
                .is_err()
        );
    }
    put(temp.path(), "upstream.json", b"{broken");
    assert!(
        ensure_paths_mutable(temp.path(), &["mods/unknown.jar".into()])
            .await
            .is_err()
    );
}

#[tokio::test]
async fn paths_and_archives_reject_traversal_and_reserved_metadata_at_any_depth() {
    let temp = fixture();
    for path in [
        "",
        "../outside",
        "/absolute",
        "C:/absolute",
        "mods\\evil.jar",
        "mods//a.jar",
        "mods/./a.jar",
        "mods/a.jar/",
        "mods/\0.jar",
        "mods/\n.jar",
        "instance.cub",
        "config/INSTANCE.CUB",
        "nested/upstream.json",
        "modpack.cub.json",
        "mods/cache.CREP",
        ".modpack-transaction/backup",
        "config/.MODPACK-stage/a",
        ".git/config",
        "nested/cubic-jar/a.jar",
        "cubic-jar-cache/a.jar",
    ] {
        assert!(valid_path(path).is_err(), "{path:?}");
        assert!(checked_path(temp.path(), path).is_err(), "{path:?}");
    }
    for path in [
        "mods/a.jar",
        "config/nested/options.json",
        "resourcepacks/Árbol.zip",
    ] {
        assert_eq!(
            checked_path(temp.path(), path).unwrap(),
            temp.path().join(path)
        );
    }
    for path in [
        "../escape",
        "instance.cub",
        "config/.git/config",
        "mods/cache.crep",
        "nested/modpack.cub.json",
    ] {
        let archive = mrpack(
            temp.path(),
            json!([index_file(path, b"bad", "required")]),
            &[],
        );
        assert!(inspect_archive(&archive).await.is_err(), "index: {path}");
        for prefix in ["overrides", "client-overrides"] {
            let entry = format!("{prefix}/{path}");
            let archive = mrpack(temp.path(), json!([]), &[(entry.as_str(), b"bad")]);
            assert!(inspect_archive(&archive).await.is_err(), "{entry}");
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn linked_files_and_ancestors_are_rejected_even_when_destination_is_missing() {
    use std::os::unix::fs::symlink;
    let temp = fixture();
    let root = temp.path().join("instance");
    let outside = temp.path().join("outside");
    put(&outside, "secret", b"untouched");
    fs::create_dir(&root).unwrap();
    symlink(&outside, root.join("config")).unwrap();
    symlink(outside.join("secret"), root.join("linked.jar")).unwrap();
    symlink(outside.join("missing"), root.join("dangling.jar")).unwrap();
    for relative in [
        "config/secret",
        "config/new/deep.json",
        "linked.jar",
        "dangling.jar",
    ] {
        assert!(checked_path(&root, relative).is_err(), "{relative}");
        assert!(
            ensure_paths_mutable(&root, &[relative.into()])
                .await
                .is_err()
        );
        assert!(verify_files(&root, &mut state(&[(relative, b"untouched")])).is_err());
    }
    assert_eq!(fs::read(outside.join("secret")).unwrap(), b"untouched");
    assert!(!outside.join("new").exists());
}
