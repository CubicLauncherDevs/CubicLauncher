use crate::commands::others::open_url;
use crate::core::errors::{AuthError, CoreError};
use crate::services::SettingsManager;
use launchwerk::auth::{MinecraftUser, microsoft::MicrosoftAuth, yggdrasil::YggdrasilAuth};
use serde::Serialize;
use tauri::command;
use tracing::{info, warn};

#[derive(Serialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[command]
pub async fn get_device_code() -> Result<DeviceCode, String> {
    info!("Obteniendo código de dispositivo de Microsoft");
    let res = tokio::task::spawn_blocking(|| {
        MicrosoftAuth::default()
            .get_device_code()
            .map_err(|e| AuthError::DeviceCodeFailed(e.to_string()).to_string())
    })
    .await
    .map_err(|e| AuthError::SpawnBlocking(e.to_string()).to_string())??;

    info!(
        "Código de dispositivo obtenido: user_code={}",
        res.user_code
    );
    Ok(DeviceCode {
        user_code: res.user_code,
        device_code: res.device_code,
        verification_uri: res.verification_uri,
        expires_in: res.expires_in,
        interval: res.interval,
    })
}

#[command]
pub async fn authenticate_with_device_code(
    device_code: String,
    interval: u64,
    expires_in: u64,
) -> Result<MinecraftUser, String> {
    info!("Autenticando con código de dispositivo...");
    open_url("https://www.microsoft.com/link".to_string())?;
    let user = tokio::task::spawn_blocking(move || {
        MicrosoftAuth::default()
            .authenticate_with_device_code(&device_code, interval, expires_in)
            .map_err(|e| AuthError::AuthFailed(e.to_string()).to_string())
    })
    .await
    .map_err(|e| AuthError::SpawnBlocking(e.to_string()).to_string())??;

    info!("Autenticación exitosa para {}", user.username);

    user.save_tokens()
        .map_err(|e| AuthError::SaveTokensFailed(e.to_string()).to_string())?;

    SettingsManager::write(|settings| {
        settings.add_user(user.clone());
        settings.active_user_idx = settings.user.len() - 1;
    })?;
    SettingsManager::save().await?;
    Ok(user)
}

#[command]
pub fn get_current_user() -> MinecraftUser {
    let settings = SettingsManager::read();
    let mut s_user = settings.get_user();

    if let Err(e) = s_user.load_tokens() {
        warn!("Error cargando tokens: {:?}", e);
    }
    s_user
}

#[command]
pub async fn logout() -> Result<(), String> {
    info!("Cerrando sesión de usuario");
    SettingsManager::write(|settings| {
        let user = settings.get_user();
        info!("Eliminando tokens para {}", user.username);
        if let Err(_e) = user.delete_tokens() {
            warn!("Error eliminando tokens.");
        }
        settings.set_user(MinecraftUser::cracked(&user.username));
    })?;
    SettingsManager::save().await?;
    info!("Sesión cerrada exitosamente");
    Ok(())
}

#[command]
pub async fn switch_user(idx: usize) -> Result<(), String> {
    info!("Cambiando al usuario en índice {}", idx);
    {
        let settings = SettingsManager::read();
        if idx >= settings.user.len() {
            return Err(CoreError::Other(format!(
                "Índice {} fuera de rango ({} usuarios)",
                idx,
                settings.user.len()
            ))
            .to_string());
        }
    }
    SettingsManager::write(|settings| {
        settings.active_user_idx = idx;
        let mut user = settings.get_user();
        if let Err(e) = user.load_tokens() {
            warn!("Error cargando tokens: {:?}", e);
        }
    })?;
    SettingsManager::save().await?;
    Ok(())
}

#[command]
pub async fn remove_user(username: String) -> Result<(), String> {
    info!("Eliminando usuario {}", username);
    {
        let user = {
            let settings = SettingsManager::read();
            settings
                .user
                .iter()
                .find(|u| u.username == username)
                .cloned()
        };
        if let Some(u) = user {
            info!("Eliminando tokens para {}", u.username);
            if let Err(e) = u.delete_tokens() {
                warn!("Error eliminando tokens: {:?}", e);
            }
        }
    }
    SettingsManager::write(|settings| {
        settings.rem_user(&username);
    })?;
    SettingsManager::save().await?;
    info!("Usuario {} eliminado exitosamente", username);
    Ok(())
}

#[command]
pub fn get_user_list() -> Vec<MinecraftUser> {
    let settings = SettingsManager::read();
    settings.user.clone()
}

// ── Yggdrasil Commands ──────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct YggdrasilServerInfo {
    pub server_name: String,
    pub skin_domains: Vec<String>,
    pub non_email_login: bool,
}

#[command]
pub async fn get_yggdrasil_server_info(url: String) -> Result<YggdrasilServerInfo, String> {
    info!("Obteniendo info del servidor Yggdrasil: {}", url);
    let auth = YggdrasilAuth::new();

    let api_root = auth
        .resolve_api_url(&url)
        .await
        .map_err(|e| e.to_string())?;

    let metadata = auth
        .fetch_metadata(&api_root)
        .await
        .map_err(|e| e.to_string())?;

    Ok(YggdrasilServerInfo {
        server_name: metadata.server_name,
        skin_domains: metadata.skin_domains,
        non_email_login: metadata.non_email_login,
    })
}

#[command]
pub async fn yggdrasil_authenticate(
    server_url: String,
    username: String,
    password: String,
) -> Result<MinecraftUser, String> {
    info!("Autenticando con servidor Yggdrasil: {}", server_url);
    let auth = YggdrasilAuth::new();

    let api_root = auth
        .resolve_api_url(&server_url)
        .await
        .map_err(|e| e.to_string())?;

    let user = auth
        .authenticate(&api_root, &username, &password)
        .await
        .map_err(|e| e.to_string())?;

    info!("Autenticación Yggdrasil exitosa para {}", user.username);

    user.save_tokens()
        .map_err(|e| AuthError::SaveTokensFailed(e.to_string()).to_string())?;

    SettingsManager::write(|settings| {
        settings.add_user(user.clone());
        settings.active_user_idx = settings.user.len() - 1;
    })?;
    SettingsManager::save().await?;

    Ok(user)
}

#[command]
pub async fn yggdrasil_refresh_token(idx: usize) -> Result<(), String> {
    info!("Refrescando token Yggdrasil para usuario en índice {}", idx);
    let mut user = {
        let settings = SettingsManager::read();
        if idx >= settings.user.len() {
            return Err(format!(
                "Índice {} fuera de rango ({} usuarios)",
                idx,
                settings.user.len()
            ));
        }
        settings.user[idx].clone()
    };

    if user.user_type != launchwerk::auth::AccountType::Yggdrasil {
        return Err("El usuario no es de tipo Yggdrasil".to_string());
    }

    user.load_tokens()
        .map_err(|e| AuthError::AuthFailed(e.to_string()).to_string())?;

    let server_url = user
        .yggdrasil_server_url
        .clone()
        .ok_or_else(|| "URL del servidor Yggdrasil no configurada".to_string())?;

    let client_token = user
        .client_token
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let auth = YggdrasilAuth::new();
    let refreshed = auth
        .refresh(
            &server_url,
            &user.access_token,
            &client_token,
            &user.uuid,
            &user.username,
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut new_user = refreshed;
    new_user.yggdrasil_server_url = Some(server_url);

    new_user
        .save_tokens()
        .map_err(|e| AuthError::SaveTokensFailed(e.to_string()).to_string())?;

    SettingsManager::write(|settings| {
        settings.set_user(new_user);
    })?;
    SettingsManager::save().await?;

    Ok(())
}
