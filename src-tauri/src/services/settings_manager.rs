use super::interface_preferences::InterfacePreferences;
use super::notification_preferences::NotificationPreferences;
use crate::core::{AppError, CoreError, FsError, PathManager, emit};
use compact_str::CompactString;
use launchwerk::auth::MinecraftUser;
use parking_lot::{RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::OnceLock;
use tokio::fs;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

// ── Static global ─────────────────────────────────────────────────────────────

static SETTINGS: LazyLock<RwLock<SettingsManager>> =
    LazyLock::new(|| RwLock::new(SettingsManager::load()));

static SAVE_TX: OnceLock<mpsc::UnboundedSender<()>> = OnceLock::new();

pub fn init_auto_save() {
    let (tx, mut rx) = mpsc::unbounded_channel::<()>();
    SAVE_TX.set(tx).ok();
    tokio::spawn(async move {
        loop {
            rx.recv().await;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            while rx.try_recv().is_ok() {}
            if let Err(e) = SettingsManager::save().await {
                warn!("Error en auto-save: {}", e);
            }
        }
    });
}

// ── Defaults (serde) ──────────────────────────────────────────────────────────

fn default_min_mem() -> u32 {
    1024
}
fn default_max_mem() -> u32 {
    2048
}
fn default_lang() -> CompactString {
    CompactString::from("es")
}
fn default_true() -> bool {
    true
}
fn default_update_channel() -> CompactString {
    CompactString::from("stable")
}
fn default_theme() -> CompactString {
    CompactString::from("dark")
}
fn default_console_history_limit() -> u32 {
    3000
}
fn default_active_user_idx() -> usize {
    0
}
fn default_user() -> Vec<MinecraftUser> {
    let vec = vec![MinecraftUser::cracked("Steve")];
    vec
}
// ── SettingsManager ───────────────────────────────────────────────────────────

/// Configuración persistida del launcher.
/// La memoria se almacena en MB.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SettingsManager {
    #[serde(default = "default_user")]
    pub user: Vec<MinecraftUser>,
    #[serde(default = "default_active_user_idx")]
    pub active_user_idx: usize,
    #[serde(default = "default_min_mem")]
    pub min_memory: u32,
    #[serde(default = "default_max_mem")]
    pub max_memory: u32,
    #[serde(default)]
    pub jre8_path: PathBuf,
    #[serde(default = "default_true")]
    pub jre8_managed: bool,
    #[serde(default)]
    pub jre17_path: PathBuf,
    #[serde(default = "default_true")]
    pub jre17_managed: bool,
    #[serde(default)]
    pub jre21_path: PathBuf,
    #[serde(default = "default_true")]
    pub jre21_managed: bool,
    #[serde(default)]
    pub jre25_path: PathBuf,
    #[serde(default = "default_true")]
    pub jre25_managed: bool,
    #[serde(default = "default_lang")]
    pub language: CompactString,
    #[serde(default = "default_true")]
    pub auto_updates: bool,
    #[serde(default = "default_update_channel")]
    pub update_channel: CompactString,
    #[serde(default)]
    pub show_error_console: bool,
    #[serde(default = "default_true")]
    pub close_launcher_on_play: bool,
    /// Close the main WebView once Minecraft initializes; restore it on exit.
    #[serde(default)]
    pub hide_on_launch: bool,
    #[serde(default)]
    pub open_console_on_launch: bool,
    #[serde(default = "default_console_history_limit")]
    pub console_history_limit: u32,
    #[serde(default = "default_true")]
    pub console_show_level_tags: bool,
    #[serde(default)]
    pub show_snapshots: bool,
    #[serde(default)]
    pub show_alpha: bool,
    #[serde(default)]
    pub show_unstable_loaders: bool,
    #[serde(default)]
    pub jvm_args: CompactString,
    #[serde(default)]
    pub env_vars: HashMap<CompactString, String>,
    #[serde(default = "default_theme")]
    pub theme: CompactString,
    #[serde(default = "default_true")]
    pub discord_presence: bool,
    #[serde(default = "default_true")]
    pub show_tutorial: bool,
    #[serde(default)]
    pub license_accepted: bool,
    #[serde(default)]
    pub market_filter_collapsed: bool,
    #[serde(default)]
    pub prominent_notifications: bool,
    // None distinguishes legacy settings from explicitly customized preferences.
    #[serde(default)]
    pub notification_preferences: Option<NotificationPreferences>,
    #[serde(default)]
    pub interface_preferences: InterfacePreferences,
    #[serde(default)]
    pub reduce_animations: bool,
    #[serde(default)]
    pub disable_blur_effects: bool,
    #[serde(default)]
    pub disable_infinite_animations: bool,
    #[serde(default)]
    pub disable_skin3d_animations: bool,
    #[serde(default)]
    pub reduce_log_animations: bool,
    /// Directorio personalizado para las instancias. Vacío = ubicación por defecto.
    #[serde(default)]
    pub custom_instances_dir: PathBuf,
    #[serde(skip)]
    pub dirty: bool,
}

