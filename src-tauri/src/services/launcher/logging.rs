//! Sanitized log history, crash snapshots and batched WebView delivery.

use super::{is_clean_shutdown, is_game_ready};
use crate::commands::log_window::wants_log_preview;
use dashmap::DashMap;
use launchwerk::auth::{AccountType, MinecraftUser};
use parking_lot::RwLock;
use regex::Regex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::{Emitter, EventTarget, Manager};
use tokio::sync::broadcast;
use tracing::warn;

const LOG_RING_CAPACITY: usize = 5000;

static LINE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

// ── Log Level ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// No se pudo clasificar la línea.
    Unknown,
    /// Error estándar genérico.
    Stderr,
    /// Mensajes propios del launcher.
    Launcher,
    Trace,
    Debug,
    Info,
    /// Mensajes de Minecraft u otros sin tag (texto plano).
    Message,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    /// Detecta el nivel de una línea de log siguiendo formatos comunes de
    /// Minecraft, log4j/logback y Java.
    pub fn from_line(line: &str, stderr: bool) -> Self {
        let stripped = strip_ansi(line);

        // Mensajes marcados por el launcher.
        if stripped.trim_start().starts_with("[CubicLauncher]")
            || stripped.trim_start().starts_with("[Cubite]")
        {
            return Self::Launcher;
        }

        // Formato típico de Minecraft/modloaders:
        // [12:34:56] [Server thread/INFO]: ...
        if let Some(cap) = minecraft_level_regex().captures(&stripped)
            && let Some(level) = cap.get(1)
        {
            return Self::from_keyword(level.as_str());
        }

        // Palabra de nivel suelta al inicio de la línea (log4j básico, etc).
        if let Some(cap) = leading_level_regex().captures(&stripped)
            && let Some(level) = cap.get(1)
        {
            return Self::from_keyword(level.as_str());
        }

        if stderr { Self::Stderr } else { Self::Message }
    }

    fn from_keyword(word: &str) -> Self {
        match word.to_uppercase().as_str() {
            "TRACE" => Self::Trace,
            "DEBUG" => Self::Debug,
            "INFO" | "CONFIG" | "FINE" | "FINER" | "FINEST" => Self::Info,
            "WARN" | "WARNING" => Self::Warn,
            "ERROR" => Self::Error,
            "FATAL" | "SEVERE" => Self::Fatal,
            _ => Self::Unknown,
        }
    }
}

fn minecraft_level_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\[[^\]/\s]{1,60}/\s*(TRACE|DEBUG|INFO|WARN|WARNING|ERROR|FATAL|SEVERE|CONFIG|FINE|FINER|FINEST)\s*\]").unwrap()
    })
}

fn leading_level_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^(?:\d{4}[-/]\d{2}[-/]\d{2}[\sT])?(?:\d{1,2}:\d{2}:\d{2}(?:\.\d+)?(?:\s*[AP]M)?\s+)?(TRACE|DEBUG|INFO|WARN|WARNING|ERROR|FATAL|SEVERE)\b").unwrap()
    })
}

fn ansi_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\x1b\x9b]\[[0-?]*[ -/]*[@-~]").unwrap())
}

fn strip_ansi(line: &str) -> String {
    ansi_regex().replace_all(line, "").into_owned()
}

fn token_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)(access[_-]?token|refresh[_-]?token|client[_-]?token|authorization|bearer)(['"]?\s*[:=]\s*['"]?|\s+)([A-Za-z0-9_\-\.]+)"#).unwrap()
    })
}

fn session_id_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)(session\s+id\s+is\s*[:=]?\s*)([A-Za-z0-9_\-\.=+/]+)"#).unwrap()
    })
}

/// Limpia líneas que puedan contener credenciales. Oculta solo el valor,
/// no descarta la línea completa.
fn sanitize_line(line: &str) -> String {
    let line = token_regex().replace_all(line, "${1}${2}***").into_owned();
    let line = session_id_regex()
        .replace_all(&line, "${1}***")
        .into_owned();
    let line = email_regex().replace_all(&line, "<email>").into_owned();
    ip_regex().replace_all(&line, "<ip>").into_owned()
}

fn ip_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"\b(?:\d{1,3}\.){3}\d{1,3}\b"#).unwrap())
}

fn email_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}"#).unwrap())
}

// ── User-aware Sensitive Data Filter ────────────────────────────────────────

#[derive(Clone)]
struct SensitiveFilter {
    patterns: Vec<(Regex, String)>,
}

