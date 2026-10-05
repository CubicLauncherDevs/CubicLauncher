use super::*;

fn fixture() -> (tempfile::TempDir, ExportInput, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("instance");
    fs::create_dir(&root).unwrap();
    let dest = dir.path().join("pack.mrpack");
    let input = ExportInput {
        root,
        name: "Example".into(),
        dependencies: dependencies(&GameVersion::from_version_id("fabric-loader-0.16.0-1.21.1"))
            .unwrap(),
        icon: None,
    };
    (dir, input, dest)
}

fn put(input: &ExportInput, path: &str, bytes: &[u8]) {
    let path = input.root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn request(paths: &[&str]) -> ExportRequest {
    ExportRequest {
        name: " My pack ".into(),
        version_id: " 2.3.4 ".into(),
        summary: " A selective pack ".into(),
        author: String::new(),
        selected_paths: paths.iter().map(|p| (*p).into()).collect(),
    }
}

#[tokio::test]
async fn author_roundtrips_through_export_import_and_export_preview() {
    let (_dir, input, dest) = fixture();
    let mut request = request(&[]);
    request.author = "  Lucía & Equipo Cubic  ".into();
    write_archive(snapshot(&input, request, &dest).unwrap()).unwrap();
    let mut archive = zip::ZipArchive::new(File::open(&dest).unwrap()).unwrap();
    let index: serde_json::Value =
        serde_json::from_reader(archive.by_name("modrinth.index.json").unwrap()).unwrap();
    assert_eq!(index["author"], "Lucía & Equipo Cubic");
    assert_eq!(index["summary"], "A selective pack");
    assert_eq!(index["formatVersion"], 1);
    let metadata = cubrinth::mrpack::parse_mrpack(&dest).unwrap();
    assert_eq!(metadata.author.as_deref(), Some("Lucía & Equipo Cubic"));
    super::super::modpack::record_install(&input.root, &dest, "local", None, None)
        .await
        .unwrap();
    assert_eq!(preview(&input).unwrap().author, "Lucía & Equipo Cubic");
}

#[test]
fn omitted_or_blank_author_remains_compatible_with_existing_requests_and_packs() {
    let (_dir, input, dest) = fixture();
    let legacy: ExportRequest = serde_json::from_value(serde_json::json!({
        "name": "Legacy", "versionId": "1", "selectedPaths": []
    }))
    .unwrap();
    assert!(legacy.author.is_empty());
    for author in ["", "   "] {
        let mut request = request(&[]);
        request.author = author.into();
        write_archive(snapshot(&input, request, &dest).unwrap()).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&dest).unwrap()).unwrap();
        let index: serde_json::Value =
            serde_json::from_reader(archive.by_name("modrinth.index.json").unwrap()).unwrap();
        assert!(index.get("author").is_none());
        assert!(
            cubrinth::mrpack::parse_mrpack(&dest)
                .unwrap()
                .author
                .is_none()
        );
    }
}

#[test]
fn author_length_is_checked_before_creating_an_export() {
    let (_dir, input, dest) = fixture();
    let mut request = request(&[]);
    request.author = "a".repeat(513);
    assert!(snapshot(&input, request, &dest).is_err());
    assert!(!dest.exists());
}

fn remote(file: &IndexFile, url: &str) -> RemoteFile {
    RemoteFile {
        hashes: file.hashes.clone(),
        size: file.file_size,
        url: url.into(),
    }
}

fn assert_no_temporary_files(parent: &Path) {
    assert!(fs::read_dir(parent).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".mrpack-")
    }));
}

