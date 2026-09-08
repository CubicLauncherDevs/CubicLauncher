use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, mpsc};
use std::time::{Duration, Instant};

use crate::core::{AppEvent, PathManager, emit, validate_identifier};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use parking_lot::Mutex;
use tracing::warn;

type Notification = (u64, notify::Result<Event>);
static STATE: LazyLock<Mutex<WatchState>> = LazyLock::new(|| Mutex::new(WatchState::default()));

#[derive(Default)]
struct WatchState {
    target: Option<PathBuf>,
    event_root: Option<PathBuf>,
    paused: HashMap<PathBuf, usize>,
    generation: u64,
    watcher: Option<notify::RecommendedWatcher>,
    tx: Option<mpsc::Sender<Notification>>,
    last_event: Option<Instant>,
}

impl WatchState {
    fn select(&mut self, target: Option<PathBuf>) {
        if self.target != target {
            self.target = target;
            self.rearm();
        }
    }

    fn rearm(&mut self) {
        // The lock acknowledges logical invalidation, not OS thread teardown.
        // Each old callback keeps its original generation, even if delayed.
        self.generation += 1;
        self.last_event = None;
        self.watcher = None;
        self.event_root = None;
        let Some(path) = &self.target else { return };
        if self.paused.contains_key(path) {
            return;
        }
        self.event_root = std::fs::canonicalize(path).ok();
        let Some(tx) = self.tx.clone() else { return };
        let generation = self.generation;
        let result = notify::recommended_watcher(move |event| {
            let _ = tx.send((generation, event));
        })
        .and_then(|mut watcher| {
            watcher.watch(path, RecursiveMode::Recursive)?;
            Ok(watcher)
        });
        match result {
            Ok(watcher) => self.watcher = Some(watcher),
            Err(e) => warn!("ThemeWatcher: no se pudo observar {:?}: {}", path, e),
        }
    }

    fn pause(&mut self, path: &Path) {
        *self.paused.entry(path.to_owned()).or_default() += 1;
        if self.target.as_deref() == Some(path) {
            self.rearm();
        }
    }

    fn resume(&mut self, path: &Path) -> bool {
        if let Some(count) = self.paused.get_mut(path) {
            *count -= 1;
            if *count == 0 {
                self.paused.remove(path);
            }
        }
        let active = self.target.as_deref() == Some(path);
        if active {
            self.rearm();
        }
        active
    }

    fn accepts(&self, generation: u64, event: &Event) -> bool {
        generation == self.generation
            && self.target.as_ref().is_some_and(|path| {
                !self.paused.contains_key(path)
                    && event.paths.iter().any(|p| {
                        p.starts_with(path)
                            || self
                                .event_root
                                .as_ref()
                                .is_some_and(|root| p.starts_with(root))
                    })
            })
            && matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
            )
    }
}

pub struct ThemeWatcher;

pub(crate) struct ImportWatchGuard {
    path: PathBuf,
    successful: bool,
}

impl ImportWatchGuard {
    pub fn finish(mut self) {
        self.successful = true;
    }
}

impl Drop for ImportWatchGuard {
    fn drop(&mut self) {
        let mut state = STATE.lock();
        if state.resume(&self.path) && self.successful {
            emit_change(&self.path);
        }
    }
}

fn emit_change(path: &Path) {
    if let Some(id) = path.file_name().and_then(|name| name.to_str()) {
        emit(AppEvent::ThemeChanged {
            id: format!("user:{id}").into(),
        });
    }
}

impl ThemeWatcher {
    pub fn watch(id: Option<String>) {
        let target = id.and_then(|id| {
            if let Err(e) = validate_identifier(&id) {
                warn!("ThemeWatcher: id invalido: {}", e);
                return None;
            }
            Some(PathManager::get().get_themes_dir().join(id))
        });
        STATE.lock().select(target);
    }

    pub(crate) fn pause_import(path: &Path) -> ImportWatchGuard {
        STATE.lock().pause(path);
        ImportWatchGuard {
            path: path.to_owned(),
            successful: false,
        }
    }

    pub async fn start() {
        let (tx, rx) = mpsc::channel::<Notification>();
        {
            let mut state = STATE.lock();
            if state.tx.is_some() {
                return;
            }
            state.tx = Some(tx);
            state.rearm();
        }
        tokio::task::spawn_blocking(move || {
            loop {
                let notification = rx.recv_timeout(Duration::from_millis(100));
                let mut state = STATE.lock();
                match notification {
                    Ok((generation, Ok(event))) if state.accepts(generation, &event) => {
                        state.last_event = Some(Instant::now());
                    }
                    Ok((generation, Err(e))) if generation == state.generation => {
                        warn!("ThemeWatcher: error de notify: {}", e);
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    _ => {}
                }
                if state
                    .last_event
                    .is_some_and(|last| last.elapsed() >= Duration::from_millis(200))
                {
                    state.last_event = None;
                    if let Some(path) = &state.target {
                        emit_change(path);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("cubic-watch-test-{}", uuid::Uuid::new_v4()));
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
        let access =
            Event::new(EventKind::Access(AccessKind::Any)).add_path(root.join("Meta.toml"));
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

        let wait_for = |state: &WatchState, removal: bool| {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let (generation, event) = rx
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    .expect("expected nested asset notification");
                let event = event.unwrap();
                if state.accepts(generation, &event)
                    && (event.paths.contains(&asset) || event.paths.contains(&canonical_asset))
                    && (!removal || matches!(event.kind, EventKind::Remove(_)))
                {
                    break;
                }
            }
        };
        std::fs::write(&asset, "old").unwrap();
        wait_for(&state, false);
        std::fs::remove_file(&asset).unwrap();
        wait_for(&state, true);

        state.pause(&root);
        assert!(state.watcher.is_none());
        std::fs::rename(&root, dir.0.join("backup")).unwrap();
        std::fs::create_dir_all(asset.parent().unwrap()).unwrap();
        assert!(state.resume(&root));
        assert!(state.watcher.is_some());
        for (generation, event) in rx.try_iter() {
            assert!(!state.accepts(generation, &event.unwrap()));
        }
        std::fs::write(&asset, "replacement").unwrap();
        wait_for(&state, false);
        std::fs::remove_file(&asset).unwrap();
        wait_for(&state, true);

        // A failed replacement uses the same resume path, on the original inode.
        state.pause(&root);
        assert!(state.resume(&root));
        assert!(state.watcher.is_some());
        std::fs::write(&asset, "after failure").unwrap();
        wait_for(&state, false);
    }
}
