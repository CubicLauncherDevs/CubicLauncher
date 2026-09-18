use super::*;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

struct Server {
    url: String,
    paths: Arc<StdMutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(routes: BTreeMap<&'static str, (u16, String)>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let paths = Arc::new(StdMutex::new(Vec::new()));
    let requests = paths.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let mut buf = [0; 1024];
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
            }
            let request = String::from_utf8_lossy(&request);
            let path = request.split_whitespace().nth(1).unwrap_or("").to_owned();
            requests.lock().unwrap().push(path.clone());
            // Give concurrently started WebView calls time to overlap.
            tokio::time::sleep(Duration::from_millis(15)).await;
            let (status, body) = routes
                .get(path.as_str())
                .cloned()
                .unwrap_or((404, "{}".into()));
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    Server { url, paths, task }
}

fn french(version: &str) -> String {
    serde_json::json!({"id":"fr-FR","version":version,"languages":{"fr":"Français"},"greeting":"Bonjour"}).to_string()
}

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!("cubic-i18n-test-{}", uuid::Uuid::new_v4()))
}

#[tokio::test]
async fn migrates_short_locale_filename() {
    let dir = temp_dir();
    fs::create_dir_all(&dir).await.unwrap();
    fs::write(dir.join("ja.json"), r#"{"id":"ja-JP","version":"1.0.0"}"#)
        .await
        .unwrap();

    let locales = read_stored_locales(&dir).await.unwrap();

    assert_eq!(locales.len(), 1);
    assert_eq!(locales[0].code, "ja");
    assert_eq!(locales[0].id, "ja-JP");
    assert!(dir.join("ja-JP.json").exists());
    assert!(!dir.join("ja.json").exists());
    fs::remove_dir_all(dir).await.unwrap();
}

#[tokio::test]
async fn saves_locale_with_full_id() {
    let dir = temp_dir();
    let data = r#"{"id":"fr-FR","version":"1.0.0"}"#.to_string();

    save_locale_to(&dir, data.clone()).await.unwrap();

    assert_eq!(
        fs::read_to_string(dir.join("fr-FR.json")).await.unwrap(),
        data
    );
    assert!(!dir.join("fr.json").exists());
    fs::remove_dir_all(dir).await.unwrap();
}

#[tokio::test]
async fn seeds_bundled_locales() {
    let dir = temp_dir();

    ensure_bundled_locales(&dir).await.unwrap();

    assert_eq!(
        fs::read_to_string(dir.join("es-ES.json")).await.unwrap(),
        BUNDLED_LOCALES[0].1
    );
    assert_eq!(
        fs::read_to_string(dir.join("en-US.json")).await.unwrap(),
        BUNDLED_LOCALES[1].1
    );
    fs::remove_dir_all(dir).await.unwrap();
}

#[tokio::test]
async fn selected_local_load_is_independent_of_other_dictionaries_and_catalog_is_metadata_only() {
    let dir = tempfile::tempdir().unwrap();
    let data = serde_json::json!({"id":"fr-FR","version":"1","languages":{"fr":"Français"},"large":"x".repeat(1024 * 1024)}).to_string();
    save_locale_to(dir.path(), data.clone()).await.unwrap();
    fs::write(dir.path().join("ja-JP.json"), "broken")
        .await
        .unwrap();
    let local = read_locale_from(dir.path(), "fr").await.unwrap().unwrap();
    assert_eq!(local.data, data);
    let metadata = read_stored_locales(dir.path()).await.unwrap();
    let json = serde_json::to_value(metadata).unwrap();
    assert_eq!(json[0]["label"], "Français");
    assert_eq!(json[0]["flag"], "🇫🇷");
    assert!(json[0].get("data").is_none());
    assert!(serde_json::to_vec(&json).unwrap().len() < 256);
    assert!(read_locale_from(dir.path(), "../fr").await.is_err());
}

#[tokio::test]
async fn single_locale_load_migrates_legacy_files_and_prefers_canonical_data() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("fr.json"), french("legacy"))
        .await
        .unwrap();
    assert_eq!(
        version_of(&read_locale_from(dir.path(), "fr").await.unwrap().unwrap()).as_deref(),
        Some("legacy")
    );
    assert!(!dir.path().join("fr.json").exists());
    fs::write(dir.path().join("fr.json"), french("old"))
        .await
        .unwrap();
    assert_eq!(
        version_of(&read_locale_from(dir.path(), "fr").await.unwrap().unwrap()).as_deref(),
        Some("legacy")
    );
}

