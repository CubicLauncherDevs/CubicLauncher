use super::*;

fn level(name: &str) -> HashMap<String, Value> {
    HashMap::from([(
        "Data".into(),
        Value::Compound(HashMap::from([
            ("LevelName".into(), Value::String(name.into())),
            ("GameType".into(), Value::Int(1)),
            ("LastPlayed".into(), Value::Long(1_700_000_000_000)),
            ("hardcore".into(), Value::Byte(0)),
            (
                "Version".into(),
                Value::Compound(HashMap::from([(
                    "Name".into(),
                    Value::String("1.21.1".into()),
                )])),
            ),
            (
                "mod:unknown".into(),
                Value::Compound(HashMap::from([
                    (
                        "bytes".into(),
                        Value::ByteArray(fastnbt::ByteArray::new(vec![-1, 0, 127])),
                    ),
                    (
                        "ints".into(),
                        Value::IntArray(fastnbt::IntArray::new(vec![i32::MIN, 42])),
                    ),
                    (
                        "longs".into(),
                        Value::LongArray(fastnbt::LongArray::new(vec![i64::MAX])),
                    ),
                    ("list".into(), Value::List(vec![Value::String("🌎".into())])),
                ])),
            ),
        ])),
    )])
}

fn write_level(path: &Path, data: &HashMap<String, Value>) {
    let mut gzip = GzEncoder::new(File::create(path).unwrap(), Compression::fast());
    gzip.write_all(&fastnbt::to_bytes(data).unwrap()).unwrap();
    gzip.finish().unwrap();
}

fn world(saves: &Path, folder: &str) -> PathBuf {
    let path = saves.join(folder);
    fs::create_dir(&path).unwrap();
    write_level(&path.join("level.dat"), &level("Mi mundo 🌎"));
    path
}

