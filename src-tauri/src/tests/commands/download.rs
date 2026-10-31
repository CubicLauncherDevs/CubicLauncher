use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn manifest_json(release: &str) -> String {
    serde_json::json!({
        "latest": {"release": release, "snapshot": release},
        "versions": [{
            "id": release,
            "type": "release",
            "url": format!("https://example.com/{release}.json"),
            "time": "2026-09-15T11:29:26+00:00",
            "releaseTime": "2026-09-15T11:23:02+00:00"
        }]
    })
    .to_string()
}

fn seed_cache(path: &Path, timestamp: u64) {
    let manifest: MinecraftManifest = serde_json::from_str(&manifest_json("26.2")).unwrap();
    let mut repo = ablage::Repo::open(path);
    repo.put(
        "manifest",
        ablage::Entry {
            version: 1,
            fingerprint: timestamp,
            data: postcard::to_stdvec(&manifest).unwrap(),
        },
    );
    repo.flush().unwrap();
}

async fn serve_manifest(status: &str, body: String) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/manifest.json", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.windows(4).any(|part| part == b"\r\n\r\n") {
            let mut buffer = [0; 1024];
            let count = socket.read(&mut buffer).await.unwrap();
            assert!(count > 0 && request.len() < 16_384);
            request.extend_from_slice(&buffer[..count]);
        }
        socket.write_all(response.as_bytes()).await.unwrap();
    });
    (url, server)
}

#[tokio::test]
async fn fresh_manifest_uses_cache_without_requesting_mojang() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.crep");
    seed_cache(&path, manifest_timestamp());
    let (url, server) = serve_manifest("200 OK", manifest_json("26.3")).await;

    let versions = load_manifest_versions(&path, &url, false).await.unwrap();
    assert_eq!(versions[0].id, "26.2");
    assert!(!server.is_finished());
    server.abort();
}

#[tokio::test]
async fn expired_legacy_and_future_dated_manifests_refresh_to_26_3() {
    let now = manifest_timestamp();
    for timestamp in [0, now - MANIFEST_CACHE_TTL_SECS, now + 7200] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.crep");
        seed_cache(&path, timestamp);
        let (url, server) = serve_manifest("200 OK", manifest_json("26.3")).await;

        let versions = load_manifest_versions(&path, &url, false).await.unwrap();
        server.await.unwrap();
        assert_eq!(versions[0].id, "26.3");
        assert_eq!(versions[0].version_type, "release");

        let repo = ablage::Repo::open(&path);
        let entry = repo.get("manifest").unwrap();
        assert!((now..=manifest_timestamp()).contains(&entry.fingerprint));
        let manifest: MinecraftManifest = postcard::from_bytes(&entry.data).unwrap();
        assert_eq!(manifest.latest.release, "26.3");
        assert_eq!(manifest.versions[0].id, "26.3");
    }
}

#[tokio::test]
async fn manual_refresh_bypasses_fresh_cache() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.crep");
    seed_cache(&path, manifest_timestamp());
    let (url, server) = serve_manifest("200 OK", manifest_json("26.3")).await;

    let versions = load_manifest_versions(&path, &url, true).await.unwrap();
    server.await.unwrap();
    assert_eq!(versions[0].id, "26.3");
}

