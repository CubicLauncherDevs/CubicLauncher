use super::*;
use std::fs;
use std::path::Path;

#[tokio::test]
async fn storage_admission_excludes_cleanup_until_all_readers_and_workers_finish() {
    // Use the real gate without initializing global paths or touching user data.
    let first = acquire().await;
    let second = acquire().await;
    assert!(try_exclusive().is_err());
    let worker_guard = first.clone();
    let (release, wait) = std::sync::mpsc::channel();
    let worker = tokio::task::spawn_blocking(move || {
        let _guard = worker_guard;
        wait.recv().unwrap();
    });
    drop(first);
    drop(second);
    assert!(
        try_exclusive().is_err(),
        "blocking worker still uses shared"
    );
    release.send(()).unwrap();
    worker.await.unwrap();
    let exclusive = try_exclusive().unwrap();
    let mut reader = std::pin::pin!(acquire());
    assert!(futures::poll!(&mut reader).is_pending());
    drop(exclusive);
    let reader = reader.await;
    assert!(try_exclusive().is_err());
    drop(reader);
    assert!(try_exclusive().is_ok());
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let versions = dir.path().join("versions").join("1.20.4");
    fs::create_dir_all(&versions).unwrap();
    fs::create_dir_all(dir.path().join("libraries")).unwrap();
    fs::write(versions.join("1.20.4.jar"), vec![0u8; 1024]).unwrap();
    fs::write(dir.path().join("libraries/x.jar"), b"lib").unwrap();
    fs::write(dir.path().join("loose.txt"), b"hi").unwrap();
    dir
}

#[test]
fn usage_counts_files_bytes_and_directories() {
    let dir = fixture();
    let mut usage = DirUsage::default();
    compute_usage(dir.path(), &mut usage);
    assert!(!usage.blocked);
    assert_eq!(usage.files, 3);
    assert_eq!(usage.bytes, 1024 + 3 + 2);
}

#[test]
fn usage_of_a_missing_directory_is_empty() {
    let usage = dir_usage(Path::new("/nonexistent/cubic-shared-test"));
    assert_eq!(usage.bytes, 0);
    assert_eq!(usage.files, 0);
    assert!(!usage.blocked);
}

#[cfg(unix)]
#[test]
fn symlinks_do_not_block_usage_and_are_not_counted() {
    let dir = fixture();
    // Los JRE gestionados traen symlinks en legal/: no deben bloquear la
    // purga ni aportar al tamaño.
    std::os::unix::fs::symlink("/etc", dir.path().join("legal")).unwrap();
    let mut usage = DirUsage::default();
    compute_usage(dir.path(), &mut usage);
    assert!(!usage.blocked);
    assert_eq!(usage.files, 3);
    assert_eq!(usage.bytes, 1024 + 3 + 2);
}

#[test]
fn clear_dir_empties_the_tree_but_keeps_the_root() {
    let dir = fixture();
    let failures = clear_dir(dir.path()).unwrap();
    assert!(failures.is_empty());
    assert!(dir.path().exists());
    assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[cfg(unix)]
#[test]
fn clear_dir_removes_symlinks_but_never_their_target() {
    let target = tempfile::tempdir().unwrap();
    fs::write(target.path().join("precious.txt"), b"keep").unwrap();

    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    std::os::unix::fs::symlink(target.path(), dir.path().join("link")).unwrap();

    let failures = clear_dir(dir.path()).unwrap();
    assert!(failures.is_empty());
    // El enlace se borra; el destino queda exactamente como estaba.
    assert!(!dir.path().join("link").exists());
    assert!(!dir.path().join("a.txt").exists());
    assert_eq!(
        fs::read_to_string(target.path().join("precious.txt")).unwrap(),
        "keep"
    );
}

#[tokio::test]
async fn purge_measures_then_clears_the_directory() {
    let dir = fixture();
    let usage = purge_shared_at(dir.path().to_path_buf()).await.unwrap();
    assert!(!usage.blocked);
    assert_eq!(usage.files, 3);
    assert_eq!(usage.bytes, 1024 + 3 + 2);
    // La carpeta queda vacía (pero existe) lista para re-descargar.
    assert!(dir.path().exists());
    assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[tokio::test]
async fn purge_of_a_missing_path_is_a_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");
    let usage = purge_shared_at(missing).await.unwrap();
    assert_eq!(usage.bytes, 0);
    assert_eq!(usage.files, 0);
}