fn zip_fixture(path: &Path, files: &[(&str, &[u8])]) {
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    for (name, content) in files {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(content).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn rename_preserves_all_unknown_nbt_types_and_existing_backup() {
    let temp = tempfile::tempdir().unwrap();
    let world = world(temp.path(), "folder");
    fs::write(world.join("level.dat_old"), b"existing backup").unwrap();
    let mut expected = level("Mi mundo 🌎");
    rename_world(&world, "Nuevo 🌍").unwrap();
    let Value::Compound(data) = expected.get_mut("Data").unwrap() else {
        panic!()
    };
    data.insert("LevelName".into(), Value::String("Nuevo 🌍".into()));
    let actual: HashMap<String, Value> =
        fastnbt::from_bytes(&read_level(&world.join("level.dat")).unwrap()).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        fs::read(world.join("level.dat_old")).unwrap(),
        b"existing backup"
    );
    assert!(world.is_dir());
}

#[test]
fn list_supports_legacy_corrupt_backup_and_cache_invalidation() {
    let temp = tempfile::tempdir().unwrap();
    let modern = world(temp.path(), "modern");
    let legacy = temp.path().join("legacy");
    fs::create_dir(&legacy).unwrap();
    write_level(
        &legacy.join("level.dat"),
        &HashMap::from([("Data".into(), Value::Compound(HashMap::new()))]),
    );
    let corrupt = world(temp.path(), "corrupt");
    fs::rename(corrupt.join("level.dat"), corrupt.join("level.dat_old")).unwrap();
    fs::write(corrupt.join("level.dat"), b"broken").unwrap();
    let unreadable = temp.path().join("unreadable");
    fs::create_dir(&unreadable).unwrap();
    fs::write(unreadable.join("level.dat"), b"broken").unwrap();
    fs::create_dir(temp.path().join("not-a-world")).unwrap();
    let worlds = list_worlds(temp.path()).unwrap();
    assert_eq!(worlds.len(), 4);
    let modern_dto = worlds.iter().find(|w| w.folder == "modern").unwrap();
    assert_eq!(modern_dto.version.as_deref(), Some("1.21.1"));
    assert_eq!(modern_dto.game_mode, Some(1));
    assert!(!modern_dto.metadata_error);
    let legacy_dto = worlds.iter().find(|w| w.folder == "legacy").unwrap();
    assert_eq!(legacy_dto.name, "legacy");
    assert_eq!(legacy_dto.version, None);
    assert!(!legacy_dto.metadata_error);
    let fallback = worlds.iter().find(|w| w.folder == "corrupt").unwrap();
    assert_eq!(fallback.name, "Mi mundo 🌎");
    assert!(fallback.metadata_error);
    assert_eq!(
        worlds
            .iter()
            .find(|w| w.folder == "unreadable")
            .unwrap()
            .name,
        "unreadable"
    );
    write_level(
        &modern.join("level.dat"),
        &level("Externally changed name with a different length"),
    );
    assert_eq!(
        list_worlds(temp.path())
            .unwrap()
            .iter()
            .find(|w| w.folder == "modern")
            .unwrap()
            .name,
        "Externally changed name with a different length"
    );
    assert!(world_dir(temp.path(), "../modern").is_err());
    assert!(world_dir(temp.path(), "not-a-world").is_err());
}

#[test]
fn failed_rename_leaves_original_bytes_untouched() {
    let temp = tempfile::tempdir().unwrap();
    let path = world(temp.path(), "world");
    let original = fs::read(path.join("level.dat")).unwrap();
    assert!(rename_world(&path, "   ").is_err());
    assert!(rename_world(&path, "bad\0name").is_err());
    assert_eq!(fs::read(path.join("level.dat")).unwrap(), original);
    fs::write(path.join("level.dat"), b"corrupt").unwrap();
    assert!(rename_world(&path, "new").is_err());
    assert_eq!(fs::read(path.join("level.dat")).unwrap(), b"corrupt");
}

#[test]
fn seed_is_read_from_random_seed_or_world_gen_settings() {
    let temp = tempfile::tempdir().unwrap();
    let saves = saves_dir(temp.path(), true).unwrap();
    let legacy = world(&saves, "legacy");
    let mut data_legacy = level("legacy");
    let Value::Compound(legacy_data) = data_legacy.get_mut("Data").unwrap() else {
        panic!()
    };
    legacy_data.insert("RandomSeed".into(), Value::Long(123_456_789));
    write_level(&legacy.join("level.dat"), &data_legacy);

    let modern = world(&saves, "modern");
    let mut data_modern = level("modern");
    let Value::Compound(modern_data) = data_modern.get_mut("Data").unwrap() else {
        panic!()
    };
    modern_data.insert(
        "WorldGenSettings".into(),
        Value::Compound(HashMap::from([("seed".into(), Value::Long(-987_654_321))])),
    );
    write_level(&modern.join("level.dat"), &data_modern);

    let _no_seed = world(&saves, "no-seed");

    let worlds = list_worlds(&saves).unwrap();
    let legacy_dto = worlds.iter().find(|w| w.folder == "legacy").unwrap();
    let modern_dto = worlds.iter().find(|w| w.folder == "modern").unwrap();
    let no_seed_dto = worlds.iter().find(|w| w.folder == "no-seed").unwrap();
    assert_eq!(legacy_dto.seed.as_deref(), Some("123456789"));
    assert_eq!(modern_dto.seed.as_deref(), Some("-987654321"));
    assert_eq!(no_seed_dto.seed, None);
}

#[test]
fn reset_icon_removes_icon_png_and_invalidates_cache() {
    let temp = tempfile::tempdir().unwrap();
    let saves = saves_dir(temp.path(), true).unwrap();
    let world = world(&saves, "icon-world");
    fs::write(world.join("icon.png"), b"png data").unwrap();

    let before = list_worlds(&saves).unwrap();
    let before_dto = before.iter().find(|w| w.folder == "icon-world").unwrap();
    assert!(before_dto.icon.is_some());

    assert!(reset_world_icon(&world).unwrap());
    assert!(!world.join("icon.png").exists());

    let after = list_worlds(&saves).unwrap();
    let after_dto = after.iter().find(|w| w.folder == "icon-world").unwrap();
    assert!(after_dto.icon.is_none());

    assert!(!reset_world_icon(&world).unwrap());
}

#[test]
fn duplicate_export_import_roundtrip_preserves_data_without_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    let saves = saves_dir(temp.path(), true).unwrap();
    let original = world(&saves, "world");
    fs::create_dir_all(original.join("dimensions/mod/moon/region")).unwrap();
    fs::write(
        original.join("dimensions/mod/moon/region/r.0.0.mca"),
        b"region data",
    )
    .unwrap();
    fs::write(original.join("session.lock"), b"lock").unwrap();
    let mut events = Vec::new();
    let mut callback = |event| events.push(event);
    let mut progress = Progress::new(&mut callback);
    let copy = duplicate(&original, &saves, &mut progress).unwrap();
    assert_eq!(copy, "world (1)");
    assert!(!saves.join(&copy).join("session.lock").exists());
    let zip = temp.path().join("export.zip");
    export_zip(&original, &zip, &mut progress).unwrap();
    let exported = fs::read(&zip).unwrap();
    assert!(export_zip(&original, &zip, &mut progress).is_err());
    assert_eq!(fs::read(&zip).unwrap(), exported);
    let imported = import_world(&zip, &saves, &mut progress).unwrap();
    let copied_folder = import_world(&original, &saves, &mut progress).unwrap();
    assert_eq!(copied_folder, "world (2)");
    assert_eq!(
        fs::read(
            saves
                .join(&imported)
                .join("dimensions/mod/moon/region/r.0.0.mca")
        )
        .unwrap(),
        b"region data"
    );
    assert_eq!(
        fs::read(original.join("level.dat")).unwrap(),
        fs::read(saves.join(&imported).join("level.dat")).unwrap()
    );
    assert!(export_zip(&original, &original.join("bad.zip"), &mut progress).is_err());
    let expected_size = fs::metadata(original.join("level.dat")).unwrap().len() + 11 + 4;
    assert_eq!(world_size(&original, &mut progress).unwrap(), expected_size);
    progress.flush();
    assert!(events.last().unwrap().bytes > expected_size);
    delete_world(&saves.join(copy)).unwrap();
    assert!(original.is_dir());
}

