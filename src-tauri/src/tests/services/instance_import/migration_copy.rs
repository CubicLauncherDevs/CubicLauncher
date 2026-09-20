use super::*;

#[test]
fn copies_content_and_empty_directories_without_credentials_or_runtimes() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join("saves/world/empty")).unwrap();
    fs::write(source.path().join("saves/world/level.dat"), b"world").unwrap();
    fs::write(source.path().join("options.txt"), b"settings").unwrap();
    fs::write(source.path().join("launcher_accounts.json"), b"credentials").unwrap();
    fs::create_dir(source.path().join("libraries")).unwrap();
    fs::write(source.path().join("libraries/test.jar"), b"runtime").unwrap();
    let mut final_bytes = (0, 0);
    copy_game(
        source.path(),
        target.path(),
        None,
        &AtomicBool::new(false),
        |copied, total, _| final_bytes = (copied, total),
    )
    .unwrap();
    assert_eq!(final_bytes, (13, 13));
    assert_eq!(
        fs::read(target.path().join("saves/world/level.dat")).unwrap(),
        b"world"
    );
    assert!(target.path().join("saves/world/empty").is_dir());
    assert!(!target.path().join("launcher_accounts.json").exists());
    assert!(!target.path().join("libraries").exists());
    assert_eq!(
        fs::read(source.path().join("saves/world/level.dat")).unwrap(),
        b"world"
    );
}

#[test]
fn cancellation_interrupts_copy_and_staging_is_removed() {
    let source = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    fs::write(source.path().join("options.txt"), b"settings").unwrap();
    let cancelled = AtomicBool::new(false);
    let stage = tempfile::tempdir_in(parent.path()).unwrap();
    let stage_path = stage.path().to_path_buf();
    let result = copy_game(source.path(), stage.path(), None, &cancelled, |_, _, _| {
        cancelled.store(true, Ordering::Relaxed)
    });
    assert!(result.is_err());
    assert!(!stage.path().join("options.txt").exists());
    drop(stage);
    assert!(!stage_path.exists());
    assert!(source.path().join("options.txt").exists());
}

#[test]
fn a_source_removed_after_preview_does_not_create_an_empty_migration() {
    let parent = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let result = copy_game(
        &parent.path().join("missing"),
        target.path(),
        None,
        &AtomicBool::new(false),
        |_, _, _| {},
    );
    assert!(result.unwrap_err().contains("no longer exists"));
}

#[cfg(unix)]
#[test]
fn symbolic_links_fail_instead_of_recursing_or_copying_unrelated_data() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("mods")).unwrap();
    std::os::unix::fs::symlink(source.path(), source.path().join("mods/loop")).unwrap();
    let result = copy_game(
        source.path(),
        target.path(),
        None,
        &AtomicBool::new(false),
        |_, _, _| {},
    );
    assert!(result.unwrap_err().contains("symbolic link"));
    assert!(fs::read_dir(target.path()).unwrap().next().is_none());
}
