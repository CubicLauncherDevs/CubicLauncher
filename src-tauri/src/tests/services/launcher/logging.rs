use super::*;

fn entry(id: u64) -> LogEntryEvent {
    LogEntryEvent {
        id,
        line: Arc::from(format!("line {id}")),
        stream: "stdout",
        level: LogLevel::Info,
        timestamp: id,
    }
}

#[test]
fn preview_only_retains_the_latest_line_without_console_batches() {
    let mut pending = PendingLogs::default();
    for id in 0..10_000 {
        pending.push(entry(id), false, true);
    }
    assert!(pending.lines.is_empty());
    assert_eq!(pending.preview.as_ref().unwrap().id, 9_999);
    pending.push(entry(10_000), false, false);
    assert!(pending.is_empty());
}

#[test]
fn console_batches_and_preview_have_independent_lifetimes() {
    let mut pending = PendingLogs::default();
    for id in 0..64 {
        pending.push(entry(id), true, true);
    }
    assert_eq!(pending.lines.len(), 64);
    assert_eq!(pending.preview.as_ref().unwrap().id, 63);
    flush_log_batch(None, "instance", "log-instance", &mut pending.lines);
    assert!(pending.lines.is_empty());
    assert_eq!(pending.preview.as_ref().unwrap().id, 63);
    flush_log_preview(None, "instance", &mut pending.preview);
    assert!(pending.is_empty());
    pending.push(entry(64), true, false);
    assert_eq!(pending.lines.len(), 1);
    assert!(pending.preview.is_none());
}

#[tokio::test]
async fn history_and_live_events_share_ids_and_timestamps() {
    let ring = LogRing::new();
    let (id, timestamp) = ring
        .push(Arc::from("client ready"), LogLevel::Info, 0)
        .await;
    let history = ring.snapshot(None).await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, id);
    assert_eq!(history[0].timestamp, timestamp);
}

#[tokio::test]
async fn background_logs_detect_early_readiness_and_drain_both_streams() {
    let id: Arc<str> = Arc::from(uuid::Uuid::new_v4().to_string());
    let (stdout_tx, stdout_rx) = broadcast::channel(16);
    let (stderr_tx, stderr_rx) = broadcast::channel(16);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    // Buffered before the forwarding task even starts, just like a fast JVM.
    stderr_tx
        .send("\x1b[32m[INFO]: Setting user: Player\x1b[0m".into())
        .unwrap();
    stdout_tx.send("access_token=secret-value".into()).unwrap();
    let task = spawn_io_forwarding(
        None,
        id.clone(),
        stdout_rx,
        stderr_rx,
        Some(ready_tx),
        Arc::new(AtomicBool::new(false)),
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        wait_for_game_readiness(ready_rx, None),
    )
    .await
    .unwrap();
    drop(stderr_tx);
    stdout_tx.send("last stdout line".into()).unwrap();
    drop(stdout_tx);
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();

    let history = get_log_history(&id, None).await;
    assert_eq!(history.len(), 3);
    assert!(
        history
            .iter()
            .any(|line| line.stream == "stderr" && line.text.contains("Setting user"))
    );
    assert!(
        history
            .iter()
            .any(|line| line.text.as_ref() == "last stdout line")
    );
    assert!(
        history
            .iter()
            .all(|line| !line.text.contains("secret-value") && !line.text.contains('\x1b'))
    );
    remove_log_ring(&id);
}

#[tokio::test]
async fn forwarding_flags_a_client_initiated_shutdown() {
    let id: Arc<str> = Arc::from(uuid::Uuid::new_v4().to_string());
    let (stdout_tx, stdout_rx) = broadcast::channel(16);
    let (stderr_tx, stderr_rx) = broadcast::channel(16);
    let clean_shutdown = Arc::new(AtomicBool::new(false));
    let task = spawn_io_forwarding(
        None,
        id.clone(),
        stdout_rx,
        stderr_rx,
        None,
        clean_shutdown.clone(),
    );
    assert!(!clean_shutdown.load(Ordering::Acquire));
    stdout_tx
        .send("[21:36:41] [Render thread/INFO]: Stopping!".into())
        .unwrap();
    drop(stdout_tx);
    drop(stderr_tx);
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert!(clean_shutdown.load(Ordering::Acquire));
    remove_log_ring(&id);
}

#[tokio::test]
async fn background_history_stays_bounded_without_a_webview() {
    let id: Arc<str> = Arc::from(uuid::Uuid::new_v4().to_string());
    let count = LOG_RING_CAPACITY + 100;
    let (stdout_tx, stdout_rx) = broadcast::channel(count);
    let (stderr_tx, stderr_rx) = broadcast::channel(1);
    for n in 0..count {
        stdout_tx.send(format!("log {n}")).unwrap();
    }
    drop(stdout_tx);
    drop(stderr_tx);
    let task = spawn_io_forwarding(
        None,
        id.clone(),
        stdout_rx,
        stderr_rx,
        None,
        Arc::new(AtomicBool::new(false)),
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    let history = get_log_history(&id, None).await;
    assert_eq!(history.len(), LOG_RING_CAPACITY);
    assert_eq!(history.first().unwrap().text.as_ref(), "log 100");
    assert_eq!(
        history.last().unwrap().text.as_ref(),
        format!("log {}", count - 1)
    );
    remove_log_ring(&id);
}