impl SensitiveFilter {
    fn from_user(user: &MinecraftUser) -> Self {
        let mut patterns = Vec::new();

        let mut add = |value: &str, label: &str| {
            if value.len() >= 2 {
                patterns.push((value.to_string(), format!("<{label}>")));
            }
        };

        add(&user.username, "username");
        add(&user.uuid, "uuid");

        // Only mask real-looking secrets to avoid replacing short placeholders
        // such as the cracked account access_token "0".
        if user.user_type != AccountType::Cracked && user.access_token.len() >= 8 {
            add(&user.access_token, "access_token");
        }
        if user.user_type != AccountType::Cracked {
            if let Some(ref token) = user.refresh_token
                && token.len() >= 8
            {
                add(token, "refresh_token");
            }
            if let Some(ref token) = user.client_token
                && token.len() >= 8
            {
                add(token, "client_token");
            }
        }

        // Replace longest needles first to avoid partial matches shadowing longer values.
        patterns.sort_by_key(|b| std::cmp::Reverse(b.0.len()));

        let compiled = patterns
            .into_iter()
            .filter_map(|(needle, replacement)| {
                let escaped = regex::escape(&needle);
                Regex::new(&format!(r"(?i)\b{}\b", escaped))
                    .ok()
                    .map(|re| (re, replacement))
            })
            .collect();

        Self { patterns: compiled }
    }

    fn apply(&self, line: &str) -> String {
        let mut out = line.to_string();
        for (re, replacement) in &self.patterns {
            out = re.replace_all(&out, replacement.as_str()).into_owned();
        }
        out
    }
}

static USER_FILTER: OnceLock<RwLock<Option<Arc<SensitiveFilter>>>> = OnceLock::new();

fn user_filter() -> &'static RwLock<Option<Arc<SensitiveFilter>>> {
    USER_FILTER.get_or_init(|| RwLock::new(None))
}

/// Crea o actualiza el filtro de datos sensibles a partir del usuario activo.
/// Debe llamarse después de cargar los tokens del almacenamiento seguro.
pub(crate) fn set_user_filter(user: &MinecraftUser) {
    let filter = Arc::new(SensitiveFilter::from_user(user));
    *user_filter().write() = Some(filter);
}

/// Sanitiza una línea aplicando el filtro de usuario (si existe) y luego el
/// reemplazo genérico de tokens.
pub(crate) fn sanitize_with_user(line: &str) -> String {
    let guard = user_filter().read();
    let user_cleaned = guard
        .as_ref()
        .map(|filter| filter.apply(line))
        .unwrap_or_else(|| line.to_string());
    drop(guard);
    sanitize_line(&user_cleaned)
}

// ── Log Ring Buffer ─────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize)]
pub struct LogLine {
    pub id: u64,
    pub text: Arc<str>,
    pub stream: String,
    pub level: LogLevel,
    pub timestamp: u64,
}

struct LogLineRaw {
    id: u64,
    text: Arc<str>,
    stream: u8,
    level: LogLevel,
    timestamp: u64,
}

pub struct LogRing {
    inner: tokio::sync::Mutex<VecDeque<LogLineRaw>>,
}

impl LogRing {
    fn new() -> Self {
        Self {
            inner: tokio::sync::Mutex::new(VecDeque::with_capacity(LOG_RING_CAPACITY)),
        }
    }

    pub async fn push(&self, text: Arc<str>, level: LogLevel, stream: u8) -> (u64, u64) {
        let mut guard = self.inner.lock().await;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if guard.len() >= LOG_RING_CAPACITY {
            guard.pop_front();
        }
        let id = LINE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        guard.push_back(LogLineRaw {
            id,
            text,
            stream,
            level,
            timestamp: ts,
        });
        (id, ts)
    }

    pub async fn snapshot(&self, limit: Option<usize>) -> Vec<LogLine> {
        let guard = self.inner.lock().await;
        let to_skip = limit.and_then(|l| guard.len().checked_sub(l)).unwrap_or(0);
        guard
            .iter()
            .skip(to_skip)
            .map(|raw| LogLine {
                id: raw.id,
                text: raw.text.clone(),
                stream: match raw.stream {
                    0 => "stdout".into(),
                    1 => "stderr".into(),
                    _ => "launcher".into(),
                },
                level: raw.level,
                timestamp: raw.timestamp,
            })
            .collect()
    }
}

static LOG_RINGS: OnceLock<DashMap<Arc<str>, Arc<LogRing>>> = OnceLock::new();