// ── SettingsSnapshot ──────────────────────────────────────────────────────────

/// Snapshot liviano para el hot path (launch).
/// Solo contiene los campos necesarios para lanzar una instancia.
pub struct SettingsSnapshot {
    pub min_memory: u32,
    pub max_memory: u32,
    pub env_vars: HashMap<CompactString, String>,
    pub jvm_args: CompactString,
    pub jre8_managed: bool,
    pub jre17_managed: bool,
    pub jre21_managed: bool,
    pub jre25_managed: bool,
    pub jre8_path: Box<Path>,
    pub jre17_path: Box<Path>,
    pub jre21_path: Box<Path>,
    pub jre25_path: Box<Path>,
    pub open_console_on_launch: bool,
}

impl SettingsSnapshot {
    pub fn get_jre8_path(&self) -> &Path {
        &self.jre8_path
    }
    pub fn get_jre17_path(&self) -> &Path {
        &self.jre17_path
    }
    pub fn get_jre21_path(&self) -> &Path {
        &self.jre21_path
    }
    pub fn get_jre25_path(&self) -> &Path {
        &self.jre25_path
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self {
            user: {
                let vec = vec![MinecraftUser::cracked("Steve")];
                vec
            },
            active_user_idx: 0,
            min_memory: 1024,
            max_memory: 2048,
            jre8_path: PathBuf::new(),
            jre8_managed: true,
            jre17_path: PathBuf::new(),
            jre17_managed: true,
            jre21_path: PathBuf::new(),
            jre21_managed: true,
            jre25_path: PathBuf::new(),
            jre25_managed: true,
            language: CompactString::from("es"),
            auto_updates: true,
            update_channel: CompactString::from("stable"),
            show_error_console: false,
            close_launcher_on_play: true,
            hide_on_launch: false,
            open_console_on_launch: false,
            console_history_limit: 3000,
            console_show_level_tags: true,
            show_snapshots: false,
            show_alpha: false,
            show_unstable_loaders: false,
            jvm_args: CompactString::default(),
            env_vars: HashMap::new(),
            theme: CompactString::from("dark"),
            discord_presence: true,
            show_tutorial: true,
            license_accepted: false,
            market_filter_collapsed: true,
            prominent_notifications: false,
            notification_preferences: Some(NotificationPreferences::default()),
            interface_preferences: InterfacePreferences::default(),
            reduce_animations: false,
            disable_blur_effects: false,
            disable_infinite_animations: false,
            disable_skin3d_animations: false,
            reduce_log_animations: false,
            custom_instances_dir: PathBuf::new(),
            dirty: true,
        }
    }
}

