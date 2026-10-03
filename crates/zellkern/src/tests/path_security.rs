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