fn get_log_ring(id: &str) -> Arc<LogRing> {
    let map = LOG_RINGS.get_or_init(DashMap::new);
    map.entry(Arc::from(id))
        .or_insert_with(|| Arc::new(LogRing::new()))
        .clone()
}

pub async fn get_log_history(id: &str, limit: Option<usize>) -> Vec<LogLine> {
    let ring = {
        let map = LOG_RINGS.get_or_init(DashMap::new);
        map.get(id).map(|entry| entry.clone())
    };

    if let Some(ring) = ring {
        ring.snapshot(limit).await
    } else {
        get_crash_log_snapshot(id, limit).await.unwrap_or_default()
    }
}

pub fn remove_log_ring(id: &str) {
    if let Some(map) = LOG_RINGS.get() {
        map.remove(id);
    }
}

struct CrashLogSnapshot {
    lines: Vec<LogLine>,
    created: std::time::Instant,
}

/// Guarda un snapshot de los logs de una instancia antes de descartar el
/// anillo, para que la ventana de logs del crash siga teniendo historial.
pub async fn save_crash_log_snapshot(id: &str) {
    let snapshot = CrashLogSnapshot {
        lines: get_log_ring(id).snapshot(None).await,
        created: std::time::Instant::now(),
    };
    crash_log_snapshots().insert(Arc::from(id), snapshot);
    cleanup_crash_log_snapshots();
}

pub fn clear_crash_log_snapshot(id: &str) {
    crash_log_snapshots().remove(id);
}

async fn get_crash_log_snapshot(id: &str, limit: Option<usize>) -> Option<Vec<LogLine>> {
    crash_log_snapshots().get(id).map(|entry| {
        let lines = &entry.value().lines;
        let to_skip = limit.and_then(|l| lines.len().checked_sub(l)).unwrap_or(0);
        lines.iter().skip(to_skip).cloned().collect()
    })
}

fn crash_log_snapshots() -> &'static DashMap<Arc<str>, CrashLogSnapshot> {
    static SNAPSHOTS: OnceLock<DashMap<Arc<str>, CrashLogSnapshot>> = OnceLock::new();
    SNAPSHOTS.get_or_init(DashMap::new)
}

const CRASH_LOG_SNAPSHOT_TTL: std::time::Duration = std::time::Duration::from_secs(3600);

fn cleanup_crash_log_snapshots() {
    let now = std::time::Instant::now();
    crash_log_snapshots()
        .retain(|_, snapshot| now.duration_since(snapshot.created) < CRASH_LOG_SNAPSHOT_TTL);
}

/// Añade un mensaje del launcher al anillo de logs de una instancia.
/// Sanitiza el contenido para evitar fugas de datos de la cuenta activa.
pub async fn push_launcher_message(id: &str, message: impl Into<String>) {
    let sanitized = sanitize_with_user(&message.into());
    get_log_ring(id)
        .push(Arc::from(sanitized), LogLevel::Launcher, 2)
        .await;
}

// ── Log Event Payloads ──────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize)]
struct LogEntryEvent {
    id: u64,
    line: Arc<str>,
    stream: &'static str,
    level: LogLevel,
    timestamp: u64,
}

#[derive(Clone, serde::Serialize)]
struct LogBatchEvent<'a> {
    id: &'a str,
    lines: &'a [LogEntryEvent],
}

#[derive(Default)]
struct PendingLogs {
    lines: Vec<LogEntryEvent>,
    preview: Option<LogEntryEvent>,
}

impl PendingLogs {
    fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.preview.is_none()
    }

    fn push(&mut self, entry: LogEntryEvent, console: bool, preview: bool) {
        if preview {
            self.preview = Some(entry.clone());
        } else {
            self.preview = None;
        }
        if console {
            self.lines.push(entry);
        } else {
            self.lines.clear();
        }
    }
}

fn log_consumers(app: Option<&tauri::AppHandle>, id: &str, log_label: &str) -> (bool, bool) {
    app.map_or((false, false), |app| {
        (
            app.get_webview_window(log_label).is_some(),
            wants_log_preview(id) && app.get_webview_window("main").is_some(),
        )
    })
}

fn flush_log_batch(
    app: Option<&tauri::AppHandle>,
    id: &str,
    log_label: &str,
    batch: &mut Vec<LogEntryEvent>,
) {
    if !batch.is_empty()
        && let Some(app) = app
        && app.get_webview_window(log_label).is_some()
    {
        let _ = app.emit_to(
            EventTarget::webview_window(log_label),
            "instance-log-batch",
            LogBatchEvent { id, lines: batch },
        );
    }
    // Keep the allocation for the next batch.
    batch.clear();
}

