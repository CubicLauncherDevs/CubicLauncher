use super::*;
use std::io::Write;

fn manifest() -> serde_json::Value {
    serde_json::json!({
        "manifestType": "minecraftModpack", "manifestVersion": 1,
        "name": "Audit", "version": "1", "author": "Audit", "files": [],
        "overrides": "overrides",
        "minecraft": {"version": "1.21", "modLoaders": [{"id": "vanilla", "primary": true}]}
    })
}

fn pack(root: &Path, manifest: serde_json::Value, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = root.join("pack.zip");
    let mut archive = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    archive.start_file("manifest.json", options).unwrap();
    archive
        .write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    for (name, contents) in entries {
        archive.start_file(*name, options).unwrap();
        archive.write_all(contents).unwrap();
    }
    archive.finish().unwrap();
    path
}

#[tokio::test]
async fn malicious_overrides_versions_and_zip_names_fail_without_touching_instance() {
    let temp = tempfile::tempdir().unwrap();
    let instance = temp.path().join("instance");
    std::fs::create_dir(&instance).unwrap();
    std::fs::write(instance.join("instance.cub"), b"original metadata").unwrap();
    let mut cases = Vec::new();
    for value in [
        "../../test.txt",
        "../outside/file.txt",
        "foo/../../outside/file.txt",
        "/outside",
        "C:\\outside",
        "",
        ".",
    ] {
        let mut m = manifest();
        m["overrides"] = value.into();
        cases.push((m, "overrides/ok.txt"));
    }
    for value in [
        "../../outside",
        "foo/../../outside",
        "/outside",
        "C:\\outside",
    ] {
        let mut m = manifest();
        m["minecraft"]["version"] = value.into();
        cases.push((m, "overrides/ok.txt"));
        let mut m = manifest();
        m["minecraft"]["modLoaders"][0]["id"] = format!("forge-{value}").into();
        cases.push((m, "overrides/ok.txt"));
    }
    for entry in [
        "../../test.txt",
        "../outside/file.txt",
        "foo/../../outside/file.txt",
        "overrides/../outside.txt",
        "/overrides/test.txt",
        "C:/overrides/test.txt",
        "overrides\\..\\outside.txt",
        "overrides/instance.cub",
        "overrides/INSTANCE.CUB",
        "overrides/instance.cub.",
        "overrides/instance.cub:stream",
        "overrides/upstream.json",
        "overrides/cubic-jar-cache/minecraft.jar",
    ] {
        cases.push((manifest(), entry));
    }
    for (m, entry) in cases {
        let archive = pack(temp.path(), m, &[(entry, b"malicious")]);
        assert!(parse_curseforge_modpack(&archive).is_err(), "{entry}");
        assert!(
            install_curseforge_modpack(&archive, &instance, &temp.path().join("shared"), None)
                .await
                .is_err()
        );
        assert_eq!(
            std::fs::read(instance.join("instance.cub")).unwrap(),
            b"original metadata"
        );
        assert_eq!(std::fs::read_dir(&instance).unwrap().count(), 1);
        assert!(!temp.path().join("shared").exists());
    }
}

#[tokio::test]
async fn failed_extraction_does_not_publish_partial_overrides() {
    let temp = tempfile::tempdir().unwrap();
    let instance = temp.path().join("instance");
    std::fs::create_dir(&instance).unwrap();
    let archive = pack(
        temp.path(),
        manifest(),
        &[
            ("overrides/first.txt", b"ok"),
            ("overrides/blocker", b"file"),
            ("overrides/blocker/child", b"bad"),
        ],
    );
    assert!(
        install_curseforge_modpack(&archive, &instance, &temp.path().join("shared"), None)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_dir(&instance).unwrap().count(), 0);
}

#[tokio::test]
async fn valid_pack_installs_portable_paths_and_icon_without_replacing_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let instance = temp.path().join("instance");
    std::fs::create_dir(&instance).unwrap();
    std::fs::write(instance.join("instance.cub"), b"original metadata").unwrap();
    let archive = pack(
        temp.path(),
        manifest(),
        &[
            ("overrides\\config\\settings.txt", b"settings"),
            ("icon.png", b"icon"),
        ],
    );
    install_curseforge_modpack(&archive, &instance, &temp.path().join("shared"), None)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(instance.join("config/settings.txt")).unwrap(),
        b"settings"
    );
    assert_eq!(std::fs::read(instance.join("icon.png")).unwrap(), b"icon");
    assert_eq!(
        std::fs::read(instance.join("instance.cub")).unwrap(),
        b"original metadata"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn installation_cannot_follow_a_destination_symlink() {
    let temp = tempfile::tempdir().unwrap();
    let instance = temp.path().join("instance");
    let outside = temp.path().join("outside");
    std::fs::create_dir(&instance).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("file.txt"), b"sentinel").unwrap();
    std::os::unix::fs::symlink(&outside, instance.join("config")).unwrap();
    let archive = pack(
        temp.path(),
        manifest(),
        &[("overrides/config/file.txt", b"bad")],
    );
    assert!(
        install_curseforge_modpack(&archive, &instance, &temp.path().join("shared"), None)
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read(outside.join("file.txt")).unwrap(),
        b"sentinel"
    );
}
