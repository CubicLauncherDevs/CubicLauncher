use crate::commands::log_window::open_log_window_for_instance;
use crate::core::event_bus::{AppEvent, emit};
use crate::core::path_manager::PathManager;
use crate::core::{AppError, AuthError, DownloadError, FsError, InstanceError};
use crate::services::SettingsManager;
use crate::services::SettingsSnapshot;
use crate::services::discord_presence;
use crate::services::download_queue::DownloadQueue;
use crate::services::instance_manager::{
    InstanceHandle, InstanceStatus, register_kill_sender, unregister_kill_sender,
};
use crate::services::java_manager::JavaManager;
use aqua::JavaVersion;
use compact_str::{CompactString, ToCompactString};
use launchwerk::auth::{
    AccountType, MinecraftUser,
    microsoft::MicrosoftAuth,
    yggdrasil::{self, YggdrasilAuth},
};
use launchwerk::models::VersionManifest;
use launchwerk::{LaunchConfig, Launchwerk};
use parking_lot::RwLock;
use regex::Regex;
use std::collections::VecDeque;
use std::mem;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::fs;
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace, warn};
use zellkern::Loader;

use dashmap::DashMap;

const LOG_RING_CAPACITY: usize = 5000;

static LINE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) static KEEP_ALIVE: AtomicBool = AtomicBool::new(false);

pub(crate) fn should_keep_alive() -> bool {
    KEEP_ALIVE.load(Ordering::Relaxed)
}

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
    inner: std::sync::Mutex<VecDeque<LogLineRaw>>,
}

impl LogRing {
    fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(VecDeque::with_capacity(LOG_RING_CAPACITY)),
        }
    }

    pub fn push(&self, text: Arc<str>, level: LogLevel, stream: u8) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if guard.len() >= LOG_RING_CAPACITY {
            guard.pop_front();
        }
        guard.push_back(LogLineRaw {
            id: LINE_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            text,
            stream,
            level,
            timestamp: ts,
        });
    }

    pub fn snapshot(&self, limit: Option<usize>) -> Vec<LogLine> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
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

