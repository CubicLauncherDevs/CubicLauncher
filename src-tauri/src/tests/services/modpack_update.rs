use super::*;
use crate::services::instance_manager::data::InstanceData;
use crate::services::modpack::PackFile;
use sha1::{Digest, Sha1};
use std::io::Write;
use zip::write::SimpleFileOptions;

fn fixture() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(".modpack-update-test-")
        .tempdir_in(env!("CARGO_MANIFEST_DIR"))
        .unwrap()
}

fn put(root: &Path, path: &str, bytes: &[u8]) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn state(version: &str, entries: &[(&str, &[u8])]) -> PackState {
    PackState {
        schema_version: 1,
        source: "local".into(),
        project_id: None,
        version_id: None,
        name: "Update fixture".into(),
        author: None,
        version: version.into(),
        game_version: "1.21.1".into(),
        locked: true,
        files: entries
            .iter()
            .map(|(p, b)| {
                (
                    (*p).into(),
                    PackFile {
                        sha1: Sha1::digest(b)
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect(),
                        size: b.len() as u64,
                        downloads: vec![],
                    },
                )
            })
            .collect(),
        needs_inventory: false,
        identities: BTreeMap::new(),
    }
}

fn prepare_fixture(
    temp: &tempfile::TempDir,
    old_entries: &[(&str, &[u8])],
    next_entries: &[(&str, &[u8])],
    local: &[(&str, &[u8])],
) -> Prepared {
    let root = temp.path().join("instance");
    fs::create_dir_all(&root).unwrap();
    for (p, bytes) in local {
        put(&root, p, bytes);
    }
    let old = state("1", old_entries);
    modpack::save(&root, &old).unwrap();
    // Construct InstanceData without PathManager/global settings or writes outside the fixture.
    put(
        &root,
        "instance.cub",
        &serde_json::to_vec(&serde_json::json!({
            "name": "instance", "version": "1.21.1", "last_played": 0,
            "uuid": uuid::Uuid::new_v4().to_string()
        }))
        .unwrap(),
    );
    let stage = tempfile::Builder::new()
        .prefix("stage-")
        .tempdir_in(temp.path())
        .unwrap();
    fs::create_dir(stage.path().join("files")).unwrap();
    for (p, bytes) in next_entries {
        put(&stage.path().join("files"), p, bytes);
    }
    build_preview(
        "fixture".into(),
        root,
        stage,
        old,
        state("2", next_entries),
        "1.21.1".into(),
    )
    .unwrap()
}

fn read_journal(root: &Path) -> Journal {
    serde_json::from_slice(&fs::read(root.join(JOURNAL_DIR).join("journal.json")).unwrap()).unwrap()
}

fn resolutions(paths: &[&str], choice: &str) -> BTreeMap<String, String> {
    paths.iter().map(|p| ((*p).into(), choice.into())).collect()
}

fn rebuild_preview(prepared: Prepared) -> Result<Prepared> {
    build_preview(
        prepared.id,
        prepared.root,
        prepared.stage,
        prepared.old,
        prepared.next,
        prepared.runtime,
    )
}

fn instance_handle(root: &Path) -> InstanceHandle {
    let mut data: InstanceData =
        serde_json::from_slice(&fs::read(root.join("instance.cub")).unwrap()).unwrap();
    data.instance_root = root.parent().unwrap().to_path_buf();
    InstanceHandle::new(data)
}

