use super::*;
use std::fs::{File, FileTimes};
use std::time::Duration;

fn set_time(path: &Path, seconds: u64) {
    File::open(path)
        .unwrap()
        .set_times(
            FileTimes::new().set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)),
        )
        .unwrap();
}

#[test]
fn screenshot_order_and_latest_agree_without_decoding_images() {
    let dir = tempfile::tempdir().unwrap();
    for (name, seconds) in [
        ("old.png", 1),
        ("new.PNG", 5),
        ("tie.png", 5),
        ("ignored.txt", 9),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, "not an image: listing only needs metadata").unwrap();
        set_time(&path, seconds);
    }
    std::fs::create_dir(dir.path().join("directory.png")).unwrap();
    let paths = sorted_screenshots(dir.path());
    let names: Vec<_> = paths
        .iter()
        .map(|path| Path::new(path).file_name().unwrap().to_str().unwrap())
        .collect();
    assert_eq!(names, ["tie.png", "new.PNG", "old.png"]);
    assert_eq!(latest_screenshot(dir.path()), paths.first().cloned());
    assert!(sorted_screenshots(&dir.path().join("missing")).is_empty());
    assert_eq!(latest_screenshot(&dir.path().join("missing")), None);
}

#[test]
fn thumbnail_is_small_reused_and_invalidated_without_touching_original() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.png");
    image::RgbImage::from_pixel(1920, 1080, image::Rgb([30, 60, 90]))
        .save(&path)
        .unwrap();
    let original = std::fs::read(&path).unwrap();
    let cache = Mutex::new(ThumbnailCache::default());
    let first = cached_thumbnail(dir.path(), "capture.png", &cache).unwrap();
    let encoded = first.strip_prefix("data:image/png;base64,").unwrap();
    let preview = image::load_from_memory(&STANDARD.decode(encoded).unwrap()).unwrap();
    assert_eq!((preview.width(), preview.height()), (400, 225));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(Arc::ptr_eq(
        &first,
        &cached_thumbnail(dir.path(), "capture.png", &cache).unwrap()
    ));
    image::RgbImage::from_pixel(100, 200, image::Rgb([200, 0, 0]))
        .save(&path)
        .unwrap();
    let second = cached_thumbnail(dir.path(), "capture.png", &cache).unwrap();
    assert_ne!(first, second);
    assert_eq!(cache.lock().unwrap().entries.len(), 1);
    std::fs::remove_file(&path).unwrap();
    assert!(cached_thumbnail(dir.path(), "capture.png", &cache).is_none());
}

#[test]
fn thumbnail_cache_bounds_payload_and_keeps_recently_used_entries() {
    let mut cache = ThumbnailCache::default();
    let stamp = (SystemTime::UNIX_EPOCH, 1);
    for i in 0..THUMBNAIL_CACHE_ENTRIES {
        cache.insert(PathBuf::from(i.to_string()), stamp, Arc::from("small"));
    }
    assert!(cache.get(Path::new("0"), stamp).is_some());
    cache.insert(PathBuf::from("next"), stamp, Arc::from("small"));
    assert_eq!(cache.entries.len(), THUMBNAIL_CACHE_ENTRIES);
    assert!(cache.get(Path::new("1"), stamp).is_none());
    assert!(cache.get(Path::new("0"), stamp).is_some());
    for i in 0..10 {
        cache.insert(
            PathBuf::from(format!("big-{i}")),
            stamp,
            Arc::from("x".repeat(1024 * 1024)),
        );
        assert!(cache.bytes <= THUMBNAIL_CACHE_BYTES);
    }
    assert_eq!(cache.entries.len(), 4);
    assert!(
        cache
            .get(Path::new("big-9"), (SystemTime::UNIX_EPOCH, 2))
            .is_none()
    );
    assert_eq!(cache.bytes, 3 * 1024 * 1024);
}

#[test]
fn corrupt_or_outside_files_do_not_produce_thumbnails() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Mutex::new(ThumbnailCache::default());
    std::fs::write(dir.path().join("broken.png"), b"broken").unwrap();
    for filename in [
        "broken.png",
        "missing.png",
        "../outside.png",
        "/outside.png",
        "foo.txt",
    ] {
        assert!(cached_thumbnail(dir.path(), filename, &cache).is_none());
    }
    assert!(cache.lock().unwrap().entries.is_empty());
}

#[cfg(unix)]
#[test]
fn thumbnail_does_not_follow_file_symlinks_outside_screenshots() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("outside.png");
    image::RgbImage::new(10, 10).save(&path).unwrap();
    std::os::unix::fs::symlink(path, dir.path().join("link.png")).unwrap();
    assert!(thumbnail_path(dir.path(), "link.png").is_none());
}