pub fn get_log_history(id: &str, limit: Option<usize>) -> Vec<LogLine> {
    let map = LOG_RINGS.get_or_init(DashMap::new);
    if let Some(entry) = map.get(id) {
        entry.snapshot(limit)
    } else {
        get_crash_log_snapshot(id, limit).unwrap_or_default()
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
pub fn save_crash_log_snapshot(id: &str) {
    let snapshot = CrashLogSnapshot {
        lines: get_log_ring(id).snapshot(None),
        created: std::time::Instant::now(),
    };
    crash_log_snapshots().insert(Arc::from(id), snapshot);
    cleanup_crash_log_snapshots();
}

pub fn clear_crash_log_snapshot(id: &str) {
    crash_log_snapshots().remove(id);
}

fn get_crash_log_snapshot(id: &str, limit: Option<usize>) -> Option<Vec<LogLine>> {
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
pub fn push_launcher_message(id: &str, message: impl Into<String>) {
    let sanitized = sanitize_with_user(&message.into());
    get_log_ring(id).push(Arc::from(sanitized), LogLevel::Launcher, 2);
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
struct LogBatchEvent {
    id: Arc<str>,
    lines: Vec<LogEntryEvent>,
}

// ── Statics ───────────────────────────────────────────────────────────────────

static LAUNCHER: OnceLock<Arc<Launcher>> = OnceLock::new();

// ── Launcher ──────────────────────────────────────────────────────────────────
//
// Solo responsabilidad: lanzar instancias.
// Ya no mezcla la lógica de descargas.

pub struct Launcher {
    app_handle: std::sync::Mutex<Option<tauri::AppHandle>>,
    lw: Launchwerk,
}

impl Launcher {
    pub fn get() -> &'static Arc<Launcher> {
        LAUNCHER
            .get()
            .expect("BUG: Launcher usado antes de inicializar")
    }

    pub fn init() -> Arc<Self> {
        let launcher = Arc::new(Self {
            app_handle: std::sync::Mutex::new(None),
            lw: Launchwerk::new(PathManager::get().get_shared_dir().to_path_buf()),
        });
        let _ = LAUNCHER.set(launcher.clone());
        launcher
    }

    pub fn set_handle(&self, handle: tauri::AppHandle) {
        *self.app_handle.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
    }

    pub async fn launch(&self, handle: InstanceHandle) -> Result<(), AppError> {
        trace!("=== CubicLaunchwerk ===");

        if handle.is_busy() {
            warn!("La instancia ya está corriendo o iniciando");
            return Err(AppError::Instance(InstanceError::AlreadyStarted));
        }
        clear_crash_log_snapshot(&handle.uuid);
        handle.set_status(InstanceStatus::Starting);

        let settings_m = SettingsManager::launch_snapshot();
        let hide_on_launch = SettingsManager::read().hide_on_launch;

        let version = handle.get_version().await;
        let name = handle.get_name().await;
        let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
        let instance_dir = PathManager::get().get_instance_dir().join(name.as_ref());

        if !instance_dir.exists() {
            fs::create_dir(&instance_dir)
                .await
                .map_err(|e| FsError::CreateDir {
                    path: instance_dir.to_string_lossy().to_string(),
                    source: e,
                })?;
        }

        // Si la versión o alguna de sus dependencias no está descargada,
        // encolar la descarga y salir con error descriptivo.
        // El frontend puede escuchar "download-finished" y reintentar el launch
        let deps = zellkern::resolve_dependencies(version.as_ref());
        let missing: Vec<String> = deps
            .iter()
            .filter(|dep| {
                let json_path = shared_dir.join(format!("versions/{dep}/{dep}.json"));
                !json_path.exists()
            })
            .cloned()
            .collect();
        if !missing.is_empty() {
            info!(
                "Faltan dependencias para {}: {:?}, encolando descarga automática...",
                version, missing
            );
            DownloadQueue::get().enqueue(version.clone()).await;
            handle.set_status(InstanceStatus::Off);
            return Err(AppError::Instance(InstanceError::VersionNotFound(
                version.to_string(),
            )));
        }

        let version_json = shared_dir.join(format!("versions/{}/{}.json", version, version));

        let manifest = VersionManifest::from_file(version_json)
            .map_err(|e| DownloadError::ParseJson(e.to_string()))?;
        let mut user = SettingsManager::read().get_user();

        // Recuperar los tokens desde el almacenamiento seguro. Sin esto los
        // campos #[serde(skip)] llegan vacíos tras cargar settings.cub y el
        // refresco de Microsoft nunca se ejecuta.
        if let Err(e) = user.load_tokens() {
            error!("No se pudieron cargar los tokens del usuario: {:?}", e);
            return Err(AppError::Auth(AuthError::AuthFailed(format!(
                "No se pudieron cargar los tokens: {}",
                e
            ))));
        }
        set_user_filter(&user);

        // Resolve java version through inheritsFrom chain (Forge version.json may omit it)
        let mut java_version_req = if manifest.java_version.is_some() {
            manifest.java_version.clone()
        } else {
            manifest.inherits_from.as_ref().and_then(|parent_id| {
                let parent_path = shared_dir.join(format!("versions/{parent_id}/{parent_id}.json"));
                VersionManifest::from_file(parent_path)
                    .ok()
                    .and_then(|p| p.java_version)
            })
        };

        // Forge/NeoForge ModLauncher requires Java 16+; bump if manifest says < 16.
        // But Forge < 36.2.26 bundles ModLauncher 8.0.9 which has a broken
        // ManifestEntryVerifier constructor — only bump for >= 36.2.26.
        let loader = Loader::from_version_id(&manifest.id_raw);
        if let Some(ref mut jv) = java_version_req
            && matches!(loader, Loader::Forge(_) | Loader::NeoForge(_))
            && jv.major_version < 16
        {
            let should_bump = match &loader {
                Loader::Forge(id) => is_forge_version_safe(id),
                Loader::NeoForge(_) => true,
                _ => false,
            };
            if should_bump {
                info!(
                    "Forge/NeoForge detected (Java {} requested), upgrading to Java 17",
                    jv.major_version
                );
                debug!("Forge version safe for Java 17 bump, setting major_version to 17");
                jv.major_version = 17;
            } else {
                info!(
                    "Old Forge detected (Java {} requested), keeping Java 8",
                    jv.major_version
                );
                debug!("Forge version NOT safe for Java 17, staying at Java 8");
            }
        }
        let overrides = handle.get_overrides().await;
        let (java_version, java_path) = if let Some(overrides) = overrides {
            if let Some(java_meta) = overrides.java_version {
                resolve_java_path(
                    &settings_m,
                    Some(&JavaVersion {
                        component: String::new(),
                        major_version: java_meta,
                    }),
                )
            } else {
                resolve_java_path(&settings_m, java_version_req.as_ref())
            }
        } else {
            resolve_java_path(&settings_m, java_version_req.as_ref())
        };

        if !java_path.exists() {
            let reason = InstanceError::JreNotFound(java_version.to_string()).to_string();
            handle.set_status(InstanceStatus::Error(reason.clone()));
            emit(AppEvent::InstanceCrashed {
                id: handle.uuid.to_compact_string(),
                name: name.to_compact_string(),
                exit_code: None,
                reason: Some(CompactString::new(reason.clone())),
            });
            let app_handle = self
                .app_handle
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            if let Some(app) = app_handle {
                let instance_id = handle.uuid.to_compact_string().to_string();
                let instance_name = name.to_compact_string().to_string();
                tokio::spawn(async move {
                    if let Err(err) =
                        open_log_window_for_instance(app, instance_id, instance_name).await
                    {
                        warn!("No se pudo abrir ventana de logs: {}", err);
                    }
                });
            }
            Err(AppError::Instance(InstanceError::JreNotFound(
                java_version.to_string(),
            )))?;
        }
        info!("Starting with {}", java_path.display());
        // Auto-refresh del token Microsoft — el lock de settings se toma y suelta rápido
        user = refresh_microsoft_token(user).await?;

        // Auto-refresh del token Yggdrasil
        user = refresh_yggdrasil_token(user).await?;

        // let min_mem = format!("{}G", settings_m.min_memory);
        // let max_mem = format!("{}G", settings_m.max_memory);

        let (max_mem, min_mem) = if let Some(ram_overrides) = overrides.and_then(|o| o.memory) {
            let max_memf = format!("{}M", ram_overrides.max_mem);
            let min_memf = format!("{}M", ram_overrides.min_mem);
            (max_memf, min_memf)
        } else {
            let min_memf = format!("{}M", settings_m.min_memory);
            let max_memf = format!("{}M", settings_m.max_memory);
            (max_memf, min_memf)
        };

        // Datos que necesitamos mostrar en el log antes de mover los originales.
        let java_path_str = java_path.display().to_string();
        let min_mem_str = min_mem.clone();
        let max_mem_str = max_mem.clone();

        let mut builder = LaunchConfig::builder()
            .java_path(java_path)
            .username(user.username)
            .ram(min_mem, max_mem)
            .cracked(user.user_type == AccountType::Cracked);

        let mut extra_jvm_args: Vec<String> = Vec::new();

        match user.user_type {
            AccountType::Microsoft => {
                builder = builder
                    .access_token(user.access_token)
                    .auth_uuid(user.uuid)
                    .user_type("msa");
            }
            AccountType::Yggdrasil => {
                builder = builder
                    .access_token(user.access_token)
                    .auth_uuid(user.uuid)
                    .user_type("mojang");

                // Download authlib-injector and fetch metadata for prefetch
                if let Some(ref server_url) = user.yggdrasil_server_url {
                    match yggdrasil::download_authlib_injector(&shared_dir).await {
                        Ok(jar_path) => {
                            // Fetch metadata and base64 encode for prefetch
                            let ygg_auth = YggdrasilAuth::new();
                            let api_root = ygg_auth
                                .resolve_api_url(server_url)
                                .await
                                .unwrap_or_else(|_| server_url.clone());
                            match yggdrasil::fetch_metadata_prefetch(&api_root).await {
                                Ok(metadata_b64) => {
                                    let agent_arg =
                                        format!("-javaagent:{}={}", jar_path.display(), api_root);
                                    builder = builder
                                        .authlib_injector_path(jar_path)
                                        .yggdrasil_metadata_b64(metadata_b64);
                                    extra_jvm_args.push(agent_arg);
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to fetch Yggdrasil metadata for prefetch: {}. Launching without prefetch.",
                                        e
                                    );
                                    let agent_arg =
                                        format!("-javaagent:{}={}", jar_path.display(), server_url);
                                    builder = builder.authlib_injector_path(jar_path);
                                    extra_jvm_args.push(agent_arg);
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to download authlib-injector: {}. Launching without it.",
                                e
                            );
                        }
                    }
                }
            }
            AccountType::Cracked => {}
        }

        for (k, v) in &settings_m.env_vars {
            if !k.is_empty() {
                builder = builder.env(k.as_str(), v);
            }
        }

        let parsed_jvm_args: Vec<String> = settings_m
            .jvm_args
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        extra_jvm_args.extend(parsed_jvm_args);
        builder = builder.extra_jvm_args(extra_jvm_args);

        let instance_name = name.clone();
        let instance_version = version.clone();
        push_launcher_message(
            &handle.uuid,
            format!(
                "Iniciando instancia \"{}\" ({})",
                instance_name, instance_version
            ),
        );
        push_launcher_message(
            &handle.uuid,
            format!("Java: {} (versión {})", java_path_str, java_version),
        );
        push_launcher_message(
            &handle.uuid,
            format!("Memoria: {} / {}", min_mem_str, max_mem_str),
        );

        let options = builder.build();

        let lw_handle = self.lw.prepare(manifest, options, instance_dir);
        handle.update_last_played().await;

        match lw_handle.launch().await {
            Ok(_) => {
                info!("Handle {} lanzado", lw_handle.id().to_string());
                handle.set_status(InstanceStatus::Started);

                let loader = handle.to_dto().await.loader;
                discord_presence::on_instance_start(
                    instance_name.to_string(),
                    instance_version.to_string(),
                    loader.into_owned(),
                )
                .await;

                let app_handle = self
                    .app_handle
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if let Some(ref app) = app_handle {
                    let id = handle.uuid.clone();
                    push_launcher_message(&id, "Proceso iniciado");
                    let stdout_rx = lw_handle.subscribe_stdout();
                    let stderr_rx = lw_handle.subscribe_stderr();
                    spawn_io_forwarding(app.clone(), id.clone(), stdout_rx, "stdout");
                    spawn_io_forwarding(app.clone(), id, stderr_rx, "stderr");
                    if hide_on_launch {
                        KEEP_ALIVE.store(true, Ordering::Relaxed);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.close();
                        }
                    }
                } else {
                    warn!("AppHandle no disponible, no se reenviará stdout/stderr");
                }

                let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();
                register_kill_sender(&handle.uuid, kill_tx);

                let uuid = handle.uuid.clone();
                let h = handle.clone();
                let inst_name = instance_name.clone();
                let app_for_show = app_handle.clone();
                tokio::spawn(async move {
                    let result = tokio::select! {
                        _ = kill_rx => {
                            info!("Kill signal received for {}", uuid);
                            if let Err(e) = lw_handle.kill().await {
                                warn!("Error al matar proceso {}: {:?}", uuid, e);
                            }
                            lw_handle.wait().await
                        }
                        result = lw_handle.wait() => {
                            info!("Instance {} exited: {:?}", uuid, result);
                            result
                        }
                    };
                    unregister_kill_sender(&uuid);
                    push_launcher_message(&uuid, format!("El proceso terminó: {:?}", result));

                    let crashed = matches!(result, Some(code) if code != 0);
                    if crashed {
                        let code = result.unwrap_or(-1);
                        push_launcher_message(&uuid, format!("Crash detectado (código {})", code));
                        save_crash_log_snapshot(&uuid);
                        emit(AppEvent::InstanceCrashed {
                            id: uuid.to_compact_string(),
                            name: inst_name.to_compact_string(),
                            exit_code: result,
                            reason: Some(CompactString::new(format!(
                                "El proceso terminó con código {}",
                                code
                            ))),
                        });
                        if let Some(app) = app_for_show.clone() {
                            let instance_id = uuid.to_compact_string().to_string();
                            let instance_name = inst_name.to_compact_string().to_string();
                            tokio::spawn(async move {
                                if let Err(err) =
                                    open_log_window_for_instance(app, instance_id, instance_name)
                                        .await
                                {
                                    warn!("No se pudo abrir ventana de logs: {}", err);
                                }
                            });
                        }
                    }

                    discord_presence::on_instance_stop(&inst_name).await;
                    remove_log_ring(&uuid);
                    h.set_status(InstanceStatus::Off);
                    // Si se cerró la ventana al lanzar el juego, la recreamos
                    if hide_on_launch {
                        KEEP_ALIVE.store(false, Ordering::Relaxed);
                        if let Some(app) = app_for_show {
                            let _ = WebviewWindowBuilder::new(
                                &app,
                                "main",
                                WebviewUrl::App("index.html".into()),
                            )
                            .title("CubicLauncher @32")
                            .inner_size(800.0, 600.0)
                            .min_inner_size(800.0, 600.0)
                            .build();
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.set_focus();
                            }
                        }
                    }
                });
            }
            Err(e) => {
                let msg = e.to_string();
                error!("{}", msg);
                push_launcher_message(&handle.uuid, format!("Error al iniciar: {}", msg));
                handle.set_status(InstanceStatus::Error(msg.clone()));
                emit(AppEvent::InstanceCrashed {
                    id: handle.uuid.to_compact_string(),
                    name: name.to_compact_string(),
                    exit_code: None,
                    reason: Some(CompactString::new(msg.clone())),
                });
                let app_handle = self
                    .app_handle
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if let Some(app) = app_handle {
                    let instance_id = handle.uuid.to_compact_string().to_string();
                    let instance_name = name.to_compact_string().to_string();
                    tokio::spawn(async move {
                        if let Err(err) =
                            open_log_window_for_instance(app, instance_id, instance_name).await
                        {
                            warn!("No se pudo abrir ventana de logs: {}", err);
                        }
                    });
                }
            }
        }
        Ok(())
    }
}

async fn refresh_microsoft_token(mut user: MinecraftUser) -> Result<MinecraftUser, AppError> {
    if user.user_type != AccountType::Microsoft {
        return Ok(user);
    }

    // Defensa: si el refresh_token no está cargado (p. ej. llamada desde otro
    // sitio), intentar recuperarlo desde el almacenamiento seguro.
    if user.refresh_token.is_none()
        && let Err(e) = user.load_tokens()
    {
        warn!("No se pudieron cargar tokens de Microsoft: {:?}", e);
    }

    let Some(refresh_token) = user.refresh_token.clone() else {
        warn!(
            "No hay refresh token de Microsoft para {}. Continuando con el token actual.",
            user.username
        );
        return Ok(user);
    };

    info!("Refrescando token de Microsoft...");
    let refresh_result = tokio::task::spawn_blocking(move || {
        MicrosoftAuth::default()
            .refresh_token(&refresh_token)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| AuthError::AuthFailed(e.to_string()))?;

    match refresh_result {
        Ok(new_user) => {
            info!("Token refrescado para {}", new_user.username);
            user = new_user;
            if let Err(e) = user.save_tokens() {
                warn!("Error guardando tokens: {:?}", e);
            }
            SettingsManager::write(|settings| {
                settings.set_user_by_uuid(&user);
            })?;
            SettingsManager::save().await?;
            Ok(user)
        }
        Err(e) => {
            error!("No se pudo refrescar el token de Microsoft: {}", e);
            Err(AppError::Auth(AuthError::SessionExpired(format!(
                "El token de Microsoft no pudo refrescarse: {}. Vuelve a iniciar sesión.",
                e
            ))))
        }
    }
}

async fn refresh_yggdrasil_token(mut user: MinecraftUser) -> Result<MinecraftUser, AppError> {
    if user.user_type != AccountType::Yggdrasil {
        return Ok(user);
    }

    let server_url = match &user.yggdrasil_server_url {
        Some(url) => url.clone(),
        None => {
            warn!("URL del servidor Yggdrasil no configurada, no se puede refrescar token");
            return Ok(user);
        }
    };

    // Load tokens from secure storage
    if let Err(e) = user.load_tokens() {
        warn!("Error cargando tokens Yggdrasil: {:?}", e);
        return Ok(user);
    }

    let client_token = user
        .client_token
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    info!("Validando token Yggdrasil...");
    let auth = YggdrasilAuth::new();
    let valid = auth
        .validate(&server_url, &user.access_token, &client_token)
        .await;

    if valid {
        info!("Token Yggdrasil válido");
        return Ok(user);
    }

    info!("Token Yggdrasil inválido, intentando refresh...");
    let refresh_result = auth
        .refresh(
            &server_url,
            &user.access_token,
            &client_token,
            &user.uuid,
            &user.username,
        )
        .await;

    match refresh_result {
        Ok(refreshed) => {
            info!("Token Yggdrasil refrescado para {}", refreshed.username);
            user = refreshed;
            user.yggdrasil_server_url = Some(server_url);
            if let Err(e) = user.save_tokens() {
                warn!("Error guardando tokens Yggdrasil: {:?}", e);
            }
            SettingsManager::write(|settings| {
                settings.set_user_by_uuid(&user);
            })?;
            SettingsManager::save().await?;
            Ok(user)
        }
        Err(e) => {
            error!("No se pudo refrescar token Yggdrasil: {}", e);
            Err(AppError::Auth(AuthError::SessionExpired(format!(
                "El token Yggdrasil no es válido y no pudo refrescarse: {}. Vuelve a iniciar sesión.",
                e
            ))))
        }
    }
}

fn spawn_io_forwarding(
    app: tauri::AppHandle,
    id: Arc<str>,
    mut rx: broadcast::Receiver<String>,
    stream: &'static str,
) {
    tokio::spawn(async move {
        let ring = get_log_ring(&id);
        let stderr = stream == "stderr";
        let stream_id: u8 = if stderr { 1 } else { 0 };
        let stream_name = stream;
        let mut batch: Vec<LogEntryEvent> = Vec::with_capacity(64);
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(80));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                line_result = rx.recv() => {
                    match line_result {
                        Ok(line) => {
                            let cleaned = sanitize_with_user(&strip_ansi(&line));
                            if cleaned.is_empty() {
                                continue;
                            }
                            let level = LogLevel::from_line(&cleaned, stderr);
                            let ts = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u64)
                                .unwrap_or(0);
                            let text: Arc<str> = Arc::from(cleaned);
                            let id_for_line = LINE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
                            ring.push(text.clone(), level, stream_id);
                            batch.push(LogEntryEvent {
                                id: id_for_line,
                                line: text,
                                stream: stream_name,
                                level,
                                timestamp: ts,
                            });
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = interval.tick() => {
                    if !batch.is_empty() {
                        let lines: Vec<LogEntryEvent> = mem::take(&mut batch);
                        let _ = app.emit(
                            "instance-log-batch",
                            LogBatchEvent { id: id.clone(), lines },
                        );
                    }
                }
            }
        }
        if !batch.is_empty() {
            let lines: Vec<LogEntryEvent> = mem::take(&mut batch);
            let _ = app.emit(
                "instance-log-batch",
                LogBatchEvent {
                    id: id.clone(),
                    lines,
                },
            );
        }
    });
}

/// Returns true if the Forge version indicates a version >= 36.2.26
/// (which bundles ModLauncher 8.1.3+ with the fixed ManifestEntryVerifier).
/// Note: `forge_ver` is just the forge version string (e.g. "14.22.1.2485"),
/// extracted by `Loader::from_version_id()`.
fn is_forge_version_safe(forge_ver: &str) -> bool {
    let parts: Vec<u32> = forge_ver
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    let safe = match parts.as_slice() {
        [major, minor, patch, ..] => (*major, *minor, *patch) >= (36, 2, 26),
        _ => false,
    };
    debug!("Forge version safety check: forge_ver={forge_ver}, parts={parts:?}, safe={safe}");
    safe
}

fn resolve_java_path(
    settings: &SettingsSnapshot,
    java_version: Option<&launchwerk::models::JavaVersion>,
) -> (u8, std::path::PathBuf) {
    let version = match java_version {
        Some(v) => v.major_version,
        None => 25,
    };
    match version {
        8 if settings.jre8_managed && JavaManager::is_installed(8) => {
            (8, JavaManager::get_java_binary(8))
        }
        17 if settings.jre17_managed && JavaManager::is_installed(17) => {
            (17, JavaManager::get_java_binary(17))
        }
        21 if settings.jre21_managed && JavaManager::is_installed(21) => {
            (21, JavaManager::get_java_binary(21))
        }
        25 if settings.jre25_managed && JavaManager::is_installed(25) => {
            (25, JavaManager::get_java_binary(25))
        }
        8 => (8, settings.get_jre8_path().to_path_buf()),
        17 => (17, settings.get_jre17_path().to_path_buf()),
        21 => (21, settings.get_jre21_path().to_path_buf()),
        25 => (25, settings.get_jre25_path().to_path_buf()),
        _ => {
            if settings.jre21_managed && JavaManager::is_installed(21) {
                (21, JavaManager::get_java_binary(21))
            } else {
                (21, settings.get_jre21_path().to_path_buf())
            }
        }
    }
}
