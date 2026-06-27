use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::core::{AppEvent, PathManager, emit};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use tracing::{info, warn};

static WATCHER_TX: OnceLock<mpsc::Sender<Option<String>>> = OnceLock::new();

pub struct ThemeWatcher;

impl ThemeWatcher {
    pub fn watch(id: Option<String>) {
        info!("ThemeWatcher: cambiando watch a {:?}", id);
        if let Some(tx) = WATCHER_TX.get() {
            let _ = tx.send(id);
        }
    }

    pub async fn start() {
        info!("ThemeWatcher: iniciando (notify)");

        let (cmd_tx, cmd_rx) = mpsc::channel::<Option<String>>();
        let _ = WATCHER_TX.set(cmd_tx);

        tokio::task::spawn_blocking(move || {
            let (notify_tx, notify_rx) = mpsc::channel::<Result<Event, notify::Error>>();

            let mut watcher: notify::RecommendedWatcher =
                match notify::recommended_watcher(move |res| {
                    let _ = notify_tx.send(res);
                }) {
                    Ok(w) => w,
                    Err(e) => {
                        warn!("ThemeWatcher: error al crear watcher: {}", e);
                        return;
                    }
                };

            let mut current_id: Option<String> = None;
            let mut watched_path: Option<PathBuf> = None;
            let mut last_event: Option<Instant> = None;
            const DEBOUNCE_MS: u64 = 200;

            loop {
                match cmd_rx.try_recv() {
                    Ok(new_id) => {
                        info!("ThemeWatcher: nuevo theme ID: {:?}", new_id);
                        current_id = new_id;

                        if let Some(old) = watched_path.take()
                            && let Err(e) = watcher.unwatch(&old)
                        {
                            warn!("ThemeWatcher: error al dejar de observar {:?}: {}", old, e);
                        }

                        if let Some(ref id) = current_id {
                            let path = PathManager::get().get_themes_dir().join(id);
                            if path.exists() {
                                info!("ThemeWatcher: observando {:?}", path);
                                if let Err(e) = watcher.watch(&path, RecursiveMode::NonRecursive) {
                                    warn!("ThemeWatcher: error al observar {:?}: {}", path, e);
                                } else {
                                    watched_path = Some(path);
                                }
                            } else {
                                warn!("ThemeWatcher: el directorio {:?} no existe", path);
                            }
                        }
                        last_event = None;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        warn!("ThemeWatcher: canal desconectado, deteniendo");
                        return;
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }

                match notify_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(event)) => {
                        if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                            info!("ThemeWatcher: cambio detectado en {:?}", event.paths);
                            last_event = Some(Instant::now());
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("ThemeWatcher: error de notify: {:?}", e);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        warn!("ThemeWatcher: notify desconectado");
                        return;
                    }
                }

                if let Some(le) = last_event
                    && le.elapsed() >= Duration::from_millis(DEBOUNCE_MS)
                {
                    if let Some(ref id) = current_id {
                        info!("ThemeWatcher: cambio confirmado en theme '{}'", id);
                        emit(AppEvent::ThemeChanged {
                            id: format!("user:{}", id).into(),
                        });
                    }
                    last_event = None;
                }
            }
        });
    }
}
