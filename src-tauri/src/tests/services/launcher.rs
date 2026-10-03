use super::*;
use crate::services::instance_manager::signal_kill;

fn attempt_handle() -> InstanceHandle {
    let data = serde_json::from_value(serde_json::json!({
        "name": "Launch regression", "version": "1.21", "last_played": 0,
        "uuid": uuid::Uuid::new_v4().to_string()
    }))
    .unwrap();
    InstanceHandle::new(data)
}

#[test]
fn failed_launch_attempts_release_busy_state_and_preserve_specific_errors() {
    let handle = attempt_handle();
    for error in [
        AppError::CoreError(crate::core::CoreError::Other("spawn failed".into())),
        AppError::Download(DownloadError::ParseJson("bad manifest".into())),
        AppError::Auth(AuthError::AuthFailed("invalid credentials".into())),
    ] {
        handle.set_status(InstanceStatus::Starting);
        let attempt = LaunchAttempt(handle.clone());
        attempt.finish(&Err(error));
        drop(attempt);
        assert!(matches!(handle.get_status(), InstanceStatus::Error(_)));
        assert!(!handle.is_busy());
    }
    handle.set_status(InstanceStatus::Starting);
    let attempt = LaunchAttempt(handle.clone());
    handle.set_status(InstanceStatus::Off); // repair was queued
    attempt.finish(&Err(AppError::Instance(InstanceError::VersionNotFound(
        "1.21".into(),
    ))));
    drop(attempt);
    assert!(matches!(handle.get_status(), InstanceStatus::Off));
}

#[tokio::test]
async fn abandoned_attempt_releases_starting_but_handoff_keeps_started() {
    let handle = attempt_handle();
    handle.set_status(InstanceStatus::Starting);
    let attempt = LaunchAttempt(handle.clone());
    let task = tokio::spawn(async move {
        let _attempt = attempt;
        std::future::pending::<()>().await;
    });
    task.abort();
    let _ = task.await;
    assert!(!handle.is_busy());
    handle.set_status(InstanceStatus::Starting);
    let attempt = LaunchAttempt(handle.clone());
    handle.set_status(InstanceStatus::Started);
    attempt.finish(&Ok(()));
    drop(attempt);
    assert!(matches!(handle.get_status(), InstanceStatus::Started));
}

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

#[test]
fn readiness_fallback_is_only_enabled_for_old_minecraft_betas() {
    for (version, parent) in [
        ("b1.0", None),
        ("b1.7_01", None),
        ("b1.7.3", None),
        ("b1.8.1", None),
        ("b1.9-pre4", None),
        ("b1.7.3-OptiFine_HD_G", None),
        ("custom-profile", Some("b1.7.3")),
    ] {
        assert_eq!(
            beta_readiness_delay(version, parent, true),
            Some(std::time::Duration::from_secs(2)),
            "{version} / {parent:?}"
        );
        assert_eq!(beta_readiness_delay(version, parent, false), None);
    }
    for version in [
        "1.21.4",
        "1.0",
        "a1.2.6",
        "26.2-snapshot-5",
        "36.0.0-beta.1",
        "fabric-loader-0.16.0-1.21",
        "bukkit",
        "b1",
        "b1.",
        "beta1.7.3",
    ] {
        assert_eq!(beta_readiness_delay(version, None, true), None, "{version}");
    }
    assert_eq!(beta_readiness_delay("b1.7.3", Some("1.21"), true), None);
}

#[tokio::test]
async fn silent_betas_become_ready_after_the_grace_period_with_open_or_closed_logs() {
    let delay = beta_readiness_delay("b1.7.3", None, true).unwrap();
    futures::future::join_all([false, true].map(|closed| async move {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let sender = (!closed).then_some(tx);
        let deadline = tokio::time::Instant::now() + delay;
        let readiness = wait_for_game_readiness(rx, Some(deadline));
        tokio::pin!(readiness);
        assert!(
            futures::poll!(&mut readiness).is_pending(),
            "hid before the grace period"
        );
        tokio::time::timeout(std::time::Duration::from_secs(5), readiness)
            .await
            .expect("silent beta never became ready");
        assert!(tokio::time::Instant::now() >= deadline);
        drop(sender);
    }))
    .await;
}

#[tokio::test]
async fn client_marker_can_hide_before_the_beta_fallback_deadline() {
    for deadline in [
        None,
        Some(tokio::time::Instant::now() + std::time::Duration::from_secs(60)),
    ] {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let readiness = wait_for_game_readiness(rx, deadline);
        tokio::pin!(readiness);
        assert!(futures::poll!(&mut readiness).is_pending());
        tx.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), readiness)
            .await
            .expect("client marker was ignored");
    }
}

#[tokio::test]
async fn closed_logs_without_a_beta_fallback_do_not_signal_readiness() {
    let (tx, rx) = tokio::sync::oneshot::channel();
    drop(tx);
    let readiness = wait_for_game_readiness(rx, None);
    tokio::pin!(readiness);
    assert!(futures::poll!(&mut readiness).is_pending());
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
