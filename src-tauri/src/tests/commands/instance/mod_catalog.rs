use super::*;
use std::io::{Cursor, Write};
use std::time::Instant;
use zip::write::SimpleFileOptions;

fn fixture(dir: &Path, count: usize) {
    let mut png = Cursor::new(Vec::new());
    image::RgbaImage::from_fn(96, 96, |x, y| {
        image::Rgba([
            ((x * 73 + y * 29) % 256) as u8,
            ((x * 13 + y * 113) % 256) as u8,
            ((x * y * 7) % 256) as u8,
            255,
        ])
    })
    .write_to(&mut png, image::ImageFormat::Png)
    .unwrap();
    for i in 0..count {
        let file = std::fs::File::create(dir.join(format!("mod-{i:03}.jar"))).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("fabric.mod.json", options).unwrap();
        write!(zip, "{}", serde_json::json!({ "id": format!("mod{i}"), "name": format!("Mod {i}"), "version": "1.0", "icon": "pack.png", "authors": ["Fixture"] })).unwrap();
        zip.start_file("pack.png", options).unwrap();
        zip.write_all(png.get_ref()).unwrap();
        zip.finish().unwrap();
    }
}

#[tokio::test]
async fn catalog_performance_50_100_200_mods() {
    for count in [50, 100, 200] {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path(), count);
        let start = Instant::now();
        let cold = snapshot(dir.path(), false);
        let cold_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(cold.missing.len(), count);
        assert!(cold.items.iter().all(|item| item.icon.is_none()));
        let start = Instant::now();
        let saved = enrich_local(dir.path().to_path_buf(), cold.missing).await;
        let enrich_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(saved.len(), count);
        let start = Instant::now();
        let warm = snapshot(dir.path(), false);
        let warm_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert!(warm.missing.is_empty());
        assert!(
            warm.items
                .iter()
                .all(|item| item.version.as_deref() == Some("1.0") && item.icon.is_none())
        );
        let requests: Vec<_> = warm
            .items
            .iter()
            .take(16)
            .map(|item| IconRequest {
                filename: item.filename.clone(),
                revision: item.icon_revision.clone().unwrap(),
            })
            .collect();
        let start = Instant::now();
        let visible = icons(dir.path(), requests);
        let visible_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(visible.len(), 16);
        assert!(visible.iter().all(|item| item.icon.is_some()));
        let light_bytes = serde_json::to_vec(&warm.items).unwrap().len();
        let start = Instant::now();
        let eager = snapshot(dir.path(), true);
        let eager_ms = start.elapsed().as_secs_f64() * 1000.0;
        let eager_bytes = serde_json::to_vec(&eager.items).unwrap().len();
        assert_eq!(
            eager
                .items
                .iter()
                .filter(|item| item.icon.is_some())
                .count(),
            count
        );
        assert!(
            light_bytes * 5 < eager_bytes,
            "catalog should not transport every logo"
        );
        println!(
            "[mod catalog] n={count}: cold={cold_ms:.2}ms, metadata={enrich_ms:.2}ms, warm={warm_ms:.2}ms, icons(16)={visible_ms:.2}ms; catalog={light_bytes} bytes, eager={eager_bytes} bytes/{eager_ms:.2}ms"
        );
    }
}

#[tokio::test]
async fn toggling_reuses_metadata_without_hashing_the_pack_again() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), 3);
    let cold = snapshot(dir.path(), false);
    enrich_local(dir.path().to_path_buf(), cold.missing).await;
    std::fs::rename(
        dir.path().join("mod-001.jar"),
        dir.path().join("mod-001.jar.disabled"),
    )
    .unwrap();
    let toggled = snapshot(dir.path(), false);
    assert_eq!(toggled.missing.len(), 1);
    assert!(toggled.missing[0].reusable.is_some());
    let changed = toggled.items.iter().find(|item| !item.enabled).unwrap();
    assert_eq!(changed.name, "Mod 1");
    assert_eq!(changed.version.as_deref(), Some("1.0"));
    enrich_local(dir.path().to_path_buf(), toggled.missing).await;
    assert!(snapshot(dir.path(), false).missing.is_empty());
}