#[test]
fn roundtrip_exact_download_hashes_and_selected_overrides() {
    let (dir, input, dest) = fixture();
    put(&input, "mods/exact.jar", b"abc");
    put(&input, "mods/offline.jar", b"unpublished");
    put(&input, "mods/excluded.jar", b"must not be exported");
    put(&input, "config/example.json", b"{\"enabled\":true}");
    put(&input, "saves/private/level.dat", b"private");
    let mut snapshot = snapshot(
        &input,
        request(&["mods/exact.jar", "mods/offline.jar", "config/example.json"]),
        &dest,
    )
    .unwrap();
    let exact = snapshot
        .files
        .iter()
        .find(|f| f.path == "mods/exact.jar")
        .unwrap();
    assert_eq!(
        exact.hashes["sha1"],
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    assert_eq!(
        exact.hashes["sha512"],
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    );
    let url = "https://cdn.modrinth.com/data/project/versions/version/exact.jar";
    let versions = BTreeMap::from([(
        exact.hashes["sha1"].clone(),
        RemoteVersion {
            files: vec![
                RemoteFile {
                    hashes: BTreeMap::new(),
                    url: "https://cdn.modrinth.com/data/wrong.jar".into(),
                    size: 3,
                },
                remote(exact, url),
            ],
        },
    )]);
    apply_resolved(&mut snapshot.files, &versions);
    // External modifications cannot desynchronize the index and archive bytes.
    put(&input, "mods/exact.jar", b"different version");
    put(&input, "config/example.json", b"edited after snapshot");
    assert_eq!(write_archive(snapshot).unwrap(), dest);
    assert_no_temporary_files(dir.path());
    let mut archive = zip::ZipArchive::new(File::open(&dest).unwrap()).unwrap();
    let names = archive
        .file_names()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        names,
        BTreeSet::from([
            "modrinth.index.json".into(),
            "overrides/mods/offline.jar".into(),
            "overrides/config/example.json".into(),
        ])
    );
    let index: serde_json::Value =
        serde_json::from_reader(archive.by_name("modrinth.index.json").unwrap()).unwrap();
    assert_eq!(index["formatVersion"], 1);
    assert_eq!(index["game"], "minecraft");
    assert_eq!(index["name"], "My pack");
    assert_eq!(index["versionId"], "2.3.4");
    assert_eq!(index["summary"], "A selective pack");
    assert_eq!(index["dependencies"]["minecraft"], "1.21.1");
    assert_eq!(index["dependencies"]["fabric-loader"], "0.16.0");
    assert_eq!(index["files"].as_array().unwrap().len(), 1);
    assert_eq!(index["files"][0]["path"], "mods/exact.jar");
    assert_eq!(index["files"][0]["fileSize"].as_u64(), Some(3));
    assert_eq!(index["files"][0]["downloads"][0], url);
    assert_eq!(
        index["files"][0]["hashes"]["sha1"],
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    let mut config = String::new();
    archive
        .by_name("overrides/config/example.json")
        .unwrap()
        .read_to_string(&mut config)
        .unwrap();
    assert_eq!(config, "{\"enabled\":true}");
    let restored = dir.path().join("restored");
    archive.extract(&restored).unwrap();
    assert_eq!(
        fs::read(restored.join("overrides/mods/offline.jar")).unwrap(),
        b"unpublished"
    );
}

#[test]
fn defaults_sizes_and_internal_metadata_are_filtered_at_every_depth() {
    let (_dir, mut input, _) = fixture();
    for path in [
        "mods/a.jar",
        "config/nested/a.json",
        "scripts/a.zs",
        "kubejs/a.js",
        "defaultconfigs/a.toml",
        "resourcepacks/a.zip",
        "shaderpacks/a.zip",
    ] {
        put(&input, path, b"abc");
    }
    for path in [
        "saves/world/level.dat",
        "logs/latest.log",
        "screenshots/a.png",
        "options.txt",
        "optionsof.txt",
        "servers.dat",
    ] {
        put(&input, path, b"abcd");
    }
    for path in [
        "instance.cub",
        "upstream.json",
        "modpack.cub.json",
        ".modpack-backup/secret",
        "mods/cache.crep",
        "config/nested/.modpack-stage/a",
        "icon.png",
        "config.json",
        "cubic-jar/a.jar",
        "cubic-jar-cache/minecraft.jar",
        "custom-icon.png",
    ] {
        put(&input, path, b"internal");
    }
    input.icon = Some(input.root.join("custom-icon.png"));
    let preview = preview(&input).unwrap();
    let files = preview
        .entries
        .iter()
        .filter(|e| !e.is_dir)
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 13);
    assert_eq!(files.iter().filter(|e| e.default_selected).count(), 7);
    assert_eq!(files.iter().filter(|e| !e.default_selected).count(), 6);
    assert!(
        files
            .iter()
            .filter(|e| e.default_selected)
            .all(|e| e.size == 3)
    );
    assert_eq!(
        preview
            .entries
            .iter()
            .find(|e| e.path == "config")
            .unwrap()
            .size,
        3
    );
    assert_eq!(
        preview
            .entries
            .iter()
            .find(|e| e.path == "config/nested")
            .unwrap()
            .size,
        3
    );
    assert!(preview.entries.iter().all(|e| !e.path.contains('\\')));
}

