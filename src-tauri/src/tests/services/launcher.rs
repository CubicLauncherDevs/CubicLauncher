use super::*;
use crate::services::instance_manager::signal_kill;

#[test]
fn only_unsolicited_nonzero_exits_are_crashes() {
    for (exit_code, kill_requested, expected) in [
        (None, false, false),
        (None, true, false),
        (Some(0), false, false),
        (Some(0), true, false),
        (Some(1), false, true),
        (Some(1), true, false),
        (Some(-1), false, true),
        (Some(-1), true, false),
        (Some(-1073741819), false, true),
        (Some(-1073741819), true, false),
    ] {
        assert_eq!(
            is_unexpected_exit(exit_code, kill_requested),
            expected,
            "exit_code={exit_code:?}, kill_requested={kill_requested}"
        );
    }
}

#[test]
fn an_exit_observed_before_receiving_kill_is_still_intentional() {
    let id = uuid::Uuid::new_v4().to_string();
    let (tx, _rx) = tokio::sync::oneshot::channel();
    let requested = register_kill_sender(&id, tx);
    assert!(signal_kill(&id));
    unregister_kill_sender(&id);

    // Simulate wait() completing without the kill branch consuming its signal.
    assert!(!is_unexpected_exit(
        Some(1),
        requested.load(Ordering::Acquire)
    ));
}

#[test]
fn readiness_uses_the_minecraft_client_marker() {
    for line in [
        "[12:00:00] [Render thread/INFO]: Setting user: Player",
        "[Client thread/INFO]: Setting user: Player",
        "SETTING USER: Player",
        "Setting user Player",
    ] {
        assert!(is_game_ready(line), "{line}");
    }
    for line in [
        "Starting Java 21",
        "Setting username: Player",
        "Setting user:",
        "Setting user:   ",
        "Resetting user: Player",
    ] {
        assert!(!is_game_ready(line), "{line}");
    }
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
    let task = spawn_io_forwarding(None, id.clone(), stdout_rx, stderr_rx, Some(ready_tx));
    tokio::time::timeout(std::time::Duration::from_secs(2), ready_rx)
        .await
        .unwrap()
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
    let task = spawn_io_forwarding(None, id.clone(), stdout_rx, stderr_rx, None);
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
