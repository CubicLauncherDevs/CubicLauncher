use super::*;

fn temp_store() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("options_profiles.cub");
    (dir, path)
}

#[test]
fn create_persists_and_reloads() {
    let (_dir, path) = temp_store();
    let mut store = StoreFile::default();
    let created = store.create("PvP", "fov:0.5\n").unwrap();
    store.persist_to(&path).unwrap();

    let reloaded = StoreFile::load_from(&path);
    assert_eq!(reloaded.list().len(), 1);
    let profile = reloaded.get(&created.id).unwrap();
    assert_eq!(profile.name, "PvP");
    assert_eq!(profile.content, "fov:0.5\n");
    assert_eq!(profile.created_at, profile.updated_at);
}

#[test]
fn update_changes_name_and_content() {
    let mut store = StoreFile::default();
    let created = store.create("Old", "a:1\n").unwrap();
    let updated = store.update(&created.id, "New", "b:2\n").unwrap();
    assert_eq!(updated.name, "New");
    assert_eq!(updated.content, "b:2\n");
    assert_eq!(store.list().len(), 1);
}

#[test]
fn update_missing_profile_errors() {
    let mut store = StoreFile::default();
    assert!(store.update("nope", "New", "x").is_err());
}

#[test]
fn delete_removes_profile() {
    let mut store = StoreFile::default();
    let created = store.create("Temp", "x:1\n").unwrap();
    store.delete(&created.id).unwrap();
    assert!(store.list().is_empty());
    assert!(store.delete(&created.id).is_err());
}

#[test]
fn empty_name_is_rejected() {
    let mut store = StoreFile::default();
    assert!(store.create("   ", "x").is_err());
}

#[test]
fn long_name_is_rejected() {
    let mut store = StoreFile::default();
    assert!(store.create(&"a".repeat(MAX_NAME_LEN + 1), "x").is_err());
}

#[test]
fn oversized_content_is_rejected() {
    let mut store = StoreFile::default();
    let big = "x".repeat(MAX_CONTENT_BYTES + 1);
    assert!(store.create("Big", &big).is_err());
}

#[test]
fn corrupt_store_falls_back_to_empty() {
    let (_dir, path) = temp_store();
    std::fs::write(&path, b"{ not json").unwrap();
    let store = StoreFile::load_from(&path);
    assert!(store.list().is_empty());
    assert!(path.with_extension("cub.bak").exists());
}

#[test]
fn apply_creates_file_without_backup() {
    let root = tempfile::tempdir().unwrap();
    let instance_dir = root.path().join("Pack");
    let backup_root = root.path().join("backups");
    let outcome = apply_to_instance(
        &instance_dir,
        "uuid-1",
        "fov:0.5\n",
        BackupMode::Timestamped,
        &backup_root,
    )
    .unwrap();
    assert!(outcome.changed);
    assert!(outcome.backup.is_none());
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("options.txt")).unwrap(),
        "fov:0.5\n"
    );
}

#[test]
fn apply_identical_content_is_noop() {
    let root = tempfile::tempdir().unwrap();
    let instance_dir = root.path().join("Pack");
    std::fs::create_dir_all(&instance_dir).unwrap();
    std::fs::write(instance_dir.join("options.txt"), "fov:0.5\n").unwrap();
    let outcome = apply_to_instance(
        &instance_dir,
        "uuid-1",
        "fov:0.5\n",
        BackupMode::Rolling,
        &root.path().join("backups"),
    )
    .unwrap();
    assert!(!outcome.changed);
    assert!(outcome.backup.is_none());
}

#[test]
fn apply_overwrites_and_backs_up() {
    let root = tempfile::tempdir().unwrap();
    let instance_dir = root.path().join("Pack");
    std::fs::create_dir_all(&instance_dir).unwrap();
    std::fs::write(instance_dir.join("options.txt"), "old:1\n").unwrap();
    let backup_root = root.path().join("backups");

    let outcome = apply_to_instance(
        &instance_dir,
        "uuid-1",
        "new:2\n",
        BackupMode::Rolling,
        &backup_root,
    )
    .unwrap();
    assert!(outcome.changed);
    let backup = outcome.backup.unwrap();
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), "old:1\n");
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("options.txt")).unwrap(),
        "new:2\n"
    );
}

#[test]
fn timestamped_backups_are_pruned() {
    let root = tempfile::tempdir().unwrap();
    let instance_dir = root.path().join("Pack");
    std::fs::create_dir_all(&instance_dir).unwrap();
    let backup_root = root.path().join("backups");
    for i in 0..(MAX_TIMESTAMPED_BACKUPS + 3) {
        std::fs::write(instance_dir.join("options.txt"), format!("v:{i}\n")).unwrap();
        apply_to_instance(
            &instance_dir,
            "uuid-1",
            &format!("new:{i}\n"),
            BackupMode::Timestamped,
            &backup_root,
        )
        .unwrap();
    }
    let backups = std::fs::read_dir(backup_root.join("uuid-1"))
        .unwrap()
        .count();
    assert!(backups <= MAX_TIMESTAMPED_BACKUPS);
}