#[test]
fn selection_rejects_traversal_absolute_paths_directories_and_internal_files() {
    let (dir, input, dest) = fixture();
    put(&input, "mods/good.jar", b"ok");
    put(&input, "instance.cub", b"internal");
    for path in [
        "../outside",
        "/etc/passwd",
        "C:/private.txt",
        "mods\\good.jar",
        "mods/../instance.cub",
        "mods//good.jar",
        "mods/./good.jar",
        "mods/good.jar/",
        "mods",
        "missing",
        "instance.cub",
        "mods/\0.jar",
    ] {
        assert!(snapshot(&input, request(&[path]), &dest).is_err(), "{path}");
        assert!(!dest.exists());
        assert_no_temporary_files(dir.path());
    }
}

#[test]
fn all_configuration_stays_in_overrides_even_when_hash_is_resolved() {
    let (_dir, input, dest) = fixture();
    for path in [
        "config/a.jar",
        "scripts/a.jar",
        "kubejs/a.jar",
        "defaultconfigs/a.jar",
        "mods/local.jar",
    ] {
        put(&input, path, b"abc");
    }
    let paths = [
        "config/a.jar",
        "scripts/a.jar",
        "kubejs/a.jar",
        "defaultconfigs/a.jar",
        "mods/local.jar",
    ];
    let mut snapshot = snapshot(&input, request(&paths), &dest).unwrap();
    let file = &snapshot.files[0];
    let versions = BTreeMap::from([(
        file.hashes["sha1"].clone(),
        RemoteVersion {
            files: vec![remote(
                file,
                "https://cdn.modrinth.com/data/p/versions/v/a.jar",
            )],
        },
    )]);
    apply_resolved(&mut snapshot.files, &versions);
    assert_eq!(
        snapshot
            .files
            .iter()
            .filter(|f| !f.downloads.is_empty())
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>(),
        ["mods/local.jar"]
    );
    write_archive(snapshot).unwrap();
    let archive = zip::ZipArchive::new(File::open(dest).unwrap()).unwrap();
    for path in &paths[..4] {
        assert!(
            archive
                .file_names()
                .any(|name| name == format!("overrides/{path}"))
        );
    }
}

#[test]
fn mismatched_sha512_size_or_download_url_fall_back_to_overrides() {
    let (_dir, input, dest) = fixture();
    put(&input, "mods/a.jar", b"abc");
    for variant in 0..4 {
        let mut snapshot = snapshot(&input, request(&["mods/a.jar"]), &dest).unwrap();
        let file = &snapshot.files[0];
        let mut candidate = remote(file, "https://cdn.modrinth.com/data/p/versions/v/a.jar");
        match variant {
            0 => {
                candidate.hashes.insert("sha512".into(), "wrong".into());
            }
            1 => candidate.size += 1,
            2 => candidate.url = "https://example.org/landing-page.jar".into(),
            _ => {
                candidate.hashes.insert("sha1".into(), "wrong".into());
            }
        }
        let versions = BTreeMap::from([(
            file.hashes["sha1"].clone(),
            RemoteVersion {
                files: vec![candidate],
            },
        )]);
        apply_resolved(&mut snapshot.files, &versions);
        assert!(snapshot.files[0].downloads.is_empty());
    }
}

