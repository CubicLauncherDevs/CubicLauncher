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
use crate::services::launch_window::LaunchSession;
use aqua::JavaVersion;
use compact_str::{CompactString, ToCompactString};
use launchwerk::auth::{
    AccountType, MinecraftUser,
    microsoft::MicrosoftAuth,
    yggdrasil::{self, YggdrasilAuth},
};
use launchwerk::models::VersionManifest;
use launchwerk::{LaunchConfig, Launchwerk};
use regex::Regex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::fs;
use tracing::{debug, error, info, trace, warn};
use zellkern::{GameVersion, Loader};

mod logging;
use logging::set_user_filter;
pub use logging::{
    LogLine, clear_crash_log_snapshot, get_log_history, push_launcher_message, remove_log_ring,
    save_crash_log_snapshot,
};
use logging::{finish_io_forwarding, spawn_io_forwarding};

// ── Statics ───────────────────────────────────────────────────────────────────

static LAUNCHER: OnceLock<Arc<Launcher>> = OnceLock::new();

// ── Launcher ──────────────────────────────────────────────────────────────────
//
// Solo responsabilidad: lanzar instancias.
// Ya no mezcla la lógica de descargas.

pub struct Launcher {
    app_handle: std::sync::Mutex<Option<tauri::AppHandle>>,
}

struct LaunchAttempt(InstanceHandle);

impl LaunchAttempt {
    fn finish(&self, result: &Result<(), AppError>) {
        if let Err(error) = result
            && matches!(self.0.get_status(), InstanceStatus::Starting)
        {
            self.0.set_status(InstanceStatus::Error(error.to_string()));
        }
    }
}

