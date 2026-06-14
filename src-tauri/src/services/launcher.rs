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
use launchwerk::auth::{AccountType, MinecraftUser, microsoft::MicrosoftAuth, yggdrasil::{self, YggdrasilAuth}};
use launchwerk::models::VersionManifest;
use zellkern::Loader;
use launchwerk::{LaunchConfig, Launchwerk};
use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};
use tauri::Emitter;
use tokio::fs;
use tokio::sync::broadcast;
use tracing::{error, info, trace, warn};

use dashmap::DashMap;

const LOG_RING_CAPACITY: usize = 5000;

// ── Log Ring Buffer ─────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize)]
pub struct LogLine {
    pub text: String,
    pub stream: String,
    pub timestamp: u64,
}

struct LogLineRaw {
    text: String,
    stream: u8,
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

    pub fn push(&self, text: String, stream: u8) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if guard.len() >= LOG_RING_CAPACITY {
            guard.pop_front();
        }
        guard.push_back(LogLineRaw {
            text,
            stream,
            timestamp: ts,
        });
    }

    pub fn drain(&self) -> Vec<LogLine> {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .drain(..)
            .map(|raw| LogLine {
                text: raw.text,
                stream: match raw.stream {
                    0 => "stdout".into(),
                    _ => "stderr".into(),
                },
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

pub fn get_log_history(id: &str) -> Vec<LogLine> {
    get_log_ring(id).drain()
}

pub fn remove_log_ring(id: &str) {
    if let Some(map) = LOG_RINGS.get() {
        map.remove(id);
    }
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
        handle.set_status(InstanceStatus::Starting);

        let settings_m = SettingsManager::launch_snapshot();

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

        // Si la versión no está descargada, encolarla y salir con error descriptivo
        // El frontend puede escuchar "download-finished" y reintentar el launch
        let version_json = shared_dir.join(format!("versions/{}/{}.json", version, version));
        if !version_json.exists() {
            info!(
                "Versión {} no descargada, encolando descarga automática...",
                version
            );
            DownloadQueue::get().enqueue(version.clone()).await;
            handle.set_status(InstanceStatus::Off);
            return Err(AppError::Instance(InstanceError::NotFound));
        }

        let manifest = VersionManifest::from_file(version_json)
            .map_err(|e| DownloadError::ParseJson(e.to_string()))?;
        let mut user = SettingsManager::read().get_user();

        // Resolve java version through inheritsFrom chain (Forge version.json may omit it)
        let mut java_version_req = if manifest.java_version.is_some() {
            manifest.java_version.clone()
        } else {
            manifest.inherits_from.as_ref().and_then(|parent_id| {
                let parent_path =
                    shared_dir.join(format!("versions/{parent_id}/{parent_id}.json"));
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
                jv.major_version = 17;
            } else {
                info!(
                    "Old Forge detected (Java {} requested), keeping Java 8 (ModLauncher < 8.1 incompatible with Java 17+)",
                    jv.major_version
                );
            }
        }

        let (java_version, java_path) =
            resolve_java_path(&settings_m, java_version_req.as_ref());

        if !java_path.exists() {
            handle.set_status(InstanceStatus::Error(
                InstanceError::JreNotFound(java_version.to_string()).to_string(),
            ));
            return Err(AppError::Instance(InstanceError::JreNotFound(
                java_version.to_string(),
            )))?;
        }

        // Auto-refresh del token Microsoft — el lock de settings se toma y suelta rápido
        user = refresh_microsoft_token(user).await?;

        // Auto-refresh del token Yggdrasil
        user = refresh_yggdrasil_token(user).await?;

        let min_mem = format!("{}G", settings_m.min_memory);
        let max_mem = format!("{}G", settings_m.max_memory);

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
                                    let agent_arg = format!(
                                        "-javaagent:{}={}",
                                        jar_path.display(),
                                        api_root
                                    );
                                    builder = builder
                                        .authlib_injector_path(jar_path)
                                        .yggdrasil_metadata_b64(metadata_b64);
                                    extra_jvm_args.push(agent_arg);
                                }
                                Err(e) => {
                                    warn!("Failed to fetch Yggdrasil metadata for prefetch: {}. Launching without prefetch.", e);
                                    let agent_arg = format!(
                                        "-javaagent:{}={}",
                                        jar_path.display(),
                                        server_url
                                    );
                                    builder = builder
                                        .authlib_injector_path(jar_path);
                                    extra_jvm_args.push(agent_arg);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to download authlib-injector: {}. Launching without it.", e);
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

        let options = builder.build();

        let lw_handle = self.lw.prepare(manifest, options, instance_dir);
        handle.update_last_played().await;
        let instance_name = name.clone();
        let instance_version = version.clone();

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

                {
                    let guard = self.app_handle.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(ref app) = *guard {
                        let id = handle.uuid.clone();
                        let stdout_rx = lw_handle.subscribe_stdout();
                        let stderr_rx = lw_handle.subscribe_stderr();
                        spawn_io_forwarding(app.clone(), id.clone(), stdout_rx, "stdout");
                        spawn_io_forwarding(app.clone(), id, stderr_rx, "stderr");
                    } else {
                        warn!("AppHandle no disponible, no se reenviará stdout/stderr");
                    }
                }

                let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();
                register_kill_sender(&handle.uuid, kill_tx);

                let uuid = handle.uuid.clone();
                let h = handle.clone();
                let inst_name = instance_name.clone();
                tokio::spawn(async move {
                    tokio::select! {
                        _ = kill_rx => {
                            info!("Kill signal received for {}", uuid);
                            if let Err(e) = lw_handle.kill().await {
                                warn!("Error al matar proceso {}: {:?}", uuid, e);
                            }
                            lw_handle.wait().await;
                        }
                        result = lw_handle.wait() => {
                            info!("Instance {} exited: {:?}", uuid, result);
                            unregister_kill_sender(&uuid);
                        }
                    }
                    discord_presence::on_instance_stop(&inst_name).await;
                    remove_log_ring(&uuid);
                    h.set_status(InstanceStatus::Off);
                });
            }
            Err(e) => {
                error!("{}", e.to_string());
                handle.set_status(InstanceStatus::Error(e.to_string()));
            }
        }
        Ok(())
    }
}

async fn refresh_microsoft_token(mut user: MinecraftUser) -> Result<MinecraftUser, AppError> {
    if user.user_type == AccountType::Microsoft
        && let Some(refresh_token) = &user.refresh_token
    {
        info!("Refrescando token de Microsoft...");
        let rt = refresh_token.clone();
        let refresh_result = tokio::task::spawn_blocking(move || {
            MicrosoftAuth::default()
                .refresh_token(&rt)
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
                    settings.set_user(user.clone());
                })?;
                SettingsManager::save().await?;
            }
            Err(e) => {
                warn!(
                    "No se pudo refrescar el token: {}. Continuando con el actual...",
                    e
                );
            }
        }
    }
    Ok(user)
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
                settings.set_user(user.clone());
            })?;
            SettingsManager::save().await?;
        }
        Err(e) => {
            warn!(
                "No se pudo refrescar token Yggdrasil: {}. Continuando con el actual...",
                e
            );
        }
    }

    Ok(user)
}

