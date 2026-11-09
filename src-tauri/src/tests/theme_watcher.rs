use super::*;
use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("cubic-watch-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn filters_generation_path_and_event_kind() {
    let mut state = WatchState::default();
    let root = std::env::temp_dir().join("themes/example");
    state.select(Some(root.clone()));
    for kind in [
        EventKind::Create(CreateKind::File),
        EventKind::Modify(ModifyKind::Any),
        EventKind::Remove(RemoveKind::File),
    ] {
        let event = Event::new(kind).add_path(root.join("assets/nested/image.png"));
        assert!(state.accepts(state.generation, &event));
        assert!(!state.accepts(state.generation - 1, &event));
    }
    let access = Event::new(EventKind::Access(AccessKind::Any)).add_path(root.join("Meta.toml"));
    assert!(!state.accepts(state.generation, &access));
    let outside = Event::new(EventKind::Modify(ModifyKind::Any))
        .add_path(root.with_file_name("example-other").join("Inject.css"));
    assert!(!state.accepts(state.generation, &outside));
}

#[test]
fn same_selection_keeps_watch_but_import_clears_pending_events() {
    let mut state = WatchState::default();
    let root = PathBuf::from("themes/example");
    state.select(Some(root.clone()));
    let old_generation = state.generation;
    state.last_event = Some(Instant::now());
    state.select(Some(root.clone()));
    assert_eq!(state.generation, old_generation);
    assert!(state.last_event.is_some());
    state.pause(&root);
    assert!(state.last_event.is_none());
    let event = Event::new(EventKind::Remove(RemoveKind::Any)).add_path(root.clone());
    assert!(!state.accepts(state.generation, &event));
    assert!(state.resume(&root));
    assert!(!state.accepts(old_generation, &event));
    assert!(state.accepts(state.generation, &event));
}

#[test]
fn finishing_import_does_not_rearm_obsolete_selection() {
    let mut state = WatchState::default();
    let imported = PathBuf::from("themes/imported");
    let other = PathBuf::from("themes/other");
    state.select(Some(imported.clone()));
    state.pause(&imported);
    state.select(Some(other.clone()));
    let generation = state.generation;
    assert!(!state.resume(&imported));
    assert_eq!(state.target, Some(other));
    assert_eq!(state.generation, generation);
    state.pause(&imported);
    state.select(None);
    assert!(!state.resume(&imported));
    assert!(state.target.is_none());
}

#[test]
fn selection_during_import_remains_paused_until_resume() {
    let mut state = WatchState::default();
    let root = PathBuf::from("themes/imported");
    state.pause(&root);
    state.select(Some(root.clone()));
    let event = Event::new(EventKind::Create(CreateKind::Any)).add_path(root.clone());
    assert!(!state.accepts(state.generation, &event));
    assert!(state.resume(&root));
    assert!(state.accepts(state.generation, &event));
}

#[cfg(unix)]
#[test]
fn canonical_events_keep_logical_pause_coordination_and_survive_removal() {
    let dir = TestDir::new();
    let storage = dir.0.join("storage");
    std::fs::create_dir_all(storage.join("theme/assets")).unwrap();
    let themes = dir.0.join("themes");
    std::os::unix::fs::symlink(&storage, &themes).unwrap();
    let logical_root = themes.join("theme");
    let canonical_root = std::fs::canonicalize(&logical_root).unwrap();
    let mut state = WatchState::default();
    state.select(Some(logical_root.clone()));
    assert_eq!(state.target.as_ref(), Some(&logical_root));
    assert_eq!(state.event_root.as_ref(), Some(&canonical_root));

    let logical_event = Event::new(EventKind::Remove(RemoveKind::File))
        .add_path(logical_root.join("assets/icon.svg"));
    let canonical_event = Event::new(EventKind::Remove(RemoveKind::File))
        .add_path(canonical_root.join("assets/icon.svg"));
    // Cached roots must also match paths that no longer exist.
    assert!(state.accepts(state.generation, &logical_event));
    assert!(state.accepts(state.generation, &canonical_event));
    let staging_event = Event::new(EventKind::Create(CreateKind::File))
        .add_path(storage.join(".theme-import-test/theme/Meta.toml"));
    assert!(!state.accepts(state.generation, &staging_event));
    let previous_generation = state.generation;
    state.pause(&logical_root);
    assert!(!state.accepts(state.generation, &canonical_event));
    assert!(state.resume(&logical_root));
    assert!(state.accepts(state.generation, &canonical_event));
    assert!(!state.accepts(previous_generation, &canonical_event));

    std::fs::remove_dir_all(&logical_root).unwrap();
    assert!(state.accepts(state.generation, &canonical_event));
    let other = dir.0.join("other");
    std::fs::create_dir(&other).unwrap();
    state.select(Some(other.clone()));
    assert_eq!(
        state.event_root,
        Some(std::fs::canonicalize(other).unwrap())
    );
    assert!(!state.accepts(state.generation, &canonical_event));
    state.select(None);
    assert!(state.event_root.is_none());
}