fn flush_log_preview(
    app: Option<&tauri::AppHandle>,
    id: &str,
    preview: &mut Option<LogEntryEvent>,
) {
    #[derive(Clone, serde::Serialize)]
    struct Preview<'a> {
        id: &'a str,
        line: &'a str,
    }
    if let Some(line) = preview.take()
        && let Some(app) = app
        && wants_log_preview(id)
        && let Some(window) = app.get_webview_window("main")
        && window.is_visible().unwrap_or(false)
        && !window.is_minimized().unwrap_or(false)
    {
        let _ = app.emit_to(
            EventTarget::webview_window("main"),
            "instance-log-preview",
            Preview {
                id,
                line: &line.line,
            },
        );
    }
}

pub(super) fn spawn_io_forwarding(
    app: Option<tauri::AppHandle>,
    id: Arc<str>,
    mut stdout: broadcast::Receiver<String>,
    mut stderr: broadcast::Receiver<String>,
    mut ready: Option<tokio::sync::oneshot::Sender<()>>,
    clean_shutdown: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let ring = get_log_ring(&id);
        let log_label = format!("log-{id}");
        let mut pending = PendingLogs::default();
        let mut stdout_closed = false;
        let mut stderr_closed = false;
        let delay = std::time::Duration::from_millis(80);
        let timer = tokio::time::sleep(delay);
        tokio::pin!(timer);

        while !stdout_closed || !stderr_closed {
            let (line_result, is_stderr) = tokio::select! {
                line = stdout.recv(), if !stdout_closed => (line, false),
                line = stderr.recv(), if !stderr_closed => (line, true),
                _ = &mut timer, if !pending.is_empty() => {
                    flush_log_batch(app.as_ref(), &id, &log_label, &mut pending.lines);
                    flush_log_preview(app.as_ref(), &id, &mut pending.preview);
                    continue;
                }
            };
            let line = match line_result {
                Ok(line) => line,
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    warn!("Se perdieron {count} líneas de logs de {id}");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    if is_stderr {
                        stderr_closed = true;
                    } else {
                        stdout_closed = true;
                    }
                    continue;
                }
            };
            let stripped = strip_ansi(&line);
            if ready.is_some()
                && is_game_ready(&stripped)
                && let Some(tx) = ready.take()
            {
                let _ = tx.send(());
            }
            let cleaned = sanitize_with_user(&stripped);
            if cleaned.is_empty() {
                continue;
            }
            if is_clean_shutdown(&cleaned) {
                clean_shutdown.store(true, Ordering::Release);
            }
            let level = LogLevel::from_line(&cleaned, is_stderr);
            let text: Arc<str> = Arc::from(cleaned);
            let (line_id, timestamp) = ring.push(text.clone(), level, u8::from(is_stderr)).await;
            // Retain sanitized history while the WebView is gone, without building
            // IPC payloads or waking an interval timer for an absent frontend.
            let (console, preview) = log_consumers(app.as_ref(), &id, &log_label);
            if console || preview {
                if pending.is_empty() {
                    timer.as_mut().reset(tokio::time::Instant::now() + delay);
                }
                pending.push(
                    LogEntryEvent {
                        id: line_id,
                        line: text,
                        stream: if is_stderr { "stderr" } else { "stdout" },
                        level,
                        timestamp,
                    },
                    console,
                    preview,
                );
                if pending.lines.len() >= 64 {
                    // Console throughput must not increase header updates above
                    // one preview per 80 ms. Its last line waits for the timer.
                    flush_log_batch(app.as_ref(), &id, &log_label, &mut pending.lines);
                }
            } else {
                pending.lines.clear();
                pending.preview = None;
            }
        }
        flush_log_batch(app.as_ref(), &id, &log_label, &mut pending.lines);
        flush_log_preview(app.as_ref(), &id, &mut pending.preview);
    })
}

pub(super) async fn finish_io_forwarding(mut task: tokio::task::JoinHandle<()>) {
    // A mod can leave a child process holding stdout/stderr open after Java exits.
    // Bound the drain time so that it cannot prevent the launcher from returning.
    match tokio::time::timeout(std::time::Duration::from_secs(2), &mut task).await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => warn!("Error procesando logs del juego: {err}"),
        Err(_) => {
            task.abort();
            let _ = task.await;
            warn!("Tiempo agotado esperando el cierre de los logs del juego");
        }
    }
}

#[cfg(test)]
#[path = "../../tests/services/launcher/logging.rs"]
mod tests;