#[test]
fn download_urls_require_approved_direct_https_locations() {
    for url in [
        "https://cdn.modrinth.com/data/p/versions/v/a.jar",
        "https://github.com/owner/repo/releases/download/v1/a.jar",
        "https://raw.githubusercontent.com/owner/repo/hash/a.zip",
        "https://gitlab.com/owner/repo/-/releases/v1/downloads/a.jar",
    ] {
        assert!(valid_download(url), "{url}");
    }
    for url in [
        "file:///etc/passwd",
        "http://cdn.modrinth.com/data/a.jar",
        "https://cdn.modrinth.com.evil.test/data/a.jar",
        "https://user:pass@cdn.modrinth.com/data/a.jar",
        "https://cdn.modrinth.com:8443/data/a.jar",
        "https://cdn.modrinth.com/data/a.jar#fragment",
        "https://github.com/owner/repo/blob/main/a.jar",
        "https://github.com/owner/repo",
        "https://modrinth.com/mod/example",
        "https://127.0.0.1/a.jar",
        "https://cdn.modrinth.com/data/a.jar?token=secret",
    ] {
        assert!(!valid_download(url), "{url}");
    }
}

#[test]
fn partial_zip_failure_cleans_temps_and_preserves_existing_destination() {
    let (dir, input, dest) = fixture();
    put(&input, "mods/a.jar", b"abc");
    fs::write(&dest, b"previous good archive").unwrap();
    let snapshot = snapshot(&input, request(&["mods/a.jar"]), &dest).unwrap();
    // Fail after the writer created a ZIP and wrote the index, not before I/O.
    fs::remove_file(snapshot.staging.path().join("0")).unwrap();
    assert!(write_archive(snapshot).is_err());
    assert_eq!(fs::read(&dest).unwrap(), b"previous good archive");
    assert_no_temporary_files(dir.path());
}

#[test]
fn failed_atomic_persist_cleans_temps() {
    let (dir, input, dest) = fixture();
    let snapshot = snapshot(&input, request(&[]), &dest).unwrap();
    fs::create_dir(&dest).unwrap();
    assert!(write_archive(snapshot).is_err());
    assert!(dest.is_dir());
    assert_no_temporary_files(dir.path());
}

#[test]
fn metadata_only_export_and_duplicate_selections_are_valid() {
    let (dir, input, dest) = fixture();
    let empty_snapshot = snapshot(&input, request(&[]), &dest).unwrap();
    write_archive(empty_snapshot).unwrap();
    let archive = zip::ZipArchive::new(File::open(&dest).unwrap()).unwrap();
    assert_eq!(archive.len(), 1);
    drop(archive);
    put(&input, "mods/a.jar", b"abc");
    let snapshot = snapshot(&input, request(&["mods/a.jar", "mods/a.jar"]), &dest).unwrap();
    assert_eq!(snapshot.files.len(), 1);
    write_archive(snapshot).unwrap();
    assert_no_temporary_files(dir.path());
}

#[test]
fn destination_and_metadata_validation_prevent_invalid_exports() {
    let (_dir, input, dest) = fixture();
    for path in [
        input.root.join("pack.mrpack"),
        PathBuf::from("relative.mrpack"),
        dest.with_extension("zip"),
        dest.parent().unwrap().join("../pack.mrpack"),
    ] {
        assert!(snapshot(&input, request(&[]), &path).is_err());
    }
    let mut invalid = request(&[]);
    invalid.name = " ".into();
    assert!(snapshot(&input, invalid, &dest).is_err());
    let mut invalid = request(&[]);
    invalid.version_id.clear();
    assert!(snapshot(&input, invalid, &dest).is_err());
}

