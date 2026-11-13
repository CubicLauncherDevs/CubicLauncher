use super::*;
use std::fs::File;
use std::io::Write as _;
use std::time::Duration;

fn uri(value: &str) -> Uri {
    value.parse().unwrap()
}

fn set_modified(path: &Path, seconds: u64) {
    File::open(path)
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)),
        )
        .unwrap();
}

#[test]
fn parses_only_https_icons_from_the_known_cdns() {
    let allowed =
        uri("cubic-market-icon://localhost/https%3A%2F%2Fcdn.modrinth.com%2Fdata%2Fx%2Fabc.png");
    assert_eq!(
        parse(&allowed).as_deref(),
        Some("https://cdn.modrinth.com/data/x/abc.png")
    );

    for rejected in [
        // Host fuera de la lista: el protocolo no debe ser un proxy.
        "cubic-market-icon://localhost/https%3A%2F%2Fevil.example%2Fa.png",
        // Solo HTTPS.
        "cubic-market-icon://localhost/http%3A%2F%2Fcdn.modrinth.com%2Fa.png",
        // Sin URL.
        "cubic-market-icon://localhost/",
        // Varios segmentos: la URL no viaja como un único path.
        "cubic-market-icon://localhost/https%3A%2F%2Fcdn.modrinth.com/a/b.png",
        // Basura.
        "cubic-market-icon://localhost/not-a-url",
    ] {
        assert!(parse(&uri(rejected)).is_none(), "{rejected}");
    }
}

#[test]
fn cache_keys_are_stable_and_unique_per_url() {
    let first = cache_path("https://cdn.modrinth.com/data/x/abc.png").unwrap();
    assert_eq!(
        first,
        cache_path("https://cdn.modrinth.com/data/x/abc.png").unwrap()
    );
    assert_ne!(
        first,
        cache_path("https://cdn.modrinth.com/data/x/def.png").unwrap()
    );
    assert_eq!(
        first.parent().unwrap().file_name().unwrap(),
        std::ffi::OsStr::new("market-icons")
    );
    assert_eq!(first.file_name().unwrap().to_str().unwrap().len(), 32);
}

#[test]
fn detects_supported_image_formats_and_rejects_pages() {
    assert_eq!(image_mime(b"\x89PNG\r\n\x1a\nbody"), Some("image/png"));
    assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
    assert_eq!(image_mime(b"GIF89a"), Some("image/gif"));
    assert_eq!(
        image_mime(b"RIFF\x10\x00\x00\x00WEBPVP8 "),
        Some("image/webp")
    );
    assert_eq!(image_mime(b"<html>404</html>"), None);
    assert_eq!(image_mime(b""), None);
}

#[test]
fn serves_images_and_hides_non_image_payloads() {
    let response = serve(b"\x89PNG\r\n\x1a\nbody".to_vec());
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/png"
    );

    let response = serve(b"<html>error</html>".to_vec());
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn store_is_atomic_and_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("icon");
    store(&path, b"\x89PNG\r\n\x1a\nbody").await.unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"\x89PNG\r\n\x1a\nbody");
    assert!(!path.with_extension("tmp").exists());
}

#[tokio::test]
async fn prune_drops_the_oldest_entries_until_the_budget_fits() {
    let dir = tempfile::tempdir().unwrap();
    let half = MAX_CACHE_BYTES / 2 + 1;
    let mut paths = Vec::new();
    for (index, seconds) in [(0, 1), (1, 5)] {
        let path = dir.path().join(format!("icon-{index}"));
        File::create(&path).unwrap().set_len(half).unwrap();
        set_modified(&path, seconds);
        paths.push(path);
    }

    prune(dir.path()).await;

    assert!(!paths[0].exists(), "el más antiguo debe descartarse");
    assert!(paths[1].exists(), "el más reciente debe conservarse");
}

#[tokio::test]
async fn prune_also_bounds_the_entry_count() {
    let dir = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    for index in 0..=MAX_CACHE_ENTRIES {
        let path = dir.path().join(format!("icon-{index}"));
        std::fs::write(&path, b"x").unwrap();
        set_modified(&path, index as u64 + 1);
        paths.push(path);
    }

    prune(dir.path()).await;

    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        MAX_CACHE_ENTRIES
    );
    assert!(!paths[0].exists(), "el más antiguo debe descartarse");
    assert!(paths[MAX_CACHE_ENTRIES].exists());
}

#[tokio::test]
async fn cached_bytes_survive_a_second_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("icon");
    let mut file = File::create(&path).unwrap();
    file.write_all(b"payload").unwrap();

    assert_eq!(read_cached(&path).await, Some(b"payload".to_vec()));
    assert_eq!(read_cached(&path).await, Some(b"payload".to_vec()));
    assert_eq!(read_cached(&dir.path().join("missing")).await, None);
}