#[test]
fn imports_wrapped_zip_and_rejects_ambiguous_or_escaping_archives() {
    let temp = tempfile::tempdir().unwrap();
    let saves = saves_dir(temp.path(), true).unwrap();
    let zip = temp.path().join("world.zip");
    let mut noop = |_| {};
    let mut progress = Progress::new(&mut noop);
    zip_fixture(
        &zip,
        &[
            ("Wrapper/My world/level.dat", b"damaged but recoverable"),
            ("Wrapper/My world/region/r.mca", b"region"),
            ("readme.txt", b"outside"),
        ],
    );
    let imported = import_world(&zip, &saves, &mut progress).unwrap();
    assert_eq!(imported, "My world");
    assert!(!saves.join(&imported).join("readme.txt").exists());
    assert!(saves.join(&imported).join("region/r.mca").is_file());
    for bad in [
        "../escape",
        "/absolute",
        "C:/escape",
        "world/../../escape",
        "..\\escape",
        "world/evil:stream",
        "world/./alias",
    ] {
        zip_fixture(&zip, &[("level.dat", b"world"), (bad, b"evil")]);
        assert!(
            import_world(&zip, &saves, &mut progress).is_err(),
            "accepted {bad}"
        );
    }
    zip_fixture(&zip, &[("A/level.dat", b"a"), ("B/level.dat", b"b")]);
    assert!(import_world(&zip, &saves, &mut progress).is_err());
    // A directory conflicting with a previously extracted file must roll back the entire import.
    zip_fixture(
        &zip,
        &[
            ("level.dat", b"world"),
            ("region", b"file"),
            ("region/r.mca", b"region"),
        ],
    );
    assert!(import_world(&zip, &saves, &mut progress).is_err());
    assert_eq!(fs::read_dir(&saves).unwrap().count(), 1);
    assert!(!temp.path().join("escape").exists());
}

#[cfg(unix)]
#[test]
fn never_follows_symlinks_and_failed_copy_cleans_staging() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let saves = saves_dir(temp.path(), true).unwrap();
    let original = world(&saves, "world");
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"keep me").unwrap();
    symlink(&outside, original.join("region")).unwrap();
    symlink(&original, saves.join("linked-world")).unwrap();
    let mut noop = |_| {};
    let mut progress = Progress::new(&mut noop);
    // Listing succeeds without entering the linked region; linked worlds are omitted.
    assert_eq!(list_worlds(&saves).unwrap().len(), 1);
    assert!(world_dir(&saves, "linked-world").is_err());
    assert!(duplicate(&original, &saves, &mut progress).is_err());
    assert_eq!(fs::read_dir(&saves).unwrap().count(), 2);
    let zip = temp.path().join("world.zip");
    assert!(export_zip(&original, &zip, &mut progress).is_err());
    assert!(!zip.exists());
    delete_world(&original).unwrap();
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"keep me");
    let fake_instance = temp.path().join("fake-instance");
    fs::create_dir(&fake_instance).unwrap();
    symlink(&outside, fake_instance.join("saves")).unwrap();
    assert!(saves_dir(&fake_instance, true).is_err());
}

#[test]
fn bounded_level_decompression() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("level.dat");
    let mut gzip = GzEncoder::new(File::create(&file).unwrap(), Compression::fast());
    std::io::copy(&mut std::io::repeat(0).take(MAX_LEVEL_BYTES + 1), &mut gzip).unwrap();
    gzip.finish().unwrap();
    assert!(read_level(&file).unwrap_err().contains("16 MiB"));
}

#[test]
#[ignore = "manual performance measurement: 1000 worlds and streaming a 256 MiB region"]
fn world_performance_fixture() {
    let temp = tempfile::tempdir().unwrap();
    let saves = saves_dir(temp.path(), true).unwrap();
    for index in 0..1000 {
        world(&saves, &format!("World-{index}"));
    }
    let started = Instant::now();
    assert_eq!(list_worlds(&saves).unwrap().len(), 1000);
    let cold = started.elapsed();
    let started = Instant::now();
    assert_eq!(list_worlds(&saves).unwrap().len(), 1000);
    let warm = started.elapsed();
    let world = saves.join("World-0");
    fs::create_dir(world.join("region")).unwrap();
    File::create(world.join("region/r.0.0.mca"))
        .unwrap()
        .set_len(256 * 1024 * 1024)
        .unwrap();
    let mut events = 0;
    let mut callback = |_| events += 1;
    let mut progress = Progress::new(&mut callback);
    let zip = temp.path().join("large.zip");
    let started = Instant::now();
    export_zip(&world, &zip, &mut progress).unwrap();
    let export = started.elapsed();
    let started = Instant::now();
    let imported = import_world(&zip, &saves, &mut progress).unwrap();
    let import = started.elapsed();
    assert_eq!(
        fs::metadata(saves.join(imported).join("region/r.0.0.mca"))
            .unwrap()
            .len(),
        256 * 1024 * 1024
    );
    eprintln!(
        "1000 worlds: cold={cold:?}, cached={warm:?}; 256 MiB zero-filled region: export={export:?}, import={import:?}, progress events={events}"
    );
}
