use super::*;

#[test]
fn icon_cache_detects_size_changes_even_if_modification_time_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod.jar");
    std::fs::write(&path, b"old").unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    ADDON_CACHE.lock().unwrap().insert(
        path.clone(),
        ((modified, 3), None, Some(Arc::new("stale-icon".into()))),
    );
    std::fs::write(&path, b"replacement").unwrap();
    std::fs::File::open(&path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    assert!(AddonManager::get_mod_icon(&path).is_none());
}

#[test]
fn cache_retains_a_200_mod_pack_and_evicts_by_use_not_file_age() {
    let mut cache = AddonCache::default();
    for i in 0..MAX_CACHE_ENTRIES {
        cache.insert(
            PathBuf::from(format!("mod-{i}")),
            (
                (SystemTime::UNIX_EPOCH, 0),
                None,
                Some(Arc::new("icon".into())),
            ),
        );
    }
    for i in 0..200 {
        assert!(cache.get(Path::new(&format!("mod-{i}"))).is_some());
    }
    cache.insert(
        PathBuf::from("new-mod"),
        ((SystemTime::UNIX_EPOCH, 0), None, None),
    );
    assert!(cache.get(Path::new("mod-0")).is_some());
    assert!(cache.get(Path::new("mod-200")).is_none());
    assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);
}

#[test]
fn cache_is_byte_bounded_and_replacing_an_entry_does_not_flush_half_the_cache() {
    let mut cache = AddonCache::default();
    let icon = Arc::new("x".repeat(1024 * 1024));
    for i in 0..100 {
        cache.insert(
            PathBuf::from(format!("mod-{i}")),
            ((SystemTime::UNIX_EPOCH, 0), None, Some(icon.clone())),
        );
    }
    assert!(cache.bytes <= MAX_CACHE_BYTES);
    assert!(cache.entries.len() < 33);
    let count = cache.entries.len();
    cache.insert(
        PathBuf::from("mod-99"),
        ((SystemTime::UNIX_EPOCH, 0), None, Some(icon)),
    );
    assert_eq!(cache.entries.len(), count);
    cache.insert(
        PathBuf::from("oversized"),
        (
            (SystemTime::UNIX_EPOCH, 0),
            None,
            Some(Arc::new("x".repeat(MAX_CACHE_BYTES + 1))),
        ),
    );
    assert!(cache.get(Path::new("oversized")).is_none());
    assert!(cache.get(Path::new("mod-99")).is_some());
}

#[test]
fn large_png_icons_are_downscaled_to_the_display_budget() {
    let source = image::RgbaImage::from_pixel(512, 512, image::Rgba([10, 20, 30, 255]));
    let mut png = Vec::new();
    source
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    let encoded = AddonManager::encode_icon("image/png", png);
    let (_, b64) = encoded.split_once(',').unwrap();
    let decoded = general_purpose::STANDARD.decode(b64).unwrap();
    let resized = image::load_from_memory(&decoded).unwrap();

    assert_eq!(resized.width(), MAX_ICON_DIMENSION);
    assert_eq!(resized.height(), MAX_ICON_DIMENSION);
}

#[test]
fn non_square_icons_keep_their_aspect_ratio_when_downscaled() {
    let source = image::RgbaImage::from_pixel(512, 128, image::Rgba([10, 20, 30, 255]));
    let mut png = Vec::new();
    source
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    let resized = AddonManager::downscale_png(&png).unwrap();
    let img = image::load_from_memory(&resized).unwrap();

    assert_eq!(img.width(), MAX_ICON_DIMENSION);
    assert_eq!(img.height(), MAX_ICON_DIMENSION / 4);
}

#[test]
fn icons_within_the_budget_keep_their_original_bytes() {
    let source = image::RgbaImage::from_pixel(64, 64, image::Rgba([10, 20, 30, 255]));
    let mut png = Vec::new();
    source
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    assert!(AddonManager::downscale_png(&png).is_none());
}

#[test]
fn corrupt_png_icons_fall_back_to_the_original_bytes() {
    let encoded = AddonManager::encode_icon("image/png", b"not a png".to_vec());
    let (_, b64) = encoded.split_once(',').unwrap();

    assert_eq!(general_purpose::STANDARD.decode(b64).unwrap(), b"not a png");
}

#[test]
fn pack_cache_fingerprint_changes_with_the_icon_pipeline_schema() {
    assert_ne!(pack_cache_fingerprint(0), 0);
    assert_ne!(pack_cache_fingerprint(42), 42);
}