#[test]
fn dependencies_cover_supported_loaders_and_reject_optifine() {
    for (id, key, expected) in [
        ("1.20.1-forge-47.2.0", "forge", "47.2.0"),
        ("1.21.1-neoforge-21.1.0", "neoforge", "21.1.0"),
        ("fabric-loader-0.16.0-1.21.1", "fabric-loader", "0.16.0"),
        ("quilt-loader-0.25.0-1.21.1", "quilt-loader", "0.25.0"),
    ] {
        let deps = dependencies(&GameVersion::from_version_id(id)).unwrap();
        assert_eq!(deps[key], expected);
        assert_eq!(deps.len(), 2);
    }
    assert_eq!(
        dependencies(&GameVersion::from_version_id("1.21.1")).unwrap(),
        BTreeMap::from([("minecraft".into(), "1.21.1".into())])
    );
    assert!(
        dependencies(&GameVersion::from_version_id("1.20.1-OptiFine_HD_U_I6"))
            .unwrap_err()
            .contains("OptiFine")
    );
    assert!(
        dependencies(&GameVersion {
            mc_version: "1.21.1".into(),
            loader: Loader::Fabric(String::new())
        })
        .is_err()
    );
}

#[test]
fn custom_minecraft_jars_and_jar_mods_are_not_representable() {
    use crate::services::minecraft_jar::{JarFile, JarMod, MinecraftJarConfig};
    let jar = JarFile {
        file: "custom.jar".into(),
        name: "Custom Minecraft".into(),
    };
    let mut config = MinecraftJarConfig::default();
    assert!(validate_minecraft_jar(&config).is_ok());
    config.replacement = Some(jar.clone());
    assert!(validate_minecraft_jar(&config).unwrap_err().contains("JAR"));
    config.replacement = None;
    config.mods.push(JarMod {
        archive: jar,
        enabled: true,
    });
    assert!(validate_minecraft_jar(&config).is_err());
    config.mods[0].enabled = false;
    assert!(validate_minecraft_jar(&config).is_err());
}

#[test]
fn snapshot_failure_after_copying_a_file_cleans_staging() {
    let (dir, input, dest) = fixture();
    put(&input, "mods/a.jar", b"abc");
    assert!(snapshot(&input, request(&["mods/a.jar", "zzz-missing"]), &dest).is_err());
    assert!(!dest.exists());
    assert_no_temporary_files(dir.path());
}

#[test]
fn index_sizes_are_u64_not_truncated_to_u32() {
    let file = IndexFile {
        path: "mods/large.jar".into(),
        hashes: BTreeMap::new(),
        downloads: Vec::new(),
        file_size: u64::from(u32::MAX) + 10,
    };
    assert_eq!(
        serde_json::to_value(file).unwrap()["fileSize"].as_u64(),
        Some(u64::from(u32::MAX) + 10)
    );
}

#[cfg(unix)]
#[test]
fn symlink_files_directories_roots_and_post_preview_swaps_are_rejected() {
    use std::os::unix::fs::symlink;
    let (dir, input, dest) = fixture();
    put(&input, "mods/good.jar", b"abc");
    let outside = dir.path().join("secret");
    fs::write(&outside, b"secret").unwrap();
    symlink(&outside, input.root.join("mods/link.jar")).unwrap();
    symlink(dir.path(), input.root.join("external")).unwrap();
    symlink(input.root.join("mods"), input.root.join("loop")).unwrap();
    let entries = preview(&input).unwrap().entries;
    assert_eq!(entries.len(), 2);
    for path in ["mods/link.jar", "external/secret", "loop/good.jar"] {
        assert!(snapshot(&input, request(&[path]), &dest).is_err());
    }
    fs::remove_file(input.root.join("mods/good.jar")).unwrap();
    symlink(&outside, input.root.join("mods/good.jar")).unwrap();
    assert!(snapshot(&input, request(&["mods/good.jar"]), &dest).is_err());
    symlink(&outside, &dest).unwrap();
    assert!(snapshot(&input, request(&[]), &dest).is_err());
    let linked_root = dir.path().join("linked-instance");
    symlink(&input.root, &linked_root).unwrap();
    let linked_input = ExportInput {
        root: linked_root,
        ..input
    };
    assert!(preview(&linked_input).is_err());
    assert_no_temporary_files(dir.path());
}
