use super::*;

fn session(running: bool, expired: bool) -> (String, Arc<Session>) {
    let token = uuid::Uuid::new_v4().to_string();
    let session = Arc::new(Session {
        candidates: Vec::new(),
        created: Instant::now() - Duration::from_secs(if expired { 901 } else { 0 }),
        cancelled: Arc::new(AtomicBool::new(false)),
        running: AtomicBool::new(running),
    });
    SESSIONS.lock().insert(token.clone(), session.clone());
    (token, session)
}

#[test]
fn cancelling_preview_releases_it_and_running_job_observes_cancellation() {
    let (preview, preview_session) = session(false, false);
    cancel(&preview);
    assert!(preview_session.cancelled.load(Ordering::Relaxed));
    assert!(!SESSIONS.lock().contains_key(&preview));

    let (running, running_session) = session(true, false);
    cancel(&running);
    assert!(running_session.cancelled.load(Ordering::Relaxed));
    assert!(SESSIONS.lock().contains_key(&running));
    drop(SessionGuard(running.clone()));
    assert!(!SESSIONS.lock().contains_key(&running));
}

#[tokio::test]
async fn expired_or_unknown_previews_cannot_start_migration() {
    let (token, _) = session(false, true);
    let result = migrate(token.clone(), Vec::new(), Channel::new(|_| Ok(()))).await;
    assert!(result.unwrap_err().contains("expired"));
    cancel(&token);
    let result = migrate(token, Vec::new(), Channel::new(|_| Ok(()))).await;
    assert!(result.unwrap_err().contains("expired"));
}
