use crate::core::InstanceError::JreDoesntExists;
use crate::core::path_manager::PathManager;
use crate::core::{AppError, AppEvent, AuthError, DownloadError, FsError, InstanceError, emit};
use crate::services::SettingsManager;
use crate::services::discord_presence;
use crate::services::instance_manager::{
    InstanceHandle, InstanceStatus, register_kill_sender, unregister_kill_sender,
};
use crate::services::java_manager::JavaManager;
use aqua::{DownloadManager, DownloadProgress};
use dashmap::DashMap;
use launchwerk::models::VersionManifest;
use launchwerk::{LaunchConfig, Launchwerk};
use launchwerk::{auth::AccountType, auth::microsoft::MicrosoftAuth};
use std::sync::{Arc, OnceLock};
use tauri::Emitter;
use tokio::fs;
use tokio::sync::mpsc;
use tracing::{error, info, trace, warn};
// ── Statics ───────────────────────────────────────────────────────────────────

static LAUNCHER: OnceLock<Arc<Launcher>> = OnceLock::new();
static DOWNLOAD_QUEUE: OnceLock<Arc<DownloadQueue>> = OnceLock::new();

// ── DownloadStatus ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Done,
    Error(String),
}

// ── DownloadState ────────────────────────────────────────────────────────────
// Estado plano de una descarga. Sin atomics — todo se maneja desde el worker.

#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadState {
    pub version: String,
    pub status: DownloadStatus,
    pub current: u64,
    pub total: u64,
}

impl DownloadState {
    fn new(version: String) -> Self {
        Self {
            version,
            status: DownloadStatus::Pending,
            current: 0,
            total: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            DownloadStatus::Pending | DownloadStatus::Downloading
        )
    }
}

// ── DownloadQueue ─────────────────────────────────────────────────────────────

pub struct DownloadQueue {
    sender: mpsc::Sender<String>,
    active: DashMap<String, DownloadState>,
}

