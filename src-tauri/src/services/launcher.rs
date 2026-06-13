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
use launchwerk::auth::{AccountType, MinecraftUser, microsoft::MicrosoftAuth};
use launchwerk::models::VersionManifest;
use launchwerk::{LaunchConfig, Launchwerk};
use std::sync::{Arc, OnceLock};
use tauri::Emitter;
use tokio::fs;
use tokio::sync::broadcast;
use tracing::{error, info, trace, warn};

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

        let (java_version, java_path) =
            resolve_java_path(&settings_m, manifest.java_version.as_ref());

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

        let min_mem = format!("{}G", settings_m.min_memory);
        let max_mem = format!("{}G", settings_m.max_memory);

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
                builder = builder.env(k.as_str(), v);
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

fn spawn_io_forwarding(
    app: tauri::AppHandle,
    id: Arc<str>,
    mut rx: broadcast::Receiver<String>,
    stream: &'static str,
) {
    tokio::spawn(async move {
        while let Ok(line) = rx.recv().await {
            if line.to_lowercase().contains("token") {
                continue;
            }
            if app
                .emit(
                    "instance-console-output",
                    serde_json::json!({ "id": id, "line": line, "stream": stream }),
                )
                .is_err()
            {
                break;
            }
        }
    });
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