impl Drop for LaunchAttempt {
    fn drop(&mut self) {
        if matches!(self.0.get_status(), InstanceStatus::Starting) {
            self.0.set_status(InstanceStatus::Off);
        }
    }
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
        });
        let _ = LAUNCHER.set(launcher.clone());
        launcher
    }

    pub fn set_handle(&self, handle: tauri::AppHandle) {
        *self.app_handle.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
    }

    pub async fn launch(
        self: &Arc<Self>,
        handle: InstanceHandle,
        server: Option<super::server_status::ServerAddress>,
    ) -> Result<(), AppError> {
        trace!("=== CubicLaunchwerk ===");
        let shared_guard = super::shared_storage::acquire().await;
        let files_guard = handle
            .try_lock_files()
            .map_err(|_| AppError::Instance(InstanceError::FileOperationInProgress))?;

        if handle.is_busy() {
            warn!("La instancia ya está corriendo o iniciando");
            return Err(AppError::Instance(InstanceError::AlreadyStarted));
        }
        clear_crash_log_snapshot(&handle.uuid);
        handle.set_status(InstanceStatus::Starting);
        drop(files_guard);

        let attempt = LaunchAttempt(handle.clone());
        let launcher = self.clone();
        // A confirmed launch owns preparation through the hand-off to its process
        // supervisor, even if the requesting WebView is destroyed meanwhile.
        tokio::spawn(async move {
            let result = launcher.launch_inner(handle, server, shared_guard).await;
            attempt.finish(&result);
            result
        })
        .await
        .map_err(|e| AppError::CoreError(crate::core::CoreError::Other(e.to_string())))?
    }

    async fn launch_inner(
        &self,
        handle: InstanceHandle,
        server: Option<super::server_status::ServerAddress>,
        shared_guard: super::shared_storage::SharedGuard,
    ) -> Result<(), AppError> {
        let settings_m = SettingsManager::launch_snapshot();
        let hide_on_launch = SettingsManager::read().hide_on_launch;

        let version = handle.get_version().await;
        let name = handle.get_name().await;
        let shared_dir = PathManager::get().get_shared_dir();
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
        let check_shared = shared_dir.clone();
        let check_version = version.clone();
        let check_guard = shared_guard.clone();
        let missing = tokio::task::spawn_blocking(move || {
            let _shared_guard = check_guard;
            super::version_installation::missing_dependencies(&check_shared, &check_version)
        })
        .await
        .map_err(|e| crate::core::CoreError::Other(e.to_string()))?;
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

        // If the manifest does not declare a Java version, infer one from the
        // Minecraft version string so we do not default unconditionally to Java 25.
        if java_version_req.is_none() {
            let game_version = GameVersion::from_version_id(&manifest.id_raw);
            let inferred = aqua::infer_java_version(&game_version.mc_version);
            info!(
                "No explicit java_version for {}; inferred Java {}",
                manifest.id_raw, inferred
            );
            java_version_req = Some(JavaVersion {
                component: String::new(),
                major_version: inferred,
            });
        }

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
        let (java_version, java_path) = if let Some(ref overrides) = overrides {
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
            handle.set_status(InstanceStatus::Error(reason));
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

        let (max_mem, min_mem) =
            if let Some(ram_overrides) = overrides.as_ref().and_then(|o| o.memory) {
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
            .auth_uuid(user.uuid)
            .ram(min_mem, max_mem)
            .cracked(user.user_type == AccountType::Cracked);

        if let Some(server) = server {
            let target = super::server_status::resolve_target(&server).await;
            builder = builder
                .quick_play(launchwerk::QuickPlay::Multiplayer(server.authority()))
                .legacy_server(target.host, target.port);
        }

        let mut extra_jvm_args: Vec<String> = Vec::new();

        match user.user_type {
            AccountType::Microsoft => {
                builder = builder.access_token(user.access_token).user_type("msa");
            }
            AccountType::Yggdrasil => {
                builder = builder.access_token(user.access_token).user_type("mojang");

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

        // Argumentos JVM propios de la instancia (estilo MultiMC/Prism).
        if let Some(instance_jvm_args) = overrides.as_ref().and_then(|o| o.jvm_args.as_ref()) {
            extra_jvm_args.extend(
                instance_jvm_args
                    .iter()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(String::from),
            );
        }

        builder = builder.extra_jvm_args(extra_jvm_args);

        let instance_name = name.clone();
        let instance_version = version.clone();
        push_launcher_message(
            &handle.uuid,
            format!(
                "Iniciando instancia \"{}\" ({})",
                instance_name, instance_version
            ),
        )
        .await;
        push_launcher_message(
            &handle.uuid,
            format!("Java: {} (versión {})", java_path_str, java_version),
        )
        .await;
        push_launcher_message(
            &handle.uuid,
            format!("Memoria: {} / {}", min_mem_str, max_mem_str),
        )
        .await;

        let mut options = builder.build();
        let jar_config = handle.get_minecraft_jar().await;
        if jar_config.is_active() {
            let jar_manifest = manifest.clone();
            let jar_shared = shared_dir.clone();
            let jar_instance = instance_dir.clone();
            let jar_options = options.clone();
            let jar_guard = handle.try_lock_files();
            let jar_shared_guard = shared_guard.clone();
            let result = tokio::task::spawn_blocking(move || {
                let _shared_guard = jar_shared_guard;
                // Keep filesystem admission even if launch is cancelled while patching.
                let _jar_guard = jar_guard?;
                let original = zellkern::CommandBuilder::new(
                    &jar_manifest,
                    &jar_shared,
                    &jar_instance,
                    &jar_options,
                )
                .original_minecraft_jar()
                .map_err(|e| e.to_string())?;
                super::minecraft_jar::prepare(&jar_instance, &original, &jar_config)
            })
            .await
            .map_err(|e| e.to_string())
            .and_then(|result| result);
            match result {
                Ok(path) => options.minecraft_jar = path,
                Err(message) => {
                    push_launcher_message(
                        &handle.uuid,
                        format!("Error preparando minecraft.jar: {message}"),
                    )
                    .await;
                    handle.set_status(InstanceStatus::Error(message.clone()));
                    return Err(AppError::Instance(InstanceError::MinecraftJar(message)));
                }
            }
        }

        // Auto-aplicar el perfil global de options.txt antes de lanzar. Un fallo
        // aquí no impide el arranque del juego: se avisa y se continúa.
        if handle.get_auto_apply_options().await
            && let Some(profile_id) = handle.get_options_profile_id().await
        {
            match crate::services::options_profiles::get(profile_id.as_ref()) {
                Some(profile) => {
                    let opt_instance = instance_dir.clone();
                    let opt_key = handle.uuid.to_string();
                    let opt_backup_root = crate::services::options_profiles::backup_root();
                    let opt_guard = handle.try_lock_files();
                    let result = tokio::task::spawn_blocking(move || {
                        let _guard = opt_guard.map_err(crate::core::CoreError::Other)?;
                        crate::services::options_profiles::apply_to_instance(
                            &opt_instance,
                            &opt_key,
                            &profile.content,
                            crate::services::options_profiles::BackupMode::Rolling,
                            &opt_backup_root,
                        )
                    })
                    .await;
                    match result {
                        Ok(Ok(outcome)) => {
                            if outcome.changed {
                                info!("options.txt auto-aplicado desde el perfil {}", profile_id);
                            }
                        }
                        Ok(Err(e)) => {
                            warn!("No se pudo auto-aplicar options.txt: {}", e);
                            push_launcher_message(
                                &handle.uuid,
                                format!("No se pudo aplicar el perfil de options.txt: {e}"),
                            )
                            .await;
                        }
                        Err(e) => {
                            warn!("Tarea de auto-apply de options.txt falló: {}", e);
                        }
                    }
                }
                None => {
                    warn!(
                        "Perfil de options.txt {} no encontrado, se omite el auto-apply",
                        profile_id
                    );
                }
            }
        }

        // Resolve the root per launch, after storage admission, rather than
        // retaining the directory that was active when the launcher started.
        let beta_hide_delay = beta_readiness_delay(
            &manifest.id_raw,
            manifest.inherits_from.as_deref(),
            hide_on_launch,
        );
        let lw = Arc::new(Launchwerk::new(shared_dir));
        let lw_handle = lw.prepare(manifest, options, instance_dir);
        handle.update_last_played().await;

        let app_handle = self
            .app_handle
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let session = LaunchSession::new(app_handle.clone(), hide_on_launch);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        // Se activa cuando el cliente registra su cierre normal (`Stopping!`).
        // Un apagado lento puede disparar el watchdog de Minecraft y terminar el
        // proceso con un código distinto de cero: eso no es un crash.
        let clean_shutdown = Arc::new(AtomicBool::new(false));
        // Subscribe before spawning Java: the first messages can arrive immediately.
        let io_task = spawn_io_forwarding(
            app_handle.clone(),
            handle.uuid.clone(),
            lw_handle.subscribe_stdout(),
            lw_handle.subscribe_stderr(),
            hide_on_launch.then_some(ready_tx),
            clean_shutdown.clone(),
        );

        match lw_handle.launch().await {
            Ok(_) => {
                // Start the grace period only after Java was successfully spawned.
                let beta_hide_deadline =
                    beta_hide_delay.map(|delay| tokio::time::Instant::now() + delay);
                info!("Handle {} lanzado", lw_handle.id().to_string());
                let (kill_tx, mut kill_rx) = tokio::sync::oneshot::channel::<()>();
                let kill_requested = register_kill_sender(&handle.uuid, kill_tx);
                handle.begin_play_session();
                handle.set_status(InstanceStatus::Started);

                let loader = handle.to_dto().await.loader;
                discord_presence::on_instance_start(
                    instance_name.to_string(),
                    instance_version.to_string(),
                    loader.into_owned(),
                )
                .await;

                if app_handle.is_some() {
                    let id = handle.uuid.clone();
                    if settings_m.open_console_on_launch {
                        spawn_open_log_window(
                            app_handle.clone(),
                            id.to_string(),
                            instance_name.to_string(),
                        );
                    }
                    push_launcher_message(&id, "Proceso iniciado").await;
                }

                let uuid = handle.uuid.clone();
                let h = handle.clone();
                let inst_name = instance_name.clone();
                let app_for_show = app_handle.clone();
                tokio::spawn(async move {
                    let _shared_guard = shared_guard;
                    let readiness = wait_for_game_readiness(ready_rx, beta_hide_deadline);
                    tokio::pin!(readiness);
                    let mut ready_seen = false;
                    let result = loop {
                        tokio::select! {
                            biased;
                            Ok(()) = &mut kill_rx => {
                                info!("Kill signal received for {}", uuid);
                                if let Err(e) = lw_handle.kill().await {
                                    warn!("Error al matar proceso {}: {:?}", uuid, e);
                                }
                                break lw_handle.wait().await;
                            }
                            result = lw_handle.wait() => {
                                info!("Instance {} exited: {:?}", uuid, result);
                                break result;
                            }
                            _ = &mut readiness, if hide_on_launch && !ready_seen => {
                                ready_seen = true;
                                session.game_ready();
                            }
                        }
                    };
                    session.process_exited();
                    unregister_kill_sender(&uuid);
                    // Release the manager's strong reference and drain the last logs.
                    // Otherwise its broadcast senders (and forwarding tasks) live forever.
                    lw.remove(lw_handle.id());
                    drop(lw_handle);
                    finish_io_forwarding(io_task).await;
                    push_launcher_message(&uuid, format!("El proceso terminó: {:?}", result)).await;

                    let crashed = is_unexpected_exit(
                        result,
                        kill_requested.load(Ordering::Acquire),
                        clean_shutdown.load(Ordering::Acquire),
                    );
                    if crashed {
                        let code = result.unwrap_or(-1);
                        push_launcher_message(&uuid, format!("Crash detectado (código {})", code))
                            .await;
                        save_crash_log_snapshot(&uuid).await;
                        emit(AppEvent::InstanceCrashed {
                            id: uuid.to_compact_string(),
                            name: inst_name.to_compact_string(),
                            exit_code: result,
                            reason: Some(CompactString::new(format!(
                                "El proceso terminó con código {}",
                                code
                            ))),
                        });
                        if let Some(app) = app_for_show
                            && let Err(err) = open_log_window_for_instance(
                                app,
                                uuid.to_string(),
                                inst_name.to_string(),
                            )
                            .await
                        {
                            warn!("No se pudo abrir ventana de logs: {err}");
                        }
                    }

                    discord_presence::on_instance_stop(&inst_name).await;
                    remove_log_ring(&uuid);
                    h.end_play_session().await;
                    h.set_status(InstanceStatus::Off);
                    drop(session);
                });
            }
            Err(e) => {
                lw.remove(lw_handle.id());
                drop(lw_handle);
                finish_io_forwarding(io_task).await;
                let msg = e.to_string();
                error!("{}", msg);
                push_launcher_message(&handle.uuid, format!("Error al iniciar: {}", msg)).await;
                handle.set_status(InstanceStatus::Error(msg.clone()));
                return Err(AppError::CoreError(crate::core::CoreError::Other(msg)));
            }
        }
        Ok(())
    }
}

pub(crate) async fn refresh_microsoft_token(
    mut user: MinecraftUser,
) -> Result<MinecraftUser, AppError> {
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

fn is_unexpected_exit(exit_code: Option<i32>, kill_requested: bool, clean_shutdown: bool) -> bool {
    !kill_requested && !clean_shutdown && matches!(exit_code, Some(code) if code != 0)
}

/// Minecraft registra `Stopping!` cuando el usuario cierra el cliente por su
/// cuenta (o al salir del mundo). A partir de ahí el apagado es intencional:
/// aunque el watchdog de apagado del cliente termine el proceso con un código
/// distinto de cero, no es un crash. Formatos soportados:
/// `[Render thread/INFO]: Stopping!` y
/// `[Render thread/INFO] [net.minecraft.client.Minecraft/]: Stopping!`.
fn is_clean_shutdown(line: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\]:\s*stopping!\s*$").unwrap())
        .is_match(line)
}

fn spawn_open_log_window(
    app: Option<tauri::AppHandle>,
    instance_id: String,
    instance_name: String,
) {
    if let Some(app) = app {
        tokio::spawn(async move {
            if let Err(err) = open_log_window_for_instance(app, instance_id, instance_name).await {
                warn!("No se pudo abrir ventana de logs: {}", err);
            }
        });
    }
}

// PolyMC's LauncherPartLaunch uses this message to close its UI once Minecraft
// has reached client initialization, rather than merely after Java was spawned.
fn is_game_ready(line: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\bsetting user(?::|\s)\s*\S+").unwrap())
        .is_match(line)
}

/// Old Minecraft betas may never print the modern client-ready marker. Use the
/// base version ID (including inherited/modded profiles), not the launcher version.
fn beta_readiness_delay(
    version: &str,
    parent: Option<&str>,
    hide_on_launch: bool,
) -> Option<std::time::Duration> {
    if !hide_on_launch {
        return None;
    }
    let base = GameVersion::from_version_id(parent.unwrap_or(version)).mc_version;
    let beta = base
        .strip_prefix('b')
        .and_then(|rest| rest.split_once('.'))
        .is_some_and(|(major, minor)| {
            !major.is_empty()
                && major.bytes().all(|byte| byte.is_ascii_digit())
                && minor.as_bytes().first().is_some_and(u8::is_ascii_digit)
        });
    beta.then_some(std::time::Duration::from_secs(2))
}

async fn wait_for_game_readiness(
    mut marker: tokio::sync::oneshot::Receiver<()>,
    beta_deadline: Option<tokio::time::Instant>,
) {
    let fallback = async {
        match beta_deadline {
            Some(deadline) => tokio::time::sleep_until(deadline).await,
            None => std::future::pending::<()>().await,
        }
    };
    tokio::pin!(fallback);
    tokio::select! {
        result = &mut marker => {
            // Closing stdout/stderr is not a readiness signal. Betas may run
            // without either stream; other versions must still wait for a marker.
            if result.is_err() {
                fallback.await;
            }
        }
        _ = &mut fallback => {}
    }
    // The supervisor prioritizes kill/exit over this future and drops it on exit,
    // so the timer cannot hide the launcher after a failed/terminated session.
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

#[cfg(test)]
#[path = "../tests/services/launcher.rs"]
mod tests;
