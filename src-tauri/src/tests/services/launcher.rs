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

#[test]
fn only_unsolicited_nonzero_exits_are_crashes() {
    for (exit_code, kill_requested, clean_shutdown, expected) in [
        (None, false, false, false),
        (None, true, false, false),
        (None, false, true, false),
        (Some(0), false, false, false),
        (Some(0), true, false, false),
        (Some(0), false, true, false),
        (Some(1), false, false, true),
        (Some(1), true, false, false),
        // Cierre normal con watchdog de apagado: código distinto de cero sin crash.
        (Some(1), false, true, false),
        (Some(1), true, true, false),
        (Some(-1), false, false, true),
        (Some(-1), true, false, false),
        (Some(-1), false, true, false),
        (Some(-1073741819), false, false, true),
        (Some(-1073741819), true, false, false),
        (Some(-1073741819), false, true, false),
    ] {
        assert_eq!(
            is_unexpected_exit(exit_code, kill_requested, clean_shutdown),
            expected,
            "exit_code={exit_code:?}, kill_requested={kill_requested}, clean_shutdown={clean_shutdown}"
        );
    }
}

#[test]
fn the_client_shutdown_marker_flags_a_clean_exit() {
    for line in [
        "[21:36:41] [Render thread/INFO]: Stopping!",
        "[18sept2026 11:53:49.957] [Render thread/INFO] [net.minecraft.client.Minecraft/]: Stopping!",
        "[01:17:24] [Client thread/INFO]: Stopping!",
        "[21:36:41] [Render thread/INFO]: Stopping!   ",
    ] {
        assert!(is_clean_shutdown(line), "{line}");
    }
    for line in [
        "[21:15:39] [Render thread/INFO]: Stopping worker threads",
        "[21:36:57] [Thread-3/INFO]: Saving",
        "[Render thread/INFO]: Stopping server",
        "Stopping!",
        "Player Stopping!",
    ] {
        assert!(!is_clean_shutdown(line), "{line}");
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
        requested.load(Ordering::Acquire),
        false
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