#[tokio::test]
async fn concurrent_windows_share_one_version_check_and_dictionary_download() {
    let dir = tempfile::tempdir().unwrap();
    let service = LocaleService::default();
    service.save(dir.path(), french("1")).await.unwrap();
    let server = serve(BTreeMap::from([
        ("/fr/version", (200, r#"{"version":"2"}"#.into())),
        ("/fr", (200, french("2"))),
    ]))
    .await;
    let client = reqwest::Client::new();
    let (a, b) = tokio::join!(
        service.refresh(dir.path(), "fr", Some("1"), &client, &server.url),
        service.refresh(dir.path(), "fr", Some("1"), &client, &server.url),
    );
    assert_eq!(version_of(&a.unwrap().unwrap()).as_deref(), Some("2"));
    assert_eq!(version_of(&b.unwrap().unwrap()).as_deref(), Some("2"));
    assert!(
        service
            .refresh(dir.path(), "fr", Some("2"), &client, &server.url)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(*server.paths.lock().unwrap(), ["/fr/version", "/fr"]);
    assert_eq!(
        version_of(&service.load(dir.path(), "fr").await.unwrap().unwrap()).as_deref(),
        Some("2")
    );
}

#[tokio::test]
async fn unchanged_version_does_not_download_or_return_the_dictionary() {
    let dir = tempfile::tempdir().unwrap();
    let service = LocaleService::default();
    service.save(dir.path(), french("1")).await.unwrap();
    let server = serve(BTreeMap::from([(
        "/fr/version",
        (200, r#"{"version":"1"}"#.into()),
    )]))
    .await;
    let client = reqwest::Client::new();
    for _ in 0..3 {
        assert!(
            service
                .refresh(dir.path(), "fr", Some("1"), &client, &server.url)
                .await
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(*server.paths.lock().unwrap(), ["/fr/version"]);

    service.checks.lock().await.front_mut().unwrap().1 = Instant::now() - CHECK_TTL;
    assert!(
        service
            .refresh(dir.path(), "fr", Some("1"), &client, &server.url)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        *server.paths.lock().unwrap(),
        ["/fr/version", "/fr/version"]
    );
}

#[tokio::test]
async fn failed_and_invalid_refreshes_preserve_the_offline_copy_and_back_off() {
    for (status, body) in [
        (503, "{}".to_string()),
        (200, "broken".to_string()),
        (200, r#"{"id":"de-DE","version":"2"}"#.into()),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let service = LocaleService::default();
        let original = french("1");
        service.save(dir.path(), original.clone()).await.unwrap();
        let server = serve(BTreeMap::from([
            ("/fr/version", (200, r#"{"version":"2"}"#.into())),
            ("/fr", (status, body)),
        ]))
        .await;
        let client = reqwest::Client::new();
        assert!(
            service
                .refresh(dir.path(), "fr", Some("1"), &client, &server.url)
                .await
                .is_err()
        );
        assert!(
            service
                .refresh(dir.path(), "fr", Some("1"), &client, &server.url)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            service.load(dir.path(), "fr").await.unwrap().unwrap().data,
            original
        );
        assert_eq!(*server.paths.lock().unwrap(), ["/fr/version", "/fr"]);
    }
}

#[tokio::test]
async fn local_reads_do_not_wait_for_a_network_refresh_and_catalog_requests_are_shared() {
    let dir = tempfile::tempdir().unwrap();
    let service = LocaleService::default();
    service.save(dir.path(), french("1")).await.unwrap();
    let guard = service.checks.lock().await;
    let loaded = tokio::time::timeout(Duration::from_secs(1), service.load(dir.path(), "fr"))
        .await
        .unwrap()
        .unwrap();
    assert!(loaded.is_some());
    drop(guard);
    let server = serve(BTreeMap::from([("/locales", (200, r#"[{"code":"fr","id":"fr-FR","label":"Français","flag":"🇫🇷"},{"code":"de","id":"de-DE","label":"Deutsch","flag":"🇩🇪"}]"#.into()))])).await;
    let client = reqwest::Client::new();
    let (a, b) = tokio::join!(
        service.list(dir.path(), &client, &server.url),
        service.list(dir.path(), &client, &server.url)
    );
    assert_eq!(a.unwrap().len(), 4);
    let b = b.unwrap();
    assert!(b.iter().find(|l| l.code == "fr").unwrap().installed);
    assert!(!b.iter().find(|l| l.code == "de").unwrap().installed);
    assert_eq!(*server.paths.lock().unwrap(), ["/locales"]);
}

#[tokio::test]
async fn check_bookkeeping_is_bounded_and_catalog_falls_back_offline() {
    let dir = tempfile::tempdir().unwrap();
    let service = LocaleService::default();
    service.save(dir.path(), french("1")).await.unwrap();
    let server = serve(BTreeMap::new()).await;
    let client = reqwest::Client::new();
    let entries = service
        .list(dir.path(), &client, &server.url)
        .await
        .unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|entry| entry.installed));
    for i in 0..MAX_CHECKS + 3 {
        let code = format!(
            "q{}{}",
            char::from(b'a' + (i / 26) as u8),
            char::from(b'a' + (i % 26) as u8)
        );
        let _ = service
            .refresh(dir.path(), &code, None, &client, &server.url)
            .await;
    }
    assert_eq!(service.checks.lock().await.len(), MAX_CHECKS);
}