#[tokio::test]
async fn inventory_annotation_preserves_enriched_catalog_and_leaves_user_mods_unlocked() {
    let root = tempfile::tempdir().unwrap();
    let mods_dir = root.path().join("mods");
    std::fs::create_dir(&mods_dir).unwrap();
    fixture(&mods_dir, 2);

    let archive = root.path().join("fixture.mrpack");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("modrinth.index.json", options).unwrap();
    write!(
        zip,
        "{}",
        serde_json::json!({
            "game": "minecraft", "formatVersion": 1, "versionId": "1",
            "name": "Catalog fixture", "files": [], "dependencies": {"minecraft": "1.21"}
        })
    )
    .unwrap();
    zip.start_file("overrides/mods/mod-000.jar", options)
        .unwrap();
    zip.write_all(&std::fs::read(mods_dir.join("mod-000.jar")).unwrap())
        .unwrap();
    zip.finish().unwrap();
    crate::services::modpack::record_install(root.path(), &archive, "local", None, None)
        .await
        .unwrap();

    let cold = snapshot(&mods_dir, false);
    enrich_local(mods_dir.clone(), cold.missing).await;
    let mut warm = snapshot(&mods_dir, false).items;
    let before: Vec<_> = warm
        .iter()
        .map(|item| {
            (
                item.name.clone(),
                item.version.clone(),
                item.sha1.clone(),
                item.icon_revision.clone(),
            )
        })
        .collect();
    crate::services::modpack::annotate_mods(root.path(), "mods", &mut warm)
        .await
        .unwrap();
    for (item, (name, version, sha1, revision)) in warm.iter().zip(before) {
        assert_eq!(item.name, name);
        assert_eq!(item.version, version);
        assert_eq!(item.sha1, sha1);
        assert_eq!(item.icon_revision, revision);
        assert!(!item.pack_modified);
        if item.filename == "mod-000.jar" {
            assert_eq!(item.pack_name.as_deref(), Some("Catalog fixture"));
            assert!(item.pack_locked);
        } else {
            assert!(item.pack_name.is_none());
            assert!(!item.pack_locked);
        }
    }
}

#[tokio::test]
async fn deleted_or_replaced_files_cannot_receive_stale_metadata_or_icons() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), 2);
    let cold = snapshot(dir.path(), false);
    let requests = cold
        .items
        .iter()
        .map(|item| IconRequest {
            filename: item.filename.clone(),
            revision: item.icon_revision.clone().unwrap(),
        })
        .collect();
    std::fs::remove_file(dir.path().join("mod-000.jar")).unwrap();
    std::fs::write(dir.path().join("mod-001.jar"), b"replaced").unwrap();
    let saved = enrich_local(dir.path().to_path_buf(), cold.missing).await;
    assert!(saved.is_empty());
    assert!(icons(dir.path(), requests).is_empty());
    assert_eq!(snapshot(dir.path(), false).missing.len(), 1);
}

#[test]
fn enrichment_is_shared_per_instance_and_released_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let first = EnrichmentGuard::acquire(dir.path()).unwrap();
    assert!(EnrichmentGuard::acquire(dir.path()).is_none());
    drop(first);
    assert!(EnrichmentGuard::acquire(dir.path()).is_some());
}

#[tokio::test]
async fn oversized_icon_requests_return_a_localizable_error_before_reading_files() {
    let files = (0..25)
        .map(|i| IconRequest {
            filename: format!("mod-{i}.jar"),
            revision: "0".into(),
        })
        .collect();
    let error =
        match get_instance_mod_icons("550e8400-e29b-41d4-a716-446655440000".into(), files).await {
            Err(error) => error,
            Ok(_) => panic!("oversized request was accepted"),
        };
    let value: serde_json::Value = serde_json::from_str(&error).unwrap();
    assert_eq!(value["code"], "INST_MOD_ICONS");
    assert!(value["params"]["error"].is_string());
}