impl SettingsManager {
    pub fn read() -> RwLockReadGuard<'static, SettingsManager> {
        SETTINGS.read()
    }

    pub fn write(f: impl FnOnce(&mut SettingsManager)) -> Result<(), CoreError> {
        let mut settings = SETTINGS.write();
        let previous_theme = settings.theme.clone();
        f(&mut settings);
        settings.normalize_offline_users();
        if settings.theme != previous_theme {
            // Follow write order, not the completion order of asynchronous saves.
            crate::theme_watcher::ThemeWatcher::watch(
                settings.theme.strip_prefix("user:").map(str::to_owned),
            );
        }
        settings.dirty = true;
        if let Some(tx) = SAVE_TX.get() {
            let _ = tx.send(());
        }
        Ok(())
    }

    pub fn snapshot() -> SettingsManager {
        SETTINGS.read().clone()
    }

    pub fn launch_snapshot() -> SettingsSnapshot {
        let s = SETTINGS.read();
        SettingsSnapshot {
            min_memory: s.min_memory,
            max_memory: s.max_memory,
            env_vars: s.env_vars.clone(),
            jvm_args: s.jvm_args.clone(),
            jre8_managed: s.jre8_managed,
            jre17_managed: s.jre17_managed,
            jre21_managed: s.jre21_managed,
            jre25_managed: s.jre25_managed,
            jre8_path: s.jre8_path.clone().into_boxed_path(),
            jre17_path: s.jre17_path.clone().into_boxed_path(),
            jre21_path: s.jre21_path.clone().into_boxed_path(),
            jre25_path: s.jre25_path.clone().into_boxed_path(),
            open_console_on_launch: s.open_console_on_launch,
        }
    }

    // ── Getters ───────────────────────────────────────────────────────────────

    pub fn get_min_memory(&self) -> u32 {
        self.min_memory
    }
    pub fn get_max_memory(&self) -> u32 {
        self.max_memory
    }
    pub fn add_user(&mut self, user: MinecraftUser) {
        self.user.push(user);
    }

    #[allow(dead_code)]
    pub fn rem_user(&mut self, user_name: &str) {
        let idx = self.user.iter().position(|u| u.username == user_name);
        self.user.retain(|s| s.username != user_name);
        if let Some(i) = idx {
            if self.user.is_empty() {
                self.user.push(MinecraftUser::cracked("Steve"));
                self.active_user_idx = 0;
            } else if i < self.active_user_idx || self.active_user_idx >= self.user.len() {
                self.active_user_idx = self.active_user_idx.saturating_sub(1);
            }
        }
    }

    pub fn rem_user_by_uuid(&mut self, uuid: &str) {
        let idx = self.user.iter().position(|u| u.uuid == uuid);
        self.user.retain(|u| u.uuid != uuid);
        if let Some(i) = idx {
            if self.user.is_empty() {
                self.user.push(MinecraftUser::cracked("Steve"));
                self.active_user_idx = 0;
            } else if i < self.active_user_idx || self.active_user_idx >= self.user.len() {
                self.active_user_idx = self.active_user_idx.saturating_sub(1);
            }
        }
    }

    pub fn get_user(&self) -> MinecraftUser {
        if self.active_user_idx < self.user.len() {
            return self.user[self.active_user_idx].clone();
        }
        warn!("se intento obtener el usuario actual pero este no existe, devolviendo default.");
        MinecraftUser::cracked("Steve")
    }
    /// Reemplaza el usuario activo por otro
    pub fn set_user(&mut self, user: MinecraftUser) {
        if self.active_user_idx < self.user.len() {
            self.user[self.active_user_idx] = user;
        } else {
            warn!("se intento reemplazar un usuario pero este no existe.");
        }
    }

    /// Reemplaza un usuario por su UUID
    pub fn set_user_by_uuid(&mut self, user: &MinecraftUser) {
        if let Some(existing) = self.user.iter_mut().find(|u| u.uuid == user.uuid) {
            *existing = user.clone();
        } else {
            warn!(
                "se intento reemplazar un usuario por UUID pero no se encontró: {}",
                user.uuid
            );
        }
    }
    // ── Persistencia ──────────────────────────────────────────────────────────

    /// Serializa y escribe a disco.
    /// Clona fuera del lock para minimizar la contención.
    pub async fn save() -> Result<(), AppError> {
        let (clone, path) = {
            let settings = SETTINGS.read();
            if !settings.dirty {
                return Ok(());
            }
            let path = PathManager::get().get_settings_dir().join("settings.cub");
            (settings.clone(), path)
        };

        let parent = path.parent().ok_or_else(|| {
            AppError::CoreError(CoreError::Serialize(format!(
                "Ruta de settings inválida: {}",
                path.display()
            )))
        })?;

        fs::create_dir_all(parent).await.map_err(|e| {
            AppError::Fs(FsError::CreateDir {
                path: parent.to_string_lossy().to_string(),
                source: e,
            })
        })?;

        let json_bytes = serde_json::to_vec(&clone)
            .map_err(|e| AppError::CoreError(CoreError::Serialize(e.to_string())))?;

        fs::write(&path, json_bytes).await.map_err(|e| {
            AppError::Fs(FsError::WriteFile {
                path: path.to_string_lossy().to_string(),
                source: e,
            })
        })?;

        {
            let mut settings = SETTINGS.write();
            settings.dirty = false;
        }

        info!("Configuración guardada en {:?}", path);
        emit(crate::core::AppEvent::STChanged);
        Ok(())
    }

    pub fn load() -> Self {
        let path = PathManager::get().get_settings_dir().join("settings.cub");

        if !path.exists() {
            info!("No hay archivo de configuración, usando valores por defecto");
            return Self::default();
        }

        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                error!("Error al leer la configuración desde {:?}: {}", path, e);
                return Self::default();
            }
        };

        let reader = BufReader::new(file);
        match serde_json::from_reader::<_, Self>(reader) {
            Ok(mut settings) => {
                settings.migrate();
                info!("Configuración cargada desde {:?}", path);
                settings
            }
            Err(e) => {
                error!(
                    "Configuración inválida en {:?} ({}), creando backup",
                    path, e
                );
                if let Err(e) = std::fs::copy(&path, path.with_extension("cub.bak")) {
                    warn!("Error creando backup de configuración {:?}: {}", path, e);
                }
                Self::default()
            }
        }
    }

    /// Migraciones de versiones anteriores del formato.
    fn migrate(&mut self) {
        self.normalize_offline_users();
        self.normalize_notification_preferences();
        let previous_scale = self.interface_preferences.scale;
        self.interface_preferences.normalize();
        if self.interface_preferences.scale != previous_scale {
            self.dirty = true;
        }
        // v1 → v2: memoria en MB a GB
        if self.min_memory > 128 {
            self.min_memory = (self.min_memory / 1024).max(1);
            self.dirty = true;
        }
        if self.max_memory > 128 {
            self.max_memory = (self.max_memory / 1024).max(1);
            self.dirty = true;
        }
        // v2 → v3: GB a MB
        if self.min_memory <= 64 {
            self.min_memory *= 1024;
            self.dirty = true;
        }
        if self.max_memory <= 64 {
            self.max_memory *= 1024;
            self.dirty = true;
        }
        // Asegurar que el límite del historial de consola esté en rango válido.
        let sanitized = self.console_history_limit.clamp(100, 5000);
        if sanitized != self.console_history_limit {
            self.console_history_limit = sanitized;
            self.dirty = true;
        }
    }

    fn normalize_offline_users(&mut self) {
        for user in &mut self.user {
            if user.ensure_offline_uuid() {
                self.dirty = true;
            }
        }
    }

    pub(crate) fn normalize_notification_preferences(&mut self) {
        let mut preferences = self
            .notification_preferences
            .clone()
            .unwrap_or_else(|| NotificationPreferences::from_legacy(self.prominent_notifications));
        preferences.normalize();
        if self.notification_preferences.as_ref() != Some(&preferences) {
            self.notification_preferences = Some(preferences);
            self.dirty = true;
        }
    }
}

#[cfg(test)]
#[path = "../tests/services/settings_manager.rs"]
mod tests;
