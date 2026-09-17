//! Main-window lifetime is independent from the Minecraft processes it supervises.
//! Window operations are serialized, and restoration waits for Destroyed.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};
use tracing::{error, warn};

#[derive(Default)]
struct WindowLifecycle {
    sessions: HashMap<u64, Session>,
    closing_main: bool,
    reopen_pending: bool,
}

struct Session {
    close_on_ready: bool,
    restore_on_exit: bool,
}

impl WindowLifecycle {
    fn start(&mut self, id: u64, close_on_ready: bool) {
        self.sessions.insert(
            id,
            Session {
                close_on_ready,
                restore_on_exit: false,
            },
        );
    }

    fn ready(&mut self, id: u64) -> bool {
        let Some(session) = self.sessions.get_mut(&id) else {
            return false; // A queued log notification can arrive after process exit.
        };
        if !session.close_on_ready || session.restore_on_exit {
            return false;
        }
        session.restore_on_exit = true;
        true
    }

    fn finish(&mut self, id: u64) {
        if let Some(session) = self.sessions.remove(&id) {
            self.reopen_pending |= session.restore_on_exit;
        }
    }

    fn stop_readiness(&mut self, id: u64) {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.close_on_ready = false;
        }
    }

    fn should_keep_alive(&self) -> bool {
        !self.sessions.is_empty() || self.closing_main || self.reopen_pending
    }

    fn can_restore(&self) -> bool {
        self.reopen_pending && !self.closing_main
    }
}

static LIFECYCLE: LazyLock<Mutex<WindowLifecycle>> =
    LazyLock::new(|| Mutex::new(WindowLifecycle::default()));
// Never hold this on the UI thread: WebView2 creation must run outside Tauri
// event handlers. Tauri dispatches the native operations to its event loop.
static WINDOW_OPERATIONS: Mutex<()> = Mutex::new(());
static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

pub(crate) fn should_keep_alive() -> bool {
    LIFECYCLE.lock().should_keep_alive()
}

/// Used both at startup and after a game exits, so configuration stays identical.
pub(crate) fn create_main_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .expect("main window configuration is required");
    WebviewWindowBuilder::from_config(app, config)?
        .devtools(cfg!(debug_assertions))
        .build()
}

fn restore_or_exit(app: &AppHandle) {
    if LIFECYCLE.lock().can_restore() {
        let result = app
            .get_webview_window("main")
            .map(Ok)
            .unwrap_or_else(|| create_main_window(app));
        match result {
            Ok(window) => {
                if let Err(err) = window
                    .show()
                    .and_then(|_| window.unminimize())
                    .and_then(|_| window.set_focus())
                {
                    warn!("No se pudo activar la ventana principal: {err}");
                }
            }
            Err(err) => error!("No se pudo recrear la ventana principal: {err}"),
        }
        // Never leave an invisible, immortal backend if window creation failed.
        LIFECYCLE.lock().reopen_pending = false;
    }
    if !should_keep_alive() && app.webview_windows().is_empty() {
        app.exit(0);
    }
}

/// close() is asynchronous: rebuilding before Destroyed can collide with "main".
pub(crate) fn on_main_destroyed(app: &AppHandle) {
    LIFECYCLE.lock().closing_main = false;
    schedule_restore(app);
}

fn schedule_restore(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _operations = WINDOW_OPERATIONS.lock();
        restore_or_exit(&handle);
    });
}

/// Kept by the process supervisor until logs and exit status have been handled.
/// Dropping it also cleans up a launch that failed before creating a process.
pub(crate) struct LaunchSession {
    id: u64,
    app: Option<AppHandle>,
}

impl LaunchSession {
    pub(crate) fn new(app: Option<AppHandle>, close_on_ready: bool) -> Self {
        let id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        LIFECYCLE.lock().start(id, close_on_ready);
        Self { id, app }
    }

    pub(crate) fn game_ready(&self) {
        let Some(app) = &self.app else { return };
        let id = self.id;
        let handle = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _operations = WINDOW_OPERATIONS.lock();
            let mut state = LIFECYCLE.lock();
            if !state.ready(id) || state.closing_main {
                return;
            }
            if let Some(window) = handle.get_webview_window("main") {
                state.closing_main = true;
                drop(state);
                if let Err(err) = window.close() {
                    LIFECYCLE.lock().closing_main = false;
                    warn!("No se pudo cerrar la ventana principal al jugar: {err}");
                    restore_or_exit(&handle);
                }
            }
        });
    }

    pub(crate) fn process_exited(&self) {
        LIFECYCLE.lock().stop_readiness(self.id);
    }
}

impl Drop for LaunchSession {
    fn drop(&mut self) {
        LIFECYCLE.lock().finish(self.id);
        if let Some(app) = &self.app {
            schedule_restore(app);
        }
    }
}

#[cfg(test)]
#[path = "../tests/services/launch_window.rs"]
mod tests;
