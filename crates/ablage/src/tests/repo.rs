use super::*;

fn entry(value: u8) -> Entry {
    Entry {
        version: 1,
        fingerprint: u64::from(value),
        data: vec![value; 256],
    }
}

#[test]
fn repeated_reads_share_storage_and_writes_are_isolated_until_flush() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.crep");
    let mut initial = Repo::open(&path);
    initial.put("mod", entry(1));
    initial.flush().unwrap();

    let mut writer = Repo::open(&path);
    let reader = Repo::open(&path);
    assert!(Arc::ptr_eq(&writer.entries, &reader.entries));
    writer.put("mod", entry(2));
    assert_eq!(reader.get("mod"), Some(&entry(1)));
    assert_eq!(Repo::open(&path).get("mod"), Some(&entry(1)));
    writer.flush().unwrap();
    assert_eq!(Repo::open(&path).get("mod"), Some(&entry(2)));
    assert_eq!(reader.get("mod"), Some(&entry(1)));
}

#[test]
fn identical_updates_and_missing_removals_do_not_clone_or_rewrite_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.crep");
    let mut repo = Repo::open(&path);
    repo.put("mod", entry(1));
    repo.flush().unwrap();
    let mut repo = Repo::open(&path);
    let snapshot = repo.entries.clone();
    repo.put("mod", entry(1));
    assert!(repo.remove("missing").is_none());
    assert!(!repo.is_dirty());
    assert!(Arc::ptr_eq(&snapshot, &repo.entries));

    // A clean flush must do no filesystem work, even after the directory is gone.
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(dir.path()).unwrap();
    repo.flush().unwrap();
}

#[test]
fn external_replacement_deletion_and_corruption_invalidate_warm_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.crep");
    let other = dir.path().join("replacement.crep");
    for (file, value) in [(&path, 1), (&other, 2)] {
        let mut repo = Repo::open(file);
        repo.put("mod", entry(value));
        repo.flush().unwrap();
    }
    let original = Repo::open(&path);
    assert_eq!(original.get("mod"), Some(&entry(1)));
    std::fs::remove_file(&path).unwrap();
    std::fs::rename(&other, &path).unwrap();
    assert_eq!(Repo::open(&path).get("mod"), Some(&entry(2)));
    std::fs::remove_file(&path).unwrap();
    assert!(!Repo::open(&path).has("mod"));
    std::fs::write(&path, b"corrupt cache").unwrap();
    assert!(!Repo::open(&path).has("mod"));
    let mut repaired = Repo::open(&path);
    repaired.put("mod", entry(3));
    repaired.flush().unwrap();
    assert_eq!(Repo::open(&path).get("mod"), Some(&entry(3)));
}

#[test]
fn failed_flush_is_not_published_to_other_readers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.crep");
    let mut repo = Repo::open(&path);
    repo.put("mod", entry(1));
    repo.flush().unwrap();
    assert_eq!(Repo::open(&path).get("mod"), Some(&entry(1)));
    repo.put("mod", entry(2));
    std::fs::create_dir(path.with_extension("crep.tmp")).unwrap();
    assert!(repo.flush().is_err());
    assert!(repo.is_dirty());
    assert_eq!(Repo::open(&path).get("mod"), Some(&entry(1)));
    std::fs::remove_dir(path.with_extension("crep.tmp")).unwrap();
    repo.flush().unwrap();
    assert_eq!(Repo::open(&path).get("mod"), Some(&entry(2)));
}

#[test]
fn warm_read_benchmark() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.crep");
    let mut repo = Repo::open(&path);
    for i in 0..2000 {
        repo.put(
            format!("mod-{i}"),
            Entry {
                version: 1,
                fingerprint: i,
                data: vec![42; 1024],
            },
        );
    }
    repo.flush().unwrap();
    let warm = Repo::open(&path);
    let started = std::time::Instant::now();
    for _ in 0..50 {
        assert_eq!(read_all(&path).unwrap().len(), 2000);
    }
    let uncached = started.elapsed();
    let started = std::time::Instant::now();
    for _ in 0..50 {
        let cached = Repo::open(&path);
        assert!(Arc::ptr_eq(&warm.entries, &cached.entries));
    }
    println!(
        "[repository] 2000 entries, 50 opens: uncached={uncached:?}, cached={:?}",
        started.elapsed()
    );
}
