use super::*;
use std::io::Cursor;

#[test]
fn rejects_portable_traversal_and_windows_aliases() {
    for name in [
        "../../test.txt",
        "../outside/file.txt",
        "foo/../../outside/file.txt",
        "/outside/file.txt",
        "C:/outside/file.txt",
        "C:file.txt",
        "\\\\server\\share\\file",
        "foo\\..\\file",
        "foo/.. /file",
        "foo/file:stream",
        "foo/NUL.txt",
        "foo/NUL .txt",
        "foo/CON",
        "foo/file.",
        "foo/\0",
    ] {
        assert!(archive_path(name).is_err(), "{name:?}");
    }
    assert_eq!(
        archive_path("overrides\\config\\ok.json").unwrap(),
        Path::new("overrides/config/ok.json")
    );
    for version in [
        "",
        ".",
        "..",
        "../../outside",
        "foo/../../outside",
        "/outside",
        "C:\\outside",
        "1.21:stream",
        "1.21 ",
        "NUL",
    ] {
        assert!(validate_version(version).is_err(), "{version:?}");
    }
    for version in [
        "1.21",
        "26.3-snapshot-2",
        "b1.7.3",
        "1.20.1-forge-47.2.0",
        "fabric-loader-0.16.0-1.21",
        "0.18.1+build.2",
        "3D Shareware v1.34",
    ] {
        validate_version(version).unwrap();
    }
}

#[test]
fn validates_entire_zip_before_writing_any_entry() {
    let root = tempfile::tempdir().unwrap();
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("ok.txt", options).unwrap();
    zip.write_all(b"ok").unwrap();
    zip.start_file("../outside.txt", options).unwrap();
    zip.write_all(b"bad").unwrap();
    let mut archive = zip::ZipArchive::new(zip.finish().unwrap()).unwrap();
    assert!(extract_zip(&mut archive, root.path()).is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn rejects_archive_symlinks_before_extraction() {
    let root = tempfile::tempdir().unwrap();
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.add_symlink(
        "config",
        "../outside",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    let mut archive = zip::ZipArchive::new(zip.finish().unwrap()).unwrap();
    assert!(extract_zip(&mut archive, root.path()).is_err());
    assert!(!root.path().join("config").exists());
}

#[cfg(unix)]
#[test]
fn publication_never_writes_through_symlinks_or_hardlinks() {
    let temp = tempfile::tempdir().unwrap();
    let inside = temp.path().join("inside");
    let outside = temp.path().join("outside");
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"unchanged").unwrap();
    std::os::unix::fs::symlink(&outside, inside.join("config")).unwrap();
    std::os::unix::fs::symlink(outside.join("sentinel"), inside.join("icon.png")).unwrap();
    std::fs::hard_link(outside.join("sentinel"), inside.join("hardlink")).unwrap();
    let root = ConfinedDir::open(&inside).unwrap();
    assert!(root.write(Path::new("config/sentinel"), b"bad").is_err());
    assert!(root.write(Path::new("icon.png"), b"bad").is_err());
    root.write(Path::new("hardlink"), b"new inode").unwrap();
    assert_eq!(
        std::fs::read(outside.join("sentinel")).unwrap(),
        b"unchanged"
    );
    root.write(Path::new("mods/valid.jar"), b"ok").unwrap();
    assert_eq!(root.read(Path::new("mods/valid.jar")).unwrap(), b"ok");
}

#[cfg(unix)]
#[test]
fn reads_and_deletes_refuse_symlinks_without_touching_the_target() {
    let temp = tempfile::tempdir().unwrap();
    let inside = temp.path().join("inside");
    let outside = temp.path().join("outside");
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside-data").unwrap();
    std::os::unix::fs::symlink(outside.join("sentinel"), inside.join("secret")).unwrap();
    std::os::unix::fs::symlink(&outside, inside.join("linked-dir")).unwrap();

    let root = ConfinedDir::open(&inside).unwrap();
    assert!(root.read(Path::new("secret")).is_err());
    assert!(root.read(Path::new("linked-dir/sentinel")).is_err());
    assert!(root.write(Path::new("secret"), b"bad").is_err());
    assert!(root.remove_file(Path::new("secret")).is_err());
    assert_eq!(
        std::fs::read(outside.join("sentinel")).unwrap(),
        b"outside-data"
    );
}

#[cfg(unix)]
#[test]
fn open_or_create_refuses_a_symlinked_root_and_creates_private_directories() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside");
    let linked = temp.path().join("linked-root");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, &linked).unwrap();
    assert!(ConfinedDir::open_existing(&linked).is_err());
    assert!(ConfinedDir::open_or_create(&linked).is_err());
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);

    let root = temp.path().join("secrets");
    let dir = ConfinedDir::open_or_create(&root).unwrap();
    dir.write_private(Path::new("token.enc"), b"data").unwrap();
    assert_eq!(
        std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(root.join("token.enc"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[cfg(unix)]
#[test]
fn replacing_the_root_path_does_not_redirect_an_open_handle() {
    let temp = tempfile::tempdir().unwrap();
    let inside = temp.path().join("inside");
    let outside = temp.path().join("outside");
    let moved = temp.path().join("inside-old");
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();

    let root = ConfinedDir::open(&inside).unwrap();
    std::fs::rename(&inside, &moved).unwrap();
    std::os::unix::fs::symlink(&outside, &inside).unwrap();

    // The already-open capability still points at the original directory, so
    // the replacement link cannot redirect the write.
    root.write(Path::new("secret"), b"inside").unwrap();
    assert_eq!(std::fs::read(moved.join("secret")).unwrap(), b"inside");
    assert!(!outside.join("secret").exists());
}

#[cfg(unix)]
#[test]
fn concurrent_symlink_swaps_never_escape_the_root() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let temp = tempfile::tempdir().unwrap();
    let inside = temp.path().join("inside");
    let outside = temp.path().join("outside");
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();
    let sentinel = outside.join("sentinel");
    std::fs::write(&sentinel, b"outside-data").unwrap();

    let root = ConfinedDir::open(&inside).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let target = inside.join("secret");
    let attacker_target = target.clone();
    let attacker_sentinel = sentinel.clone();
    let attacker_stop = Arc::clone(&stop);
    let attacker = std::thread::spawn(move || {
        while !attacker_stop.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&attacker_target);
            let _ = std::os::unix::fs::symlink(&attacker_sentinel, &attacker_target);
            let _ = std::fs::remove_file(&attacker_target);
            let _ = std::fs::write(&attacker_target, b"inside");
        }
    });

    for _ in 0..500 {
        let _ = root.write(Path::new("secret"), b"new");
        let _ = root.read(Path::new("secret"));
        let _ = root.remove_file(Path::new("secret"));
    }
    stop.store(true, Ordering::Relaxed);
    attacker.join().unwrap();

    assert_eq!(std::fs::read(&sentinel).unwrap(), b"outside-data");
}

