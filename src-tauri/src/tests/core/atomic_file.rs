use super::*;

#[test]
fn preparing_or_abandoning_a_write_preserves_the_previous_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.cub");
    write(&path, b"old").unwrap();
    let temporary = prepare(&path, b"new").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"old");
    drop(temporary);
    assert_eq!(std::fs::read(&path).unwrap(), b"old");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    write(&path, b"new").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"new");
}

#[test]
fn failed_publication_cleans_up_the_temporary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("occupied");
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("keep"), b"original").unwrap();
    assert!(write(&path, b"replacement").is_err());
    assert_eq!(std::fs::read(path.join("keep")).unwrap(), b"original");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