// Capture every file, including metadata, to detect unintended writes on failure.
fn contents(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), files);
            } else {
                files.insert(
                    entry.path().strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn preview_diff_only_removes_pack_owned_files_and_apply_preserves_user_additions() {
    let temp = fixture();
    let old: &[(&str, &[u8])] = &[
        ("mods/old.jar", b"old"),
        ("config/change.json", b"before"),
        ("config/same.json", b"same"),
    ];
    let next: &[(&str, &[u8])] = &[
        ("mods/new.jar", b"new"),
        ("config/change.json", b"after"),
        ("config/same.json", b"same"),
    ];
    let mut local = old.to_vec();
    local.extend_from_slice(&[
        ("mods/user.jar", b"user"),
        ("resourcepacks/user.zip", b"resources"),
        ("shaderpacks/user.zip", b"shaders"),
        ("config/own.json", b"custom"),
    ]);
    let prepared = prepare_fixture(&temp, old, next, &local);
    assert_eq!(prepared.preview.added, ["mods/new.jar"]);
    assert_eq!(prepared.preview.removed, ["mods/old.jar"]);
    assert_eq!(prepared.preview.changed, ["config/change.json"]);
    assert!(prepared.preview.conflicts.is_empty());
    apply_files(&prepared, &BTreeMap::new()).unwrap();
    assert!(!prepared.root.join("mods/old.jar").exists());
    for (p, bytes) in next.iter().chain(local[3..].iter()) {
        assert_eq!(fs::read(prepared.root.join(p)).unwrap(), *bytes, "{p}");
    }
    assert_eq!(
        modpack::load(&prepared.root).unwrap(),
        Some(prepared.next.clone())
    );
    let journal = read_journal(&prepared.root);
    assert!(!journal.paths.contains_key("mods/user.jar"));
    assert!(!journal.paths.contains_key("config/same.json"));
    recover(&prepared.root).unwrap();
    assert_eq!(
        fs::read(prepared.root.join("mods/old.jar")).unwrap(),
        b"old"
    );
    assert!(!prepared.root.join("mods/new.jar").exists());
}

#[test]
fn preview_marks_modified_configs_added_collisions_and_local_deletions_as_conflicts() {
    let temp = fixture();
    let prepared = prepare_fixture(
        &temp,
        &[
            ("config/changed", b"old"),
            ("config/removed", b"old"),
            ("config/deleted", b"old"),
            ("config/unchanged", b"old"),
        ],
        &[
            ("config/changed", b"new"),
            ("config/added", b"pack"),
            ("config/deleted", b"new"),
            ("config/unchanged", b"old"),
        ],
        &[
            ("config/changed", b"edited"),
            ("config/removed", b"edited"),
            ("config/added", b"user"),
            ("config/unchanged", b"edited"),
        ],
    );
    assert_eq!(prepared.preview.added, ["config/added"]);
    assert_eq!(prepared.preview.removed, ["config/removed"]);
    assert_eq!(
        prepared.preview.changed,
        ["config/changed", "config/deleted"]
    );
    assert_eq!(
        prepared.preview.conflicts,
        [
            "config/added",
            "config/changed",
            "config/deleted",
            "config/removed"
        ]
    );
    // Unchanged upstream configuration must not overwrite a local edit.
    apply_files(
        &prepared,
        &resolutions(
            &[
                "config/added",
                "config/changed",
                "config/deleted",
                "config/removed",
            ],
            "keep",
        ),
    )
    .unwrap();
    for (p, bytes) in [
        ("config/added", b"user".as_slice()),
        ("config/changed", b"edited"),
        ("config/removed", b"edited"),
        ("config/unchanged", b"edited"),
    ] {
        assert_eq!(fs::read(prepared.root.join(p)).unwrap(), bytes);
    }
    assert!(!prepared.root.join("config/deleted").exists());
    assert_eq!(
        read_journal(&prepared.root).paths.len(),
        2,
        "only metadata needs a backup when all changes are kept"
    );
    let mut expected = prepared.next.clone();
    // A removed file explicitly kept by the user retains its original provenance.
    expected.files.insert(
        "config/removed".into(),
        prepared.old.files["config/removed"].clone(),
    );
    assert_eq!(modpack::load(&prepared.root).unwrap(), Some(expected));
}

#[test]
fn replacing_conflicts_applies_added_changed_and_deleted_pack_paths() {
    let temp = fixture();
    let prepared = prepare_fixture(
        &temp,
        &[("config/change", b"old"), ("config/remove", b"old")],
        &[("config/change", b"new"), ("config/add", b"new")],
        &[
            ("config/change", b"custom"),
            ("config/remove", b"custom"),
            ("config/add", b"custom"),
        ],
    );
    apply_files(
        &prepared,
        &resolutions(&["config/change", "config/remove", "config/add"], "replace"),
    )
    .unwrap();
    assert_eq!(
        fs::read(prepared.root.join("config/change")).unwrap(),
        b"new"
    );
    assert_eq!(fs::read(prepared.root.join("config/add")).unwrap(), b"new");
    assert!(!prepared.root.join("config/remove").exists());
    recover(&prepared.root).unwrap();
    for p in ["config/change", "config/remove", "config/add"] {
        assert_eq!(fs::read(prepared.root.join(p)).unwrap(), b"custom");
    }
}

#[test]
fn interrupted_apply_rolls_back_actual_partial_writes_deletions_and_metadata() {
    let temp = fixture();
    let old: &[(&str, &[u8])] = &[
        ("config/a-change", b"old"),
        ("config/b-remove", b"removed"),
        ("config/z-fail", b"old last"),
    ];
    let prepared = prepare_fixture(
        &temp,
        old,
        &[
            ("config/a-change", b"new"),
            ("config/c-add", b"added"),
            ("config/z-fail", b"new last"),
        ],
        old,
    );
    let before = fs::read(prepared.root.join(modpack::STATE_FILE)).unwrap();
    let instance_before = fs::read(prepared.root.join("instance.cub")).unwrap();
    // Missing final source fails after earlier operations have reached the real filesystem.
    fs::remove_file(prepared.stage.path().join("files/config/z-fail")).unwrap();
    assert!(apply_files(&prepared, &BTreeMap::new()).is_err());
    assert_eq!(
        fs::read(prepared.root.join("config/a-change")).unwrap(),
        b"new"
    );
    assert!(!prepared.root.join("config/b-remove").exists());
    assert_eq!(
        fs::read(prepared.root.join("config/c-add")).unwrap(),
        b"added"
    );
    assert!(!read_journal(&prepared.root).committed);
    recover(&prepared.root).unwrap();
    for (p, bytes) in old {
        assert_eq!(fs::read(prepared.root.join(p)).unwrap(), *bytes);
    }
    assert!(!prepared.root.join("config/c-add").exists());
    assert_eq!(
        fs::read(prepared.root.join(modpack::STATE_FILE)).unwrap(),
        before
    );
    assert_eq!(
        fs::read(prepared.root.join("instance.cub")).unwrap(),
        instance_before
    );
    assert!(!prepared.root.join(JOURNAL_DIR).exists());
    recover(&prepared.root).unwrap();
}

#[test]
fn crash_after_metadata_save_before_commit_restores_exact_original_metadata_and_files() {
    let temp = fixture();
    let prepared = prepare_fixture(
        &temp,
        &[("config/a", b"old")],
        &[("config/a", b"new"), ("mods/new.jar", b"new mod")],
        &[("config/a", b"custom"), ("mods/user.jar", b"user")],
    );
    // Preserve noncanonical JSON bytes as well as semantic values during rollback.
    put(
        &prepared.root,
        modpack::STATE_FILE,
        &serde_json::to_vec_pretty(&prepared.old).unwrap(),
    );
    let before = fs::read(prepared.root.join(modpack::STATE_FILE)).unwrap();
    let instance_before = fs::read(prepared.root.join("instance.cub")).unwrap();
    apply_files(&prepared, &resolutions(&["config/a"], "replace")).unwrap();
    assert_eq!(
        modpack::load(&prepared.root).unwrap(),
        Some(prepared.next.clone())
    );
    // Model the crash window after instance.cub is saved, before the durable commit bit.
    let mut instance: serde_json::Value = serde_json::from_slice(&instance_before).unwrap();
    instance["version"] = "1.21.2".into();
    put(
        &prepared.root,
        "instance.cub",
        &serde_json::to_vec(&instance).unwrap(),
    );
    let journal = read_journal(&prepared.root);
    assert!(!journal.committed);
    assert_eq!(journal.paths[modpack::STATE_FILE], true);
    assert_eq!(journal.paths["instance.cub"], true);
    assert_eq!(journal.paths["mods/new.jar"], false);
    recover(&prepared.root).unwrap();
    assert_eq!(fs::read(prepared.root.join("config/a")).unwrap(), b"custom");
    assert_eq!(
        fs::read(prepared.root.join("mods/user.jar")).unwrap(),
        b"user"
    );
    assert!(!prepared.root.join("mods/new.jar").exists());
    assert_eq!(
        fs::read(prepared.root.join(modpack::STATE_FILE)).unwrap(),
        before
    );
    assert_eq!(
        fs::read(prepared.root.join("instance.cub")).unwrap(),
        instance_before
    );
    assert!(!prepared.root.join(JOURNAL_DIR).exists());
}

#[test]
fn recovery_of_committed_transaction_only_cleans_journal_and_is_idempotent() {
    let temp = fixture();
    let prepared = prepare_fixture(
        &temp,
        &[("config/a", b"old")],
        &[("config/a", b"new")],
        &[("config/a", b"old")],
    );
    apply_files(&prepared, &BTreeMap::new()).unwrap();
    let mut journal = read_journal(&prepared.root);
    journal.committed = true;
    persist_json(
        &prepared.root.join(JOURNAL_DIR).join("journal.json"),
        &journal,
    )
    .unwrap();
    for _ in 0..2 {
        recover(&prepared.root).unwrap();
        assert_eq!(fs::read(prepared.root.join("config/a")).unwrap(), b"new");
        assert_eq!(
            modpack::load(&prepared.root).unwrap(),
            Some(prepared.next.clone())
        );
        assert!(!prepared.root.join(JOURNAL_DIR).exists());
    }
}

#[test]
fn recovery_removes_new_metadata_if_it_did_not_exist_before_the_transaction() {
    let temp = fixture();
    let prepared = prepare_fixture(&temp, &[], &[("config/new", b"new")], &[]);
    fs::remove_file(prepared.root.join(modpack::STATE_FILE)).unwrap();
    fs::remove_file(prepared.root.join("instance.cub")).unwrap();
    apply_files(&prepared, &BTreeMap::new()).unwrap();
    put(&prepared.root, "instance.cub", b"new metadata");
    let journal = read_journal(&prepared.root);
    assert!(!journal.paths[modpack::STATE_FILE]);
    assert!(!journal.paths["instance.cub"]);
    recover(&prepared.root).unwrap();
    for p in [
        modpack::STATE_FILE,
        "instance.cub",
        "config/new",
        JOURNAL_DIR,
    ] {
        assert!(!prepared.root.join(p).exists(), "{p}");
    }
}

#[test]
fn recovery_abandons_incomplete_backup_but_retains_corrupt_journal_for_retry() {
    let temp = fixture();
    put(temp.path(), "config/a", b"current");
    put(
        temp.path(),
        &format!("{JOURNAL_DIR}/backup/config/a"),
        b"backup",
    );
    recover(temp.path()).unwrap();
    assert_eq!(fs::read(temp.path().join("config/a")).unwrap(), b"current");
    assert!(!temp.path().join(JOURNAL_DIR).exists());
    put(
        temp.path(),
        &format!("{JOURNAL_DIR}/journal.json"),
        b"{incomplete",
    );
    assert!(recover(temp.path()).is_err());
    assert!(temp.path().join(JOURNAL_DIR).exists());
    assert_eq!(fs::read(temp.path().join("config/a")).unwrap(), b"current");
}

fn jar(metadata_path: &str, metadata: &str) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file(
        metadata_path,
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )
    .unwrap();
    zip.write_all(metadata.as_bytes()).unwrap();
    zip.finish().unwrap().into_inner()
}

#[test]
fn jar_identity_collisions_use_mod_ids_across_versions_and_not_display_names() {
    for (metadata_path, old_json, next_json, other_json) in [
        (
            "fabric.mod.json",
            r#"{"id":"shared","name":"Old name","version":"1"}"#,
            r#"{"id":"shared","name":"New name","version":"2"}"#,
            r#"{"id":"other","name":"New name","version":"2"}"#,
        ),
        (
            "quilt.mod.json",
            r#"{"quilt_loader":{"id":"shared","version":"1"}}"#,
            r#"{"quilt_loader":{"id":"shared","version":"2"}}"#,
            r#"{"quilt_loader":{"id":"other","version":"2"}}"#,
        ),
        (
            "META-INF/mods.toml",
            "[[mods]]\nmodId='unrelated'\nversion='1'\n[[mods]]\nmodId='shared'\nversion='1'",
            "[[mods]]\nmodId='shared'\nversion='2'",
            "[[mods]]\nmodId='other'\nversion='2'",
        ),
        (
            "META-INF/neoforge.mods.toml",
            "[[mods]]\nmodId='shared'\nversion='1'",
            "[[mods]]\nmodId='shared'\nversion='2'",
            "[[mods]]\nmodId='other'\nversion='2'",
        ),
    ] {
        let old_jar = jar(metadata_path, old_json);
        let next_jar = jar(metadata_path, next_json);
        let other_jar = jar(metadata_path, other_json);
        assert_ne!(old_jar, next_jar);
        for choice in ["keep", "replace"] {
            let temp = fixture();
            let prepared = prepare_fixture(
                &temp,
                &[],
                &[("mods/pack-v2.jar", &next_jar)],
                &[
                    ("mods/user-v1.jar", &old_jar),
                    ("mods/other.jar", &other_jar),
                ],
            );
            assert_eq!(
                prepared.preview.conflicts,
                ["mods/pack-v2.jar"],
                "{metadata_path}"
            );
            assert_eq!(
                prepared.collisions["mods/pack-v2.jar"],
                ["mods/user-v1.jar"]
            );
            apply_files(&prepared, &resolutions(&["mods/pack-v2.jar"], choice)).unwrap();
            assert_eq!(
                fs::read(prepared.root.join("mods/other.jar")).unwrap(),
                other_jar
            );
            if choice == "keep" {
                assert!(!prepared.root.join("mods/pack-v2.jar").exists());
                assert_eq!(
                    fs::read(prepared.root.join("mods/user-v1.jar")).unwrap(),
                    old_jar
                );
            } else {
                assert!(!prepared.root.join("mods/user-v1.jar").exists());
                assert_eq!(
                    fs::read(prepared.root.join("mods/pack-v2.jar")).unwrap(),
                    next_jar
                );
            }
            recover(&prepared.root).unwrap();
            assert_eq!(
                fs::read(prepared.root.join("mods/user-v1.jar")).unwrap(),
                old_jar
            );
            assert!(!prepared.root.join("mods/pack-v2.jar").exists());
        }
    }
}

#[test]
fn disabled_pack_alias_is_a_conflict_and_keep_does_not_reenable_it() {
    for choice in ["keep", "replace"] {
        let temp = fixture();
        let prepared = prepare_fixture(
            &temp,
            &[("mods/pack.jar", b"old")],
            &[("mods/pack.jar", b"new")],
            &[("mods/pack.jar.disabled", b"old")],
        );
        assert_eq!(prepared.preview.conflicts, ["mods/pack.jar"]);
        assert_eq!(
            prepared.collisions["mods/pack.jar"],
            ["mods/pack.jar.disabled"]
        );
        apply_files(&prepared, &resolutions(&["mods/pack.jar"], choice)).unwrap();
        assert_eq!(
            prepared.root.join("mods/pack.jar").exists(),
            choice == "replace"
        );
        assert_eq!(
            prepared.root.join("mods/pack.jar.disabled").exists(),
            choice == "keep"
        );
        recover(&prepared.root).unwrap();
        assert!(!prepared.root.join("mods/pack.jar").exists());
        assert_eq!(
            fs::read(prepared.root.join("mods/pack.jar.disabled")).unwrap(),
            b"old"
        );
    }
}

#[test]
fn changed_jar_collides_with_disabled_user_version_but_renamed_pack_jar_is_not_a_user_mod() {
    let previous = jar("fabric.mod.json", r#"{"id":"shared","version":"1"}"#);
    let next = jar("fabric.mod.json", r#"{"id":"shared","version":"2"}"#);
    let user = jar("fabric.mod.json", r#"{"id":"shared","version":"3"}"#);
    let unrelated = jar("fabric.mod.json", r#"{"id":"unrelated","version":"1"}"#);
    let temp = fixture();
    let prepared = prepare_fixture(
        &temp,
        &[("mods/pack.jar", &previous)],
        &[("mods/pack.jar", &next)],
        &[
            ("mods/pack.jar", &previous),
            ("mods/user-v3.jar.disabled", &user),
            ("mods/unrelated.jar", &unrelated),
        ],
    );
    assert!(prepared.preview.added.is_empty());
    assert_eq!(prepared.preview.changed, ["mods/pack.jar"]);
    assert_eq!(prepared.preview.conflicts, ["mods/pack.jar"]);
    assert_eq!(
        prepared.collisions["mods/pack.jar"],
        ["mods/user-v3.jar.disabled"]
    );
    apply_files(&prepared, &resolutions(&["mods/pack.jar"], "replace")).unwrap();
    assert_eq!(fs::read(prepared.root.join("mods/pack.jar")).unwrap(), next);
    assert!(!prepared.root.join("mods/user-v3.jar.disabled").exists());
    assert_eq!(
        fs::read(prepared.root.join("mods/unrelated.jar")).unwrap(),
        unrelated
    );
    recover(&prepared.root).unwrap();
    assert_eq!(
        fs::read(prepared.root.join("mods/pack.jar")).unwrap(),
        previous
    );
    assert_eq!(
        fs::read(prepared.root.join("mods/user-v3.jar.disabled")).unwrap(),
        user
    );

    let temp = fixture();
    let prepared = prepare_fixture(
        &temp,
        &[("mods/pack-v1.jar", &previous)],
        &[("mods/pack-v2.jar", &next)],
        &[("mods/pack-v1.jar", &previous)],
    );
    assert_eq!(prepared.preview.added, ["mods/pack-v2.jar"]);
    assert_eq!(prepared.preview.removed, ["mods/pack-v1.jar"]);
    assert!(prepared.preview.conflicts.is_empty());
    assert!(prepared.collisions.is_empty());
}

#[tokio::test]
async fn keeping_edited_disabled_or_deleted_renamed_mod_skips_replacement_and_retains_provenance() {
    let original = jar("fabric.mod.json", r#"{"id":"shared","version":"1"}"#);
    let replacement = jar("fabric.mod.json", r#"{"id":"shared","version":"2"}"#);
    let edited = jar(
        "fabric.mod.json",
        r#"{"id":"shared","version":"user-edited"}"#,
    );
    for (change, persisted_ids) in [
        ("edited", false),
        ("disabled", false),
        ("edited", true),
        ("disabled", true),
        ("deleted", true),
    ] {
        let temp = fixture();
        let mut prepared = prepare_fixture(
            &temp,
            &[("mods/old.jar", &original)],
            &[("mods/new.jar", &replacement)],
            &[
                ("mods/old.jar", &original),
                ("config/user", b"user settings"),
            ],
        );
        if persisted_ids {
            // Record identities while the original JAR still exists, then load from disk.
            modpack::verify_files(&prepared.root, &mut prepared.old).unwrap();
            modpack::save(&prepared.root, &prepared.old).unwrap();
            prepared.old = modpack::load(&prepared.root).unwrap().unwrap();
            assert_eq!(
                prepared.old.identities["mods/old.jar"],
                BTreeSet::from(["shared".into()])
            );
        }
        modpack::verify_files(&prepared.stage.path().join("files"), &mut prepared.next).unwrap();
        match change {
            "edited" => put(&prepared.root, "mods/old.jar", &edited),
            "disabled" => fs::rename(
                prepared.root.join("mods/old.jar"),
                prepared.root.join("mods/old.jar.disabled"),
            )
            .unwrap(),
            "deleted" => fs::remove_file(prepared.root.join("mods/old.jar")).unwrap(),
            _ => unreachable!(),
        }
        let before = contents(&prepared.root);
        let prepared = rebuild_preview(prepared).unwrap();
        assert_eq!(prepared.preview.added, ["mods/new.jar"]);
        assert_eq!(prepared.preview.removed, ["mods/old.jar"]);
        assert_eq!(
            prepared.preview.conflicts,
            ["mods/old.jar"],
            "{change}, persisted={persisted_ids}"
        );
        assert_eq!(prepared.replacements["mods/old.jar"], ["mods/new.jar"]);
        apply_files(&prepared, &resolutions(&["mods/old.jar"], "keep")).unwrap();
        assert!(!prepared.root.join("mods/new.jar").exists(), "{change}");
        match change {
            "edited" => assert_eq!(
                fs::read(prepared.root.join("mods/old.jar")).unwrap(),
                edited
            ),
            "disabled" => {
                assert!(!prepared.root.join("mods/old.jar").exists());
                assert_eq!(
                    fs::read(prepared.root.join("mods/old.jar.disabled")).unwrap(),
                    original
                );
            }
            "deleted" => assert!(!prepared.root.join("mods/old.jar").exists()),
            _ => unreachable!(),
        }
        let saved = modpack::load(&prepared.root).unwrap().unwrap();
        let mut expected = prepared.next.clone();
        expected.files = prepared.old.files.clone();
        expected.identities = prepared.old.identities.clone();
        assert_eq!(saved, expected, "{change}, persisted={persisted_ids}");
        assert_eq!(read_journal(&prepared.root).paths.len(), 2);
        assert!(
            modpack::ensure_paths_mutable(&prepared.root, &["mods/old.jar".into()])
                .await
                .is_err()
        );
        assert!(
            modpack::ensure_paths_mutable(&prepared.root, &["mods/old.jar.disabled".into()])
                .await
                .is_err()
        );
        assert_eq!(
            fs::read(prepared.root.join("config/user")).unwrap(),
            b"user settings"
        );
        recover(&prepared.root).unwrap();
        assert_eq!(contents(&prepared.root), before);
    }
}

#[test]
fn keeping_removed_mod_and_explicitly_replacing_its_successor_is_rejected_before_journaling() {
    let original = jar("fabric.mod.json", r#"{"id":"shared","version":"1"}"#);
    let replacement = jar("fabric.mod.json", r#"{"id":"shared","version":"2"}"#);
    let edited = jar("fabric.mod.json", r#"{"id":"shared","version":"edited"}"#);
    let temp = fixture();
    let prepared = prepare_fixture(
        &temp,
        &[("mods/old.jar", &original)],
        &[("mods/new.jar", &replacement)],
        &[("mods/old.jar", &edited), ("mods/new.jar", b"user file")],
    );
    assert_eq!(prepared.preview.conflicts, ["mods/new.jar", "mods/old.jar"]);
    let before = contents(&prepared.root);
    let error = apply_files(
        &prepared,
        &BTreeMap::from([
            ("mods/old.jar".into(), "keep".into()),
            ("mods/new.jar".into(), "replace".into()),
        ]),
    )
    .unwrap_err();
    assert!(error.contains("incompatibles"), "{error}");
    assert_eq!(contents(&prepared.root), before);
    assert!(!prepared.root.join(JOURNAL_DIR).exists());
}

fn crossed_collisions(temp: &tempfile::TempDir) -> Prepared {
    let a = jar("fabric.mod.json", r#"{"id":"alpha","version":"1"}"#);
    let b = jar("fabric.mod.json", r#"{"id":"beta","version":"1"}"#);
    let next_a = jar("fabric.mod.json", r#"{"id":"alpha","version":"2"}"#);
    let next_b = jar("fabric.mod.json", r#"{"id":"beta","version":"2"}"#);
    let prepared = prepare_fixture(
        temp,
        &[],
        &[("mods/A.jar", &next_a), ("mods/B.jar", &next_b)],
        &[("mods/A.jar", &b), ("mods/B.jar", &a)],
    );
    assert_eq!(prepared.preview.conflicts, ["mods/A.jar", "mods/B.jar"]);
    assert_eq!(prepared.collisions["mods/A.jar"], ["mods/B.jar"]);
    assert_eq!(prepared.collisions["mods/B.jar"], ["mods/A.jar"]);
    prepared
}

#[test]
fn crossed_collisions_cannot_delete_a_kept_path_and_fail_without_any_writes() {
    for (keep, replace) in [("mods/A.jar", "mods/B.jar"), ("mods/B.jar", "mods/A.jar")] {
        let temp = fixture();
        let prepared = crossed_collisions(&temp);
        let before = contents(&prepared.root);
        let error = apply_files(
            &prepared,
            &BTreeMap::from([
                (keep.into(), "keep".into()),
                (replace.into(), "replace".into()),
            ]),
        )
        .unwrap_err();
        assert!(error.contains("incompatibles"), "{error}");
        assert_eq!(contents(&prepared.root), before);
        assert!(!prepared.root.join(JOURNAL_DIR).exists());
    }
}

#[test]
fn replacing_both_crossed_collisions_keeps_both_installed_jars_and_can_roll_back() {
    let temp = fixture();
    let prepared = crossed_collisions(&temp);
    let before = contents(&prepared.root);
    apply_files(
        &prepared,
        &resolutions(&["mods/A.jar", "mods/B.jar"], "replace"),
    )
    .unwrap();
    for path in ["mods/A.jar", "mods/B.jar"] {
        assert_eq!(
            fs::read(prepared.root.join(path)).unwrap(),
            fs::read(prepared.stage.path().join("files").join(path)).unwrap()
        );
    }
    assert_eq!(
        mod_ids(&prepared.root.join("mods/A.jar")),
        BTreeSet::from(["alpha".into()])
    );
    assert_eq!(
        mod_ids(&prepared.root.join("mods/B.jar")),
        BTreeSet::from(["beta".into()])
    );
    assert_eq!(
        modpack::load(&prepared.root).unwrap(),
        Some(prepared.next.clone())
    );
    recover(&prepared.root).unwrap();
    assert_eq!(contents(&prepared.root), before);
}

#[test]
fn preview_rejects_case_only_aliases_from_pack_or_user_paths_before_writes() {
    for case in [
        "next pair",
        "old versus next",
        "user versus next",
        "disabled alias",
        "config directory",
        "unicode",
    ] {
        let temp = fixture();
        let mut prepared = prepare_fixture(&temp, &[], &[], &[("config/untouched", b"sentinel")]);
        match case {
            "next pair" => {
                prepared.next = state("2", &[("mods/A.jar", b"a"), ("mods/a.jar", b"b")])
            }
            "old versus next" => {
                prepared.old = state("1", &[("mods/A.jar", b"a")]);
                prepared.next = state("2", &[("mods/a.jar", b"b")]);
                put(&prepared.root, "mods/A.jar", b"a");
            }
            "user versus next" => {
                prepared.next = state("2", &[("mods/A.jar", b"a")]);
                put(&prepared.root, "mods/a.jar", b"user");
            }
            "disabled alias" => {
                prepared.next = state("2", &[("mods/A.jar", b"a")]);
                put(&prepared.root, "mods/a.jar.disabled", b"disabled user");
            }
            "config directory" => {
                prepared.next = state("2", &[("config/a.json", b"a"), ("Config/a.json", b"b")])
            }
            "unicode" => {
                prepared.next = state("2", &[("mods/Árbol.jar", b"a"), ("mods/árbol.jar", b"b")])
            }
            _ => unreachable!(),
        }
        modpack::save(&prepared.root, &prepared.old).unwrap();
        let root = prepared.root.clone();
        let before = contents(&root);
        // Staged sources intentionally absent: rejection must happen before install I/O.
        let error = rebuild_preview(prepared)
            .err()
            .expect("case alias was accepted");
        assert!(error.contains("mayúsculas"), "{case}: {error}");
        assert_eq!(contents(&root), before, "{case}");
        assert!(!root.join(JOURNAL_DIR).exists());
    }
}

#[test]
fn legacy_mcmod_info_variants_detect_collisions_across_versions_and_loaders() {
    for metadata in [
        r#"{"modid":"shared","name":"Old name","version":"1"}"#,
        r#"[{"modid":"secondary"},{"modid":"shared","version":"1"},{"modid":"shared"},{"name":"not an id"}]"#,
        r#"{"modList":[{"modid":"secondary"},{"modid":"shared","version":"1"}],"modid":"not_a_mod"}"#,
    ] {
        let old = jar("mcmod.info", metadata);
        let next = jar(
            "fabric.mod.json",
            r#"{"id":"shared","name":"New name","version":"2"}"#,
        );
        let other = jar(
            "mcmod.info",
            r#"{"modid":"other","name":"New name","version":"2"}"#,
        );
        let temp = fixture();
        let prepared = prepare_fixture(
            &temp,
            &[],
            &[("mods/new.jar", &next)],
            &[("mods/legacy.jar", &old), ("mods/other.jar", &other)],
        );
        let expected_ids = if metadata.contains("secondary") {
            BTreeSet::from(["secondary".into(), "shared".into()])
        } else {
            BTreeSet::from(["shared".into()])
        };
        assert_eq!(
            mod_ids(&prepared.root.join("mods/legacy.jar")),
            expected_ids
        );
        assert_eq!(prepared.preview.conflicts, ["mods/new.jar"]);
        assert_eq!(prepared.collisions["mods/new.jar"], ["mods/legacy.jar"]);
        let before = contents(&prepared.root);
        apply_files(&prepared, &resolutions(&["mods/new.jar"], "replace")).unwrap();
        assert_eq!(fs::read(prepared.root.join("mods/new.jar")).unwrap(), next);
        assert!(!prepared.root.join("mods/legacy.jar").exists());
        assert_eq!(
            fs::read(prepared.root.join("mods/other.jar")).unwrap(),
            other
        );
        recover(&prepared.root).unwrap();
        assert_eq!(contents(&prepared.root), before);
    }
}

#[test]
fn malformed_or_idless_legacy_metadata_does_not_create_spurious_collisions() {
    let next = jar(
        "mcmod.info",
        r#"{"modid":"shared","name":"Common display name"}"#,
    );
    for metadata in [
        "{broken",
        "null",
        "[]",
        r#"{"modList":[]}"#,
        r#"{"name":"Common display name"}"#,
        r#"[{"modid":123},{"name":"Common display name"}]"#,
    ] {
        let invalid = jar("mcmod.info", metadata);
        let temp = fixture();
        let prepared = prepare_fixture(
            &temp,
            &[],
            &[("mods/new.jar", &next)],
            &[("mods/user.jar", &invalid)],
        );
        assert!(
            mod_ids(&prepared.root.join("mods/user.jar")).is_empty(),
            "{metadata}"
        );
        assert!(prepared.preview.conflicts.is_empty(), "{metadata}");
        assert!(prepared.collisions.is_empty());
        apply_files(&prepared, &BTreeMap::new()).unwrap();
        assert_eq!(
            fs::read(prepared.root.join("mods/user.jar")).unwrap(),
            invalid
        );
        assert_eq!(fs::read(prepared.root.join("mods/new.jar")).unwrap(), next);
    }
}

#[tokio::test]
async fn failed_recovery_blocks_file_guards_and_apply_until_journal_is_recovered() {
    let temp = fixture();
    let prepared = prepare_fixture(
        &temp,
        &[("config/a", b"old")],
        &[("config/a", b"new")],
        &[("config/a", b"old")],
    );
    let handle = instance_handle(&prepared.root);
    let clone = handle.clone();
    let before = contents(&prepared.root);
    let guard = handle.try_lock_files().unwrap();
    apply_files(&prepared, &BTreeMap::new()).unwrap();
    drop(guard);
    let backup_path = prepared.root.join(JOURNAL_DIR).join("backup/config/a");
    let backup = fs::read(&backup_path).unwrap();
    fs::remove_file(&backup_path).unwrap();
    assert!(recover(&prepared.root).is_err());
    let failed = contents(&prepared.root);
    for candidate in [&handle, &clone] {
        let error = candidate
            .try_lock_files()
            .err()
            .expect("pending journal must block admission");
        assert!(error.contains("recuperación"), "{error}");
    }
    let error = apply(&handle, "unused-token".into(), BTreeMap::new())
        .await
        .unwrap_err();
    assert!(error.contains("recuperación"), "{error}");
    assert_eq!(contents(&prepared.root), failed);
    fs::write(&backup_path, backup).unwrap();
    recover(&prepared.root).unwrap();
    assert_eq!(contents(&prepared.root), before);
    assert!(!prepared.root.join(JOURNAL_DIR).exists());
    // Failed admission must release the mutex; normal mutations can resume after recovery.
    let guard = clone.try_lock_files().unwrap();
    assert!(handle.try_lock_files().is_err());
    drop(guard);
    assert!(handle.try_lock_files().is_ok());
}

#[test]
fn recovery_rejects_reserved_and_traversing_journal_paths() {
    for path in [
        "../outside",
        "upstream.json",
        "config/.git/config",
        "mods/cache.crep",
    ] {
        let temp = fixture();
        let root = temp.path().join("instance");
        put(temp.path(), "outside", b"outside");
        put(&root, "upstream.json", b"upstream");
        fs::create_dir(root.join(JOURNAL_DIR)).unwrap();
        let journal = Journal {
            paths: BTreeMap::from([(path.into(), false)]),
            committed: false,
        };
        persist_json(&root.join(JOURNAL_DIR).join("journal.json"), &journal).unwrap();
        assert!(recover(&root).is_err(), "{path}");
        assert_eq!(fs::read(temp.path().join("outside")).unwrap(), b"outside");
        assert_eq!(fs::read(root.join("upstream.json")).unwrap(), b"upstream");
        assert!(root.join(JOURNAL_DIR).exists());
    }
}

#[tokio::test]
async fn apply_rejects_stale_snapshots_before_mutating_files_or_metadata() {
    for mutation in [
        "changed",
        "deleted",
        "added collision",
        "user mod added",
        "user mod edited",
        "disabled",
        "metadata",
        "runtime",
        "expired",
    ] {
        let temp = fixture();
        let mut prepared = prepare_fixture(
            &temp,
            &[("config/a", b"old"), ("mods/pack.jar", b"pack")],
            &[
                ("config/a", b"new"),
                ("mods/pack.jar", b"pack"),
                ("mods/new.jar", b"next"),
            ],
            &[
                ("config/a", b"old"),
                ("mods/pack.jar", b"pack"),
                ("mods/user.jar", b"user"),
            ],
        );
        let mut data: InstanceData =
            serde_json::from_slice(&fs::read(prepared.root.join("instance.cub")).unwrap()).unwrap();
        data.instance_root = temp.path().to_path_buf();
        let handle = InstanceHandle::new(data);
        prepared.id = handle.uuid.to_string();
        let root = prepared.root.clone();
        match mutation {
            "changed" => put(&root, "config/a", b"edit"),
            "deleted" => fs::remove_file(root.join("config/a")).unwrap(),
            "added collision" => put(&root, "mods/new.jar", b"local addition"),
            "user mod added" => put(&root, "mods/extra.jar", b"new user mod"),
            "user mod edited" => put(&root, "mods/user.jar", b"edited user mod"),
            "disabled" => fs::rename(
                root.join("mods/pack.jar"),
                root.join("mods/pack.jar.disabled"),
            )
            .unwrap(),
            "metadata" => {
                let mut changed = prepared.old.clone();
                changed.locked = false;
                modpack::save(&root, &changed).unwrap();
            }
            "runtime" => handle.set_version("1.21.2".into()).await,
            "expired" => prepared.created = Instant::now() - PREVIEW_TTL - Duration::from_secs(1),
            _ => unreachable!(),
        }
        let metadata = fs::read(root.join(modpack::STATE_FILE)).unwrap();
        let instance = fs::read(root.join("instance.cub")).unwrap();
        let before = snapshot(&root, &prepared.old, &prepared.next).unwrap();
        let old = prepared.old.clone();
        let next = prepared.next.clone();
        let token = prepared.preview.token.clone();
        PREVIEWS.lock().unwrap().insert(token.clone(), prepared);
        let _guard = handle.try_lock_files().unwrap();
        let error = apply(&handle, token.clone(), BTreeMap::new())
            .await
            .unwrap_err();
        assert!(error.contains("cambió"), "{mutation}: {error}");
        assert!(!PREVIEWS.lock().unwrap().contains_key(&token));
        assert_eq!(snapshot(&root, &old, &next).unwrap(), before, "{mutation}");
        assert_eq!(fs::read(root.join(modpack::STATE_FILE)).unwrap(), metadata);
        assert_eq!(fs::read(root.join("instance.cub")).unwrap(), instance);
        assert!(!root.join(JOURNAL_DIR).exists());
    }
}

#[cfg(unix)]
#[test]
fn snapshot_and_recovery_reject_symlink_destinations_without_touching_target() {
    use std::os::unix::fs::symlink;
    let temp = fixture();
    let prepared = prepare_fixture(
        &temp,
        &[("config/a", b"old")],
        &[("config/a", b"new")],
        &[("config/a", b"old")],
    );
    apply_files(&prepared, &BTreeMap::new()).unwrap();
    let outside = temp.path().join("outside");
    fs::write(&outside, b"outside").unwrap();
    fs::remove_file(prepared.root.join("config/a")).unwrap();
    symlink(&outside, prepared.root.join("config/a")).unwrap();
    assert!(snapshot(&prepared.root, &prepared.old, &prepared.next).is_err());
    assert!(recover(&prepared.root).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"outside");
    assert!(prepared.root.join(JOURNAL_DIR).exists());
    // Once the obstacle is removed, the retained journal supports a real retry.
    fs::remove_file(prepared.root.join("config/a")).unwrap();
    recover(&prepared.root).unwrap();
    assert_eq!(fs::read(prepared.root.join("config/a")).unwrap(), b"old");
    assert_eq!(fs::read(&outside).unwrap(), b"outside");
}