#[tokio::test]
async fn failed_refresh_preserves_cached_catalog_and_timestamp() {
    for force in [false, true] {
        for (status, body) in [
            ("503 Service Unavailable", manifest_json("26.3")),
            ("200 OK", "invalid json".to_string()),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("manifest.crep");
            seed_cache(&path, 0);
            let before = std::fs::read(&path).unwrap();
            let (url, server) = serve_manifest(status, body).await;

            let versions = load_manifest_versions(&path, &url, force).await.unwrap();
            server.await.unwrap();
            assert_eq!(versions[0].id, "26.2");
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }
}

#[tokio::test]
async fn offline_refresh_falls_back_to_cache() {
    // Keep the port reserved but never respond to HTTP: dropping accepted sockets
    // simulates a connection failure without contacting an external service.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/manifest.json", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.crep");
    seed_cache(&path, 0);
    let before = std::fs::read(&path).unwrap();

    for force in [false, true] {
        let versions = load_manifest_versions(&path, &url, force).await.unwrap();
        assert_eq!(versions[0].id, "26.2");
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    server.abort();
}

#[tokio::test]
async fn missing_or_corrupt_cache_downloads_manifest() {
    for corrupt in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.crep");
        if corrupt {
            std::fs::write(&path, b"corrupt cache").unwrap();
        }
        let (url, server) = serve_manifest("200 OK", manifest_json("26.3")).await;

        let versions = load_manifest_versions(&path, &url, false).await.unwrap();
        server.await.unwrap();
        assert_eq!(versions[0].id, "26.3");
        assert!(ablage::Repo::open(&path).has("manifest"));
    }
}

#[tokio::test]
async fn request_failure_without_cache_returns_error() {
    for force in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.crep");
        let (url, server) = serve_manifest("503 Service Unavailable", manifest_json("26.3")).await;

        assert!(load_manifest_versions(&path, &url, force).await.is_err());
        server.await.unwrap();
        assert!(!path.exists());
    }
}

fn catalog_entry(
    version_id: &str,
    game_version: &str,
    display_version: &str,
    stable: bool,
) -> (String, String, String, bool) {
    (
        version_id.to_string(),
        game_version.to_string(),
        display_version.to_string(),
        stable,
    )
}

#[test]
fn catalog_groups_and_sorts_versions_like_the_front() {
    let catalog = build_version_catalog(
        [
            catalog_entry("1.9-OptiFine_HD_U_I6", "1.9", "HD_U_I6", true),
            catalog_entry("1.20.1-OptiFine_HD_U_I5", "1.20.1", "HD_U_I5", true),
            catalog_entry("1.21-OptiFine_HD_U_J1_pre9", "1.21", "HD_U_J1_pre9", false),
            catalog_entry(
                "1.20.1-OptiFine_HD_U_I7_pre1",
                "1.20.1",
                "HD_U_I7_pre1",
                false,
            ),
            catalog_entry("1.20.1-OptiFine_HD_U_I6", "1.20.1", "HD_U_I6", true),
        ]
        .into_iter(),
    );

    let games: Vec<&str> = catalog
        .games
        .iter()
        .map(|group| group.game_version.as_str())
        .collect();
    assert_eq!(games, ["1.21", "1.20.1", "1.9"]);

    let stable_games: Vec<&str> = catalog
        .games
        .iter()
        .filter(|group| group.has_stable)
        .map(|group| group.game_version.as_str())
        .collect();
    assert_eq!(stable_games, ["1.20.1", "1.9"]);

    let versions: Vec<&str> = catalog.games[1]
        .versions
        .iter()
        .map(|version| version.display_version.as_str())
        .collect();
    assert_eq!(versions, ["HD_U_I6", "HD_U_I5", "HD_U_I7_pre1"]);
    assert_eq!(catalog.games[0].versions.len(), 1);
    assert!(!catalog.games[0].has_stable);
}

#[test]
fn catalog_version_cmp_matches_the_front_ordering() {
    let mut versions = [
        "66.1.1",
        "66.1.2",
        "66.1.0",
        "66.1.10",
        "12.18.0.1992-1.10.0",
        "12.18.0.1993-1.10.0",
        "HD_U_J1",
        "HD_U_J1_pre9",
    ];
    versions.sort_by(|a, b| catalog_version_cmp(a, b));

    assert_eq!(
        versions,
        [
            "66.1.10",
            "66.1.2",
            "66.1.1",
            "66.1.0",
            "12.18.0.1993-1.10.0",
            "12.18.0.1992-1.10.0",
            "HD_U_J1_pre9",
            "HD_U_J1",
        ]
    );
}
