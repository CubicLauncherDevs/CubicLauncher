use super::*;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Clone)]
struct Route {
    status: u16,
    body: String,
    etag: Option<&'static str>,
}

impl Route {
    fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            etag: None,
        }
    }

    fn etagged(status: u16, body: impl Into<String>, etag: &'static str) -> Self {
        Self {
            status,
            body: body.into(),
            etag: Some(etag),
        }
    }
}

struct Server {
    url: String,
    paths: Arc<StdMutex<Vec<String>>>,
    if_none_match: Arc<StdMutex<Vec<Option<String>>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(routes: BTreeMap<&'static str, Route>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let paths = Arc::new(StdMutex::new(Vec::new()));
    let if_none_match = Arc::new(StdMutex::new(Vec::new()));
    let requests = paths.clone();
    let validators = if_none_match.clone();
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
            let sent = request.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.trim()
                    .eq_ignore_ascii_case("if-none-match")
                    .then(|| value.trim().to_owned())
            });
            validators.lock().unwrap().push(sent.clone());
            // Give concurrently started WebView calls time to overlap.
            tokio::time::sleep(Duration::from_millis(15)).await;
            let route = routes
                .get(path.as_str())
                .cloned()
                .unwrap_or_else(|| Route::new(404, "{}"));
            let validator = route
                .etag
                .map(|tag| format!("ETag: {tag}\r\n"))
                .unwrap_or_default();
            let response = if route.etag.is_some_and(|tag| sent.as_deref() == Some(tag)) {
                format!("HTTP/1.1 304 Not Modified\r\n{validator}Connection: close\r\n\r\n")
            } else {
                format!(
                    "HTTP/1.1 {} Test\r\nContent-Type: application/json\r\n{validator}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    route.status,
                    route.body.len(),
                    route.body
                )
            };
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    Server {
        url,
        paths,
        if_none_match,
        task,
    }
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
        ("/fr/version", Route::new(200, r#"{"version":"2"}"#)),
        ("/fr", Route::new(200, french("2"))),
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
        Route::new(200, r#"{"version":"1"}"#),
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
            ("/fr/version", Route::new(200, r#"{"version":"2"}"#)),
            ("/fr", Route::new(status, body)),
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
    let server = serve(BTreeMap::from([(
        "/locales",
        Route::new(
            200,
            r#"[{"code":"fr","id":"fr-FR","label":"Français","flag":"🇫🇷"},{"code":"de","id":"de-DE","label":"Deutsch","flag":"🇩🇪"}]"#,
        ),
    )]))
    .await;
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

#[tokio::test]
async fn etag_captured_then_reused_to_skip_unchanged_dictionary() {
    let dir = tempfile::tempdir().unwrap();
    let service = LocaleService::default();
    service.save(dir.path(), french("1")).await.unwrap();
    let server = serve(BTreeMap::from([
        ("/fr/version", Route::new(200, r#"{"version":"2"}"#)),
        ("/fr", Route::etagged(200, french("2"), "\"v1\"")),
    ]))
    .await;
    let client = reqwest::Client::new();

    let first = service
        .refresh(dir.path(), "fr", Some("1"), &client, &server.url)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(version_of(&first).as_deref(), Some("2"));
    assert_eq!(
        read_etag(dir.path(), "fr-FR").await.as_deref(),
        Some("\"v1\"")
    );
    assert_eq!(*server.paths.lock().unwrap(), ["/fr/version", "/fr"]);

    service.checks.lock().await.front_mut().unwrap().1 = Instant::now() - CHECK_TTL;
    assert!(
        service
            .refresh(dir.path(), "fr", Some("2"), &client, &server.url)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(*server.paths.lock().unwrap(), ["/fr/version", "/fr", "/fr"]);
    assert_eq!(
        server.if_none_match.lock().unwrap()[2].as_deref(),
        Some("\"v1\"")
    );
    assert_eq!(
        version_of(&service.load(dir.path(), "fr").await.unwrap().unwrap()).as_deref(),
        Some("2")
    );
}

#[tokio::test]
async fn changed_etag_forces_fresh_download() {
    let dir = tempfile::tempdir().unwrap();
    let service = LocaleService::default();
    service.save(dir.path(), french("1")).await.unwrap();
    write_etag(dir.path(), "fr-FR", "\"v1\"").await.unwrap();
    let server = serve(BTreeMap::from([(
        "/fr",
        Route::etagged(200, french("3"), "\"v2\""),
    )]))
    .await;
    let client = reqwest::Client::new();

    let updated = service
        .refresh(dir.path(), "fr", Some("1"), &client, &server.url)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(version_of(&updated).as_deref(), Some("3"));
    assert_eq!(*server.paths.lock().unwrap(), ["/fr"]);
    assert_eq!(
        read_etag(dir.path(), "fr-FR").await.as_deref(),
        Some("\"v2\"")
    );
}

#[tokio::test]
async fn save_locale_invalidates_stored_etag() {
    let dir = tempfile::tempdir().unwrap();
    write_etag(dir.path(), "fr-FR", "\"v1\"").await.unwrap();

    save_locale_to(dir.path(), french("2")).await.unwrap();

    assert!(read_etag(dir.path(), "fr-FR").await.is_none());
}

#[tokio::test]
async fn identical_body_refreshes_the_validator_without_rewriting() {
    let dir = tempfile::tempdir().unwrap();
    let service = LocaleService::default();
    service.save(dir.path(), french("2")).await.unwrap();
    write_etag(dir.path(), "fr-FR", "\"v1\"").await.unwrap();
    let server = serve(BTreeMap::from([(
        "/fr",
        Route::etagged(200, french("2"), "\"v2\""),
    )]))
    .await;
    let client = reqwest::Client::new();

    // Same body, new validator: nothing to return, but the etag must advance.
    assert!(
        service
            .refresh(dir.path(), "fr", Some("2"), &client, &server.url)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(*server.paths.lock().unwrap(), ["/fr"]);
    assert_eq!(
        read_etag(dir.path(), "fr-FR").await.as_deref(),
        Some("\"v2\"")
    );
    assert_eq!(
        service.load(dir.path(), "fr").await.unwrap().unwrap().data,
        french("2")
    );
}

#[tokio::test]
async fn empty_or_absent_etag_is_not_persisted() {
    for route in [
        Route::etagged(200, french("2"), ""),
        Route::new(200, french("2")),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let service = LocaleService::default();
        let server = serve(BTreeMap::from([("/fr", route)])).await;
        let client = reqwest::Client::new();

        let locale = service
            .refresh(dir.path(), "fr", None, &client, &server.url)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(version_of(&locale).as_deref(), Some("2"));
        assert!(read_etag(dir.path(), "fr-FR").await.is_none());
    }
}

#[tokio::test]
async fn catalog_deserializes_percentage_and_omits_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let service = LocaleService::default();
    let server = serve(BTreeMap::from([(
        "/locales",
        Route::new(
            200,
            r#"[{"code":"fr","id":"fr-FR","label":"Français","flag":"🇫🇷","percentage":100},{"code":"de","id":"de-DE","label":"Deutsch","flag":"🇩🇪","percentage":42.5},{"code":"es","id":"es-ES","label":"Español","flag":"🇪🇸"}]"#,
        ),
    )]))
    .await;
    let client = reqwest::Client::new();

    let entries = service
        .list(dir.path(), &client, &server.url)
        .await
        .unwrap();
    let find = |code: &str| entries.iter().find(|entry| entry.code == code).unwrap();
    assert_eq!(find("fr").percentage, Some(100.0));
    assert_eq!(find("de").percentage, Some(42.5));
    assert_eq!(find("es").percentage, None);

    let value = serde_json::to_value(&entries).unwrap();
    let es = value
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["code"] == "es")
        .unwrap();
    assert!(es.get("percentage").is_none());
}
