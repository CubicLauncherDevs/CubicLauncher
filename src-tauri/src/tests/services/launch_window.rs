use super::*;

#[test]
fn java_start_without_client_readiness_does_not_request_reopening() {
    let mut state = WindowLifecycle::default();
    state.start(1, true);
    assert!(state.should_keep_alive());
    state.finish(1); // Failed start, early crash, or a client without the marker.
    assert!(!state.can_restore());
    assert!(!state.should_keep_alive());
}

#[test]
fn disabled_setting_never_closes_or_restores() {
    let mut state = WindowLifecycle::default();
    state.start(1, false);
    assert!(!state.ready(1));
    state.finish(1);
    assert!(!state.can_restore());
    assert!(!state.should_keep_alive());
}

#[test]
fn client_readiness_is_handled_only_once_per_session() {
    let mut state = WindowLifecycle::default();
    state.start(1, true);
    assert!(state.ready(1));
    assert!(!state.ready(1));
    state.finish(1);
    assert!(state.can_restore());
    assert!(state.should_keep_alive()); // Keep the event loop until UI restoration.
    state.reopen_pending = false;
    assert!(!state.should_keep_alive());
}

#[test]
fn quick_exit_waits_for_old_window_destruction() {
    let mut state = WindowLifecycle::default();
    state.start(1, true);
    assert!(state.ready(1));
    state.closing_main = true;
    state.finish(1);
    assert!(!state.can_restore());
    assert!(state.should_keep_alive());
    state.closing_main = false;
    assert!(state.can_restore());
}

#[test]
fn delayed_readiness_after_process_exit_cannot_close_the_window() {
    let mut state = WindowLifecycle::default();
    state.start(1, true);
    state.stop_readiness(1); // Process ended; its final logs are still draining.
    assert!(!state.ready(1));
    state.finish(1);
    assert!(!state.ready(1));
    assert!(!state.can_restore());
}

#[test]
fn cleanup_after_exit_preserves_reopening_for_a_ready_client() {
    let mut state = WindowLifecycle::default();
    state.start(1, true);
    assert!(state.ready(1));
    state.stop_readiness(1);
    state.finish(1); // Same transition for normal exit, crash and explicit kill.
    assert!(state.can_restore());
}

#[test]
fn concurrent_sessions_keep_backend_alive_in_either_exit_order() {
    for (first, last) in [(1, 2), (2, 1)] {
        let mut state = WindowLifecycle::default();
        state.start(1, true);
        state.start(2, true);
        assert!(state.ready(1));
        assert!(state.ready(2));
        state.finish(first);
        assert!(state.can_restore());
        state.reopen_pending = false;
        assert!(state.should_keep_alive());
        state.finish(last);
        assert!(state.can_restore());
        state.reopen_pending = false;
        assert!(!state.should_keep_alive());
    }
}

#[test]
fn an_unrelated_exit_does_not_restore_or_release_another_session() {
    let mut state = WindowLifecycle::default();
    state.start(1, true);
    state.start(2, false);
    assert!(state.ready(1));
    state.finish(2);
    assert!(!state.can_restore());
    assert!(state.should_keep_alive());
    state.finish(1);
    assert!(state.can_restore());
}
