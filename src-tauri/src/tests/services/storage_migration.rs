use super::*;
use std::fs;
use std::sync::atomic::Ordering;

fn fixture() -> (tempfile::TempDir, tempfile::TempDir) {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("saves")).unwrap();
    fs::write(source.path().join("saves/world.dat"), b"world").unwrap();
    fs::write(source.path().join("options.txt"), b"opts").unwrap();
    (source, target)
}

#[test]
fn transfer_reports_hardlink_for_a_fresh_destination() {
    let (source, target) = fixture();
    let (strategy, failures) = move_tree(
        source.path(),
        target.path(),
        &AtomicBool::new(false),
        |_, _, _, _| {},
    )
    .unwrap();
    assert_eq!(strategy, "hardlink");
    assert!(failures.is_empty());
    assert_eq!(
        fs::read(target.path().join("saves/world.dat")).unwrap(),
        b"world"
    );
    assert_eq!(
        fs::read(target.path().join("options.txt")).unwrap(),
        b"opts"
    );
}

#[test]
fn destination_that_already_exists_falls_back_to_copy_and_overwrites() {
    let (source, target) = fixture();
    // Un destino preexistente impide el hardlink y ejercita el fallback a copia.
    fs::create_dir_all(target.path().join("saves")).unwrap();
    fs::write(target.path().join("saves/world.dat"), b"stale").unwrap();
    let (strategy, failures) = move_tree(
        source.path(),
        target.path(),
        &AtomicBool::new(false),
        |_, _, _, _| {},
    )
    .unwrap();
    assert_eq!(strategy, "copy");
    assert!(failures.is_empty());
    assert_eq!(
        fs::read(target.path().join("saves/world.dat")).unwrap(),
        b"world"
    );
    // El original nunca se toca durante el traslado.
    assert_eq!(
        fs::read(source.path().join("saves/world.dat")).unwrap(),
        b"world"
    );
}

#[test]
fn cancellation_leaves_the_destination_empty_and_the_source_intact() {
    let (source, target) = fixture();
    let cancelled = AtomicBool::new(true);
    let result = move_tree(source.path(), target.path(), &cancelled, |_, _, _, _| {});
    assert!(result.is_err());
    assert!(fs::read_dir(target.path()).unwrap().next().is_none());
    assert!(source.path().join("saves/world.dat").is_file());
    assert!(cancelled.load(Ordering::Relaxed));
}

#[test]
fn remove_tree_deletes_the_whole_original_directory() {
    let (source, _target) = fixture();
    remove_tree(source.path());
    assert!(!source.path().exists());
}

#[cfg(unix)]
#[test]
fn hardlinks_share_one_copy_and_survive_removing_the_original() {
    use std::os::unix::fs::MetadataExt;

    let (source, target) = fixture();
    let mut reported = (0u64, 0u64);
    let (strategy, failures) = move_tree(
        source.path(),
        target.path(),
        &AtomicBool::new(false),
        |current, total, _, _| reported = (current, total),
    )
    .unwrap();
    assert_eq!(strategy, "hardlink");
    assert!(failures.is_empty());
    assert_eq!(reported, (9, 9)); // "world" (5) + "opts" (4)

    // Hardlink: mismo inodo y dos enlaces al mismo contenido.
    let src_meta = fs::metadata(source.path().join("saves/world.dat")).unwrap();
    let dst_meta = fs::metadata(target.path().join("saves/world.dat")).unwrap();
    assert_eq!(src_meta.ino(), dst_meta.ino());
    assert_eq!(src_meta.nlink(), 2);

    // Borrar el original no pierde datos: el enlace destino mantiene el contenido.
    remove_tree(source.path());
    assert!(!source.path().exists());
    assert_eq!(
        fs::read(target.path().join("saves/world.dat")).unwrap(),
        b"world"
    );
    assert_eq!(
        fs::read(target.path().join("options.txt")).unwrap(),
        b"opts"
    );
    assert_eq!(
        fs::metadata(target.path().join("saves/world.dat"))
            .unwrap()
            .nlink(),
        1
    );
}

#[cfg(unix)]
#[test]
fn empty_directories_are_reported_without_files() {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("empty")).unwrap();
    let mut entries = Vec::new();
    let mut total = 0;
    collect_files(
        source.path(),
        source.path(),
        &mut entries,
        &mut total,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(entries.is_empty());
    assert_eq!(total, 0);
}