fn spawn_io_forwarding(
    app: tauri::AppHandle,
    id: Arc<str>,
    mut rx: broadcast::Receiver<String>,
    stream: &'static str,
) {
    tokio::spawn(async move {
        let ring = get_log_ring(&id);
        let stream_id: u8 = if stream == "stderr" { 1 } else { 0 };
        let stream_name = stream;
        let mut batch: Vec<serde_json::Value> = Vec::with_capacity(64);
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(80));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                line_result = rx.recv() => {
                    match line_result {
                        Ok(line) => {
                            if line.to_lowercase().contains("token") {
                                continue;
                            }
                            let ts = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u64)
                                .unwrap_or(0);
                            ring.push(line.clone(), stream_id);
                            batch.push(serde_json::json!({
                                "line": line,
                                "stream": stream_name,
                                "timestamp": ts,
                            }));
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = interval.tick() => {
                    if !batch.is_empty() {
                        let lines: Vec<serde_json::Value> = batch.drain(..).collect();
                        let _ = app.emit(
                            "instance-log-batch",
                            serde_json::json!({ "id": id, "lines": lines }),
                        );
                    }
                }
            }
        }
        if !batch.is_empty() {
            let lines: Vec<serde_json::Value> = batch.drain(..).collect();
            let _ = app.emit(
                "instance-log-batch",
                serde_json::json!({ "id": id, "lines": lines }),
            );
        }
    });
}

/// Returns true if the Forge version ID indicates a version >= 36.2.26
/// (which bundles ModLauncher 8.1.3+ with the fixed ManifestEntryVerifier).
fn is_forge_version_safe(version_id: &str) -> bool {
    let Some(idx) = version_id.rfind("-forge-") else {
        return true;
    };
    let forge_ver = &version_id[idx + 7..];
    let parts: Vec<u32> = forge_ver
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    match parts.as_slice() {
        [major, minor, patch, ..] => {
            (*major, *minor, *patch) > (36, 2, 25)
        }
        _ => true,
    }
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