impl DownloadQueue {
    pub fn get() -> &'static Arc<DownloadQueue> {
        DOWNLOAD_QUEUE
            .get()
            .expect("BUG: DownloadQueue usado antes de inicializar")
    }

    pub async fn init(_app_handle: Option<tauri::AppHandle>) -> Arc<Self> {
        let (tx, rx) = mpsc::channel::<String>(64);

        let queue = Arc::new(Self {
            sender: tx,
            active: DashMap::new(),
        });

        let queue_clone = queue.clone();
        tokio::spawn(async move {
            Self::worker(rx, queue_clone).await;
        });

        let _ = DOWNLOAD_QUEUE.set(queue.clone());
        queue
    }

    pub async fn enqueue(&self, version: String) {
        if let Some(state) = self.active.get(&version)
            && state.is_active()
        {
            return;
        }

        info!("{} encolada", &version);

        self.active
            .insert(version.clone(), DownloadState::new(version.clone()));

        emit(AppEvent::DEnqueue {
            version: version.clone(),
        });

        if let Err(e) = self.sender.send(version).await {
            error!("Error al encolar descarga: {}", e);
        }
    }

    pub async fn get_active_downloads(&self) -> Vec<DownloadState> {
        self.active
            .iter()
            .filter(|r| r.value().is_active())
            .map(|r| r.value().clone())
            .collect()
    }

    // ── Worker ────────────────────────────────────────────────────────────────

    async fn worker(mut rx: mpsc::Receiver<String>, queue: Arc<DownloadQueue>) {
        while let Some(version) = rx.recv().await {
            let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
            let manager = DownloadManager::new(shared_dir);

            if let Some(mut state) = queue.active.get_mut(&version) {
                state.status = DownloadStatus::Downloading;
            } else {
                error!("State no encontrado para {}, saltando", version);
                continue;
            }

            let download_handle = match manager.prepare(&version).await {
                Ok(h) => h,
                Err(_) if version.starts_with("fabric-loader-") => {
                    let gv = version.split('-').next_back().unwrap_or("");
                    match manager.prepare(gv).await {
                        Ok(h) => h,
                        Err(_) => {
                            let msg = format!("No se pudo resolver base {} para Fabric", gv);
                            error!("{}", msg);
                            if let Some(mut state) = queue.active.get_mut(&version) {
                                state.status = DownloadStatus::Error(msg);
                            }
                            continue;
                        }
                    }
                }
                Err(_) => {
                    let msg = format!("La versión solicitada no existe: {}", version);
                    error!("{}", msg);
                    if let Some(mut state) = queue.active.get_mut(&version) {
                        state.status = DownloadStatus::Error(msg);
                    }
                    continue;
                }
            };

            let (tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);

            // Monitor inline con join! — sin spawn extra
            let monitor = async {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(75));
                interval.tick().await;
                let mut latest: Option<DownloadProgress> = None;

                loop {
                    tokio::select! {
                        biased;
                        _ = interval.tick() => {
                            if let Some(ref p) = latest {
                                emit(AppEvent::DProgress {
                                    version: version.clone(),
                                    current: p.current as u32,
                                    total: p.total as u32,
                                    d_type: format!("{:?}", p.download_type),
                                });
                            }
                        }
                        maybe = progress_rx.recv() => {
                            match maybe {
                                Some(progress) => {
                                    if let Some(mut state) = queue.active.get_mut(&version) {
                                        state.current = progress.current as u64;
                                        state.total = progress.total as u64;
                                    }
                                    latest = Some(progress);
                                }
                                None => break,
                            }
                        }
                    }
                }

                if let Some(p) = latest {
                    emit(AppEvent::DProgress {
                        version: version.clone(),
                        current: p.current as u32,
                        total: p.total as u32,
                        d_type: format!("{:?}", p.download_type),
                    });
                }
            };

            let (dl_result, ()) = tokio::join!(download_handle.download_all(Some(tx)), monitor);

            match dl_result {
                Ok(_) => {
                    info!("Versión {} descargada correctamente", version);
                    if let Some(mut state) = queue.active.get_mut(&version) {
                        state.status = DownloadStatus::Done;
                    }
                    emit(AppEvent::DFinish { version });
                }
                Err(e) => {
                    let msg = format!("No se pudo descargar {}: {:?}", version, e);
                    error!("{}", msg);
                    if let Some(mut state) = queue.active.get_mut(&version) {
                        state.status = DownloadStatus::Error(msg);
                    }
                }
            }

            queue.active.retain(|_, s| s.is_active());
        }

        error!("Worker de descargas terminó inesperadamente — el channel fue cerrado");
    }
}

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
            let msg = "La instancia ya está corriendo o iniciando".to_string();
            warn!("{}", msg);
            return Err(AppError::Instance(InstanceError::AlreadyStarted));
        }
        handle.set_status(InstanceStatus::Starting);

        let settings_m = SettingsManager::snapshot();

        let version = handle.get_version().await;
        let name = handle.get_name().await;
        let shared_dir = PathManager::get().get_shared_dir().to_path_buf();
        let instance_dir = PathManager::get().get_instance_dir().join(&name);

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
        let mut user = SettingsManager::read().get_minecraft_user();

        let java_path = resolve_java_path(&settings_m, manifest.java_version.as_ref());

        if !java_path.exists() {
            handle.set_status(InstanceStatus::Error(
                JreDoesntExists(manifest.java_major_version().to_string()).to_string(),
            ));
            return Err(AppError::Instance(InstanceError::JreDoesntExists(
                manifest.java_major_version().to_string(),
            )))?;
        }

        // Auto-refresh del token Microsoft — el lock de settings se toma y suelta rápido
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
                        settings.set_user(Some(user.clone()));
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

        let min_mem = format!("{}G", settings_m.get_min_memory());
        let max_mem = format!("{}G", settings_m.get_max_memory());

        let mut builder = LaunchConfig::builder()
            .java_path(java_path)
            .username(user.username)
            .ram(min_mem, max_mem)
            .cracked(user.user_type != AccountType::Microsoft);

        if user.user_type == AccountType::Microsoft {
            builder = builder
                .access_token(user.access_token)
                .auth_uuid(user.uuid)
                .user_type("msa");
        }

        for (k, v) in &settings_m.env_vars {
            if !k.is_empty() {
                builder = builder.env(k, v);
            }
        }

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
                    instance_name.clone(),
                    instance_version.clone(),
                    loader,
                )
                .await;

                {
                    let guard = self.app_handle.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(ref app) = *guard {
                        let stdout_rx = lw_handle.subscribe_stdout();
                        let stderr_rx = lw_handle.subscribe_stderr();
                        let id = handle.uuid.clone();

                        let app_stdout = app.clone();
                        let id_stdout = id.clone();
                        tokio::spawn(async move {
                            let mut rx = stdout_rx;
                            while let Ok(line) = rx.recv().await {
                                if line.to_lowercase().contains("token") {
                                    continue;
                                }
                                if app_stdout
                                    .emit(
                                        "instance-console-output",
                                        serde_json::json!({
                                            "id": id_stdout,
                                            "line": line,
                                            "stream": "stdout"
                                        }),
                                    )
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        });

                        let app_stderr = app.clone();
                        tokio::spawn(async move {
                            let mut rx = stderr_rx;
                            while let Ok(line) = rx.recv().await {
                                if line.to_lowercase().contains("token") {
                                    continue;
                                }
                                if app_stderr
                                    .emit(
                                        "instance-console-output",
                                        serde_json::json!({
                                            "id": id,
                                            "line": line,
                                            "stream": "stderr"
                                        }),
                                    )
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        });
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

fn resolve_java_path(
    settings: &SettingsManager,
    java_version: Option<&launchwerk::models::JavaVersion>,
) -> std::path::PathBuf {
    let version = match java_version {
        Some(ref v) => v.major_version,
        None => 25,
    };

    match version {
        8 if settings.jre8_managed && JavaManager::is_installed(8) => {
            JavaManager::get_java_binary(8)
        }
        17 if settings.jre17_managed && JavaManager::is_installed(17) => {
            JavaManager::get_java_binary(17)
        }
        21 if settings.jre21_managed && JavaManager::is_installed(21) => {
            JavaManager::get_java_binary(21)
        }
        25 if settings.jre25_managed && JavaManager::is_installed(25) => {
            JavaManager::get_java_binary(25)
        }
        8 => settings.get_jre8_path().to_path_buf(),
        17 => settings.get_jre17_path().to_path_buf(),
        21 => settings.get_jre21_path().to_path_buf(),
        25 => settings.get_jre25_path().to_path_buf(),
        _ => {
            if settings.jre21_managed && JavaManager::is_installed(21) {
                JavaManager::get_java_binary(21)
            } else {
                settings.get_jre21_path().to_path_buf()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_state_pending() {
        let s = DownloadState::new("1.21".into());
        assert_eq!(s.status, DownloadStatus::Pending);
        assert!(s.is_active());
    }

    #[test]
    fn test_download_state_not_active_done() {
        let mut s = DownloadState::new("1.21".into());
        s.status = DownloadStatus::Done;
        assert!(!s.is_active());
    }

    #[test]
    fn test_download_state_not_active_error() {
        let mut s = DownloadState::new("1.21".into());
        s.status = DownloadStatus::Error("err".into());
        assert!(!s.is_active());
    }
}