#[cfg(windows)]
#[test]
fn windows_reparse_points_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let inside = temp.path().join("inside");
    let outside = temp.path().join("outside");
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside-data").unwrap();

    let linked_dir = inside.join("linked");
    let linked_file = inside.join("secret");
    let dir_link = std::os::windows::fs::symlink_dir(&outside, &linked_dir);
    let file_link = std::os::windows::fs::symlink_file(outside.join("sentinel"), &linked_file);
    if dir_link.is_err() || file_link.is_err() {
        eprintln!("skipping Windows reparse test: symlink creation is unavailable");
        return;
    }

    let root = ConfinedDir::open(&inside).unwrap();
    assert!(root.read(Path::new("linked/sentinel")).is_err());
    assert!(root.read(Path::new("secret")).is_err());
    assert!(root.write(Path::new("secret"), b"bad").is_err());
    assert!(root.remove_file(Path::new("secret")).is_err());
    assert_eq!(
        std::fs::read(outside.join("sentinel")).unwrap(),
        b"outside-data"
    );
}

#[cfg(windows)]
#[test]
fn windows_reparse_root_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside");
    let root = temp.path().join("root");
    std::fs::create_dir(&outside).unwrap();
    if std::os::windows::fs::symlink_dir(&outside, &root).is_err() {
        eprintln!("skipping Windows reparse test: symlink creation is unavailable");
        return;
    }
    assert!(ConfinedDir::open_existing(&root).is_err());
    assert!(ConfinedDir::open_or_create(&root).is_err());
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
}