#[test]
fn recursive_watch_observes_removal_and_same_id_replacement() {
    let dir = TestDir::new();
    let root = dir.0.join("theme");
    let asset = root.join("assets/nested/icon.svg");
    std::fs::create_dir_all(asset.parent().unwrap()).unwrap();
    let canonical_root = std::fs::canonicalize(&root).unwrap();
    let canonical_asset = std::fs::canonicalize(asset.parent().unwrap())
        .unwrap()
        .join("icon.svg");
    let (tx, rx) = mpsc::channel();
    let mut state = WatchState {
        tx: Some(tx),
        ..Default::default()
    };
    state.select(Some(root.clone()));
    assert!(state.watcher.is_some());

    let is_relevant = |event: &Event| {
        event.paths.iter().any(|path| {
            (path == &asset || path == &canonical_asset)
                || ((asset.starts_with(path) || canonical_asset.starts_with(path))
                    && path != &root
                    && path != &canonical_root)
        })
    };
    let wait_for = |state: &WatchState, removal: bool, trigger: &mut dyn FnMut()| {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            trigger();
            let attempt = Instant::now() + Duration::from_millis(750);
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                assert!(!remaining.is_zero(), "expected nested asset notification");
                let wait = remaining.min(attempt.saturating_duration_since(Instant::now()));
                let notification = match rx.recv_timeout(wait) {
                    Ok(notification) => notification,
                    Err(_) => break,
                };
                let (generation, event) = notification;
                let Ok(event) = event else { continue };
                if state.accepts(generation, &event)
                    && is_relevant(&event)
                    && (!removal || matches!(event.kind, EventKind::Remove(_)))
                {
                    return;
                }
            }
            assert!(
                Instant::now() < deadline,
                "expected nested asset notification"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    };
    let write = |content: &'static str| {
        std::fs::write(&asset, content).unwrap();
    };
    let mut remove = || {
        if !asset.exists() {
            std::fs::write(&asset, "retry").unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = std::fs::remove_file(&asset);
    };
    wait_for(&state, false, &mut || write("old"));
    wait_for(&state, true, &mut remove);

    let stale_generation = state.generation;
    state.pause(&root);
    assert!(state.watcher.is_none());
    std::fs::rename(&root, dir.0.join("backup")).unwrap();
    std::fs::create_dir_all(asset.parent().unwrap()).unwrap();
    assert!(state.resume(&root));
    assert!(state.watcher.is_some());
    for (generation, event) in rx.try_iter() {
        let Ok(event) = event else { continue };
        if generation <= stale_generation {
            assert!(!state.accepts(generation, &event));
        }
    }
    wait_for(&state, false, &mut || write("replacement"));
    wait_for(&state, true, &mut remove);

    // A failed replacement uses the same resume path, on the original inode.
    state.pause(&root);
    assert!(state.resume(&root));
    assert!(state.watcher.is_some());
    wait_for(&state, false, &mut || write("after failure"));
}
