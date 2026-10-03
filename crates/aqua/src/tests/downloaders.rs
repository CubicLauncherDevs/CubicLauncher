use super::*;
use crate::progress::DownloadProgress;
use std::time::Duration;
use tokio::sync::watch;

struct FinalizingBatch {
    items: Vec<DownloadItemSpec>,
    finalized: Arc<AtomicBool>,
}

impl DownloadBatch for FinalizingBatch {
    fn name(&self) -> String {
        "fallback-regression".into()
    }
    fn items(&self) -> &[DownloadItemSpec] {
        &self.items
    }
    fn finalize(
        &self,
        _: Option<ProgressSender>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AquaError>> + Send + '_>>
    {
        Box::pin(async {
            self.finalized.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
}

#[tokio::test]
async fn fallback_chain_only_finalizes_when_required_files_are_available() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for (success_path, required, expected_ok) in [
        ("/main.jar", true, true),
        ("/fallback.jar", true, true),
        ("/fallback-universal.jar", true, true),
        ("/never", true, false),
        ("/never", false, true),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = [0; 4096];
                let n = socket.read(&mut bytes).await.unwrap();
                let request = String::from_utf8_lossy(&bytes[..n]);
                let response = if request.split_whitespace().nth(1) == Some(success_path) {
                    "HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\njar"
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                };
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });
        let dir = std::env::temp_dir().join(format!("aqua-fallback-{}", uuid::Uuid::new_v4()));
        let mut item =
            DownloadItemSpec::new(format!("{base}/main.jar"), dir.join("lib.jar"), "library")
                .with_fallback_url(format!("{base}/fallback.jar"));
        item.required = required;
        let finalized = Arc::new(AtomicBool::new(false));
        let manager = DownloadManager::new(dir.clone());
        let handle = manager
            .prepare_batch(Box::new(FinalizingBatch {
                items: vec![item],
                finalized: finalized.clone(),
            }))
            .await
            .unwrap();
        let (tx, mut rx) = watch::channel(DownloadProgress::empty(1));
        let result = tokio::time::timeout(Duration::from_secs(10), handle.download_all(Some(tx)))
            .await
            .unwrap();
        assert_eq!(
            result.is_ok(),
            expected_ok,
            "{success_path}, required={required}"
        );
        assert_eq!(finalized.load(Ordering::SeqCst), expected_ok);
        tokio::time::timeout(Duration::from_secs(1), async {
            while rx.changed().await.is_ok() {}
        })
        .await
        .expect("progress sender leaked");
        server.abort();
        let _ = server.await;
        tokio::fs::remove_dir_all(dir).await.unwrap();
    }
}

#[tokio::test]
async fn required_item_failure_does_not_hang() {
    let temp_dir = std::env::temp_dir().join(format!("aqua-test-failure-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&temp_dir).await.unwrap();

    let manager = DownloadManager::new(temp_dir.clone());

    // Empty URL + required item forces `download_file_with_headers` to
    // fail immediately, which previously left the progress forwarder
    // looping forever and blocked callers waiting on the watch channel.
    let item = DownloadItemSpec::new("", temp_dir.join("missing.jar"), "missing-library")
        .with_hash("deadbeef");
    let batch = GenericBatch::new("test-batch", vec![item]);

    let handle = manager.prepare_batch(Box::new(batch)).await.unwrap();

    let (tx, _rx) = watch::channel(DownloadProgress::empty(handle.progress().1));

    let result = tokio::time::timeout(Duration::from_secs(5), handle.download_all(Some(tx))).await;

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    assert!(
        result.is_ok(),
        "download_all timed out (progress forwarder deadlock?)"
    );
    assert!(
        result.unwrap().is_err(),
        "download_all should have returned an error"
    );
}

#[tokio::test]
async fn failed_batch_drains_active_transfers_before_returning() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let slow_started = Arc::new(tokio::sync::Notify::new());
    let release_slow = Arc::new(tokio::sync::Notify::new());
    let failure_sent = Arc::new(tokio::sync::Notify::new());
    let attempts = Arc::new(AtomicUsize::new(0));
    let server = {
        let slow_started = slow_started.clone();
        let release_slow = release_slow.clone();
        let failure_sent = failure_sent.clone();
        tokio::spawn(async move {
            let mut clients = tokio::task::JoinSet::new();
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let slow_started = slow_started.clone();
                let release_slow = release_slow.clone();
                let failure_sent = failure_sent.clone();
                let attempts = attempts.clone();
                clients.spawn(async move {
                    let mut bytes = [0; 4096];
                    let n = socket.read(&mut bytes).await.unwrap();
                    if String::from_utf8_lossy(&bytes[..n]).contains("/slow.jar") {
                        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\n").await.unwrap();
                        slow_started.notify_one();
                        release_slow.notified().await;
                        let _ = socket.write_all(b"jar").await;
                    } else {
                        let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                        if attempt == 0 {
                            slow_started.notified().await;
                        }
                        socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
                        if attempt == 2 { failure_sent.notify_one(); }
                    }
                });
            }
        })
    };
    let dir = std::env::temp_dir().join(format!("aqua-drain-{}", uuid::Uuid::new_v4()));
    let manager = DownloadManager::new(dir.clone()).with_max_downloads(2);
    let handle = manager
        .prepare_batch(Box::new(GenericBatch::new(
            "drain",
            vec![
                DownloadItemSpec::new(format!("{base}/slow.jar"), dir.join("slow.jar"), "slow"),
                DownloadItemSpec::new(format!("{base}/fail.jar"), dir.join("fail.jar"), "fail"),
            ],
        )))
        .await
        .unwrap();
    let mut download = tokio::spawn(async move { handle.download_all(None).await });
    tokio::time::timeout(Duration::from_secs(5), failure_sent.notified())
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut download)
            .await
            .is_err(),
        "a failed batch returned while another transfer still owned files"
    );
    release_slow.notify_one();
    assert!(
        tokio::time::timeout(Duration::from_secs(5), download)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert_eq!(tokio::fs::read(dir.join("slow.jar")).await.unwrap(), b"jar");
    server.abort();
    let _ = server.await;
    tokio::fs::remove_dir_all(dir).await.unwrap();
}
