use super::*;
use serde_json::{Value, json};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cubrinth-mrpack-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn archive(&self, manifest: &Value, entries: &[(&str, &str)]) -> PathBuf {
        let path = self.0.join("test.mrpack");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("modrinth.index.json", options).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();
        for (name, content) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn manifest() -> Value {
    json!({
        "game": "minecraft", "formatVersion": 1, "versionId": "test",
        "name": "Fixture", "files": [], "dependencies": {"minecraft": "1.21"}
    })
}

fn file(path: &str) -> Value {
    json!({
        "path": path, "hashes": {}, "fileSize": 1,
        "downloads": ["https://example.invalid/test.jar"]
    })
}

#[tokio::test]
async fn unsupported_format_is_rejected_by_parse_and_install() {
    let fixture = Fixture::new();
    for version in [0, 2, -1] {
        let mut manifest = manifest();
        manifest["formatVersion"] = json!(version);
        let path = fixture.archive(&manifest, &[]);
        assert!(matches!(parse_mrpack(&path), Err(MrpackError::Invalid(_))));
        let instance = fixture.0.join("instance");
        assert!(matches!(
            install_mrpack(&path, &instance, &fixture.0.join("shared"), None).await,
            Err(MrpackError::Invalid(_))
        ));
        assert!(!instance.exists());
    }
}

#[test]
fn file_sizes_are_not_truncated_at_four_gib() {
    let mut manifest = manifest();
    let mut large = file("mods/large.jar");
    large["fileSize"] = json!(u64::from(u32::MAX) + 42);
    manifest["files"] = json!([large]);
    let pack: PackFormat = serde_json::from_value(manifest).unwrap();
    assert_eq!(pack.files[0].file_size, u64::from(u32::MAX) + 42);
    assert!(download_items(&pack, Path::new("instance")).is_ok());
}

#[tokio::test]
async fn invalid_downloads_fail_before_any_writes_or_network() {
    let fixture = Fixture::new();
    let mut invalid = vec![
        file("../escape.jar"),
        file("/absolute.jar"),
        file("C:/escape.jar"),
        file("mods\\escape.jar"),
        file("mods/null\0.jar"),
        file(""),
        file("."),
    ];
    for downloads in [json!([]), json!([""]), json!(["   "])] {
        let mut missing_url = file("mods/no-url.jar");
        missing_url["downloads"] = downloads;
        invalid.push(missing_url);
    }
    for entry in invalid {
        let mut manifest = manifest();
        // A valid first item must not start downloading before the bad second item is checked.
        manifest["files"] = json!([file("mods/valid.jar"), entry]);
        let path = fixture.archive(&manifest, &[("overrides/config/test.txt", "unchanged")]);
        let instance = fixture.0.join("instance");
        let shared = fixture.0.join("shared");
        assert!(matches!(
            install_mrpack(&path, &instance, &shared, None).await,
            Err(MrpackError::Invalid(_))
        ));
        assert!(!instance.exists());
        assert!(!shared.exists());
    }
}

#[test]
fn client_unsupported_files_are_intentionally_skipped() {
    let mut manifest = manifest();
    let mut server = file("mods/server.jar");
    server["env"] = json!({"client": "unsupported"});
    server["downloads"] = json!([]);
    manifest["files"] = json!([server]);
    let pack = serde_json::from_value(manifest).unwrap();
    assert!(
        download_items(&pack, Path::new("instance"))
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn client_overrides_win_in_either_zip_order() {
    for client_first in [true, false] {
        let fixture = Fixture::new();
        let mut entries = vec![
            ("client-overrides/config/test.txt", "client"),
            ("overrides/config/test.txt", "common"),
        ];
        if !client_first {
            entries.reverse();
        }
        entries.push(("server-overrides/config/server.txt", "server"));
        let path = fixture.archive(&manifest(), &entries);
        let instance = fixture.0.join("instance");
        install_mrpack(&path, &instance, &fixture.0.join("shared"), None)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(instance.join("config/test.txt")).unwrap(),
            "client"
        );
        assert!(!instance.join("config/server.txt").exists());
    }
}

#[tokio::test]
async fn unsafe_override_aborts_before_extracting_valid_entries() {
    let fixture = Fixture::new();
    let path = fixture.archive(
        &manifest(),
        &[
            ("overrides/config/valid.txt", "valid"),
            ("overrides/../escape.txt", "invalid"),
        ],
    );
    let instance = fixture.0.join("instance");
    assert!(matches!(
        install_mrpack(&path, &instance, &fixture.0.join("shared"), None).await,
        Err(MrpackError::Invalid(_))
    ));
    assert!(!instance.exists());
}
