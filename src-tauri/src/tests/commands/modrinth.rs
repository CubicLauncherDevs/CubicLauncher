use super::*;
use std::io::Write;
use std::path::Path;

fn download(filename: &str) -> ModDownloadInfo {
    ModDownloadInfo {
        source: None,
        url: "https://example.invalid/file".into(),
        filename: filename.into(),
        project_id: None,
        version_id: None,
        sha1: None,
        headers: HashMap::new(),
    }
}

#[test]
fn downloaded_mod_source_retains_provider_and_exact_version_id() {
    let mut item = download("mod.jar");
    item.project_id = Some("123".into());
    item.version_id = Some("456".into());
    item.source = Some("curseforge".into());
    let source = download_source(&item);
    assert_eq!(source.source_str(), "curseforge");
    assert_eq!(source.project_id(), Some("123"));
    assert_eq!(source.version_id(), Some("456"));
    item.source = Some("modrinth".into());
    let source = download_source(&item);
    assert_eq!(source.source_str(), "modrinth");
    assert_eq!(source.version_id(), Some("456"));
}

async fn installed_pack(root: &Path) -> std::path::PathBuf {
    let archive = root.join("fixture.mrpack");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("modrinth.index.json", options).unwrap();
    zip.write_all(
        serde_json::json!({
            "game": "minecraft", "formatVersion": 1, "versionId": "1",
            "name": "Protected fixture", "files": [],
            "dependencies": {"minecraft": "1.21"}
        })
        .to_string()
        .as_bytes(),
    )
    .unwrap();
    for subdir in ["mods", "resourcepacks", "shaderpacks"] {
        zip.start_file(format!("overrides/{subdir}/locked.zip"), options)
            .unwrap();
        zip.write_all(b"pack content").unwrap();
    }
    zip.finish().unwrap();
    let instance = root.join("instance");
    cubrinth::mrpack::install_mrpack(&archive, &instance, &root.join("shared"), None)
        .await
        .unwrap();
    crate::services::modpack::record_install(&instance, &archive, "local", None, None)
        .await
        .unwrap();
    instance
}

#[tokio::test]
async fn market_preflight_rejects_locked_batch_members_and_disabled_aliases() {
    let root = tempfile::tempdir().unwrap();
    let instance = installed_pack(root.path()).await;
    for subdir in ["mods", "resourcepacks", "shaderpacks"] {
        for filename in ["locked.zip", "locked.zip.disabled"] {
            let batch = [download("unrelated.zip"), download(filename)];
            assert!(
                ensure_downloads_mutable(&instance, subdir, &batch)
                    .await
                    .is_err()
            );
            assert!(!instance.join(subdir).join("unrelated.zip").exists());
            assert_eq!(
                std::fs::read(instance.join(subdir).join("locked.zip")).unwrap(),
                b"pack content"
            );
        }
        ensure_downloads_mutable(&instance, subdir, &[download("unrelated.zip")])
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn market_preflight_validates_all_filenames_without_creating_directories() {
    let root = tempfile::tempdir().unwrap();
    let instance = root.path().join("instance");
    for subdir in ["mods", "resourcepacks", "shaderpacks"] {
        let batch = [download("valid.zip"), download("../invalid.zip")];
        assert!(
            ensure_downloads_mutable(&instance, subdir, &batch)
                .await
                .is_err()
        );
        assert!(!instance.exists());
    }
}
