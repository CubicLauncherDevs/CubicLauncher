use crate::core::http_client::HTTP;
use crate::services::launcher::refresh_microsoft_token;
use crate::services::{
    SettingsManager,
    skin_closet_manager::{SkinClosetEntry, SkinClosetManager, SkinImageSource, now_ts},
};
use base64::Engine as _;
use base64::engine::general_purpose;
use launchwerk::auth::AccountType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tauri::command;
use tokio::sync::Mutex as TokioMutex;
use tracing::{error, info};

pub(crate) const MSA_SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
pub(crate) const MSA_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
pub(crate) const MSA_CAPE_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile/capes/active";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfileSkin {
    pub id: String,
    pub state: String,
    pub url: String,
    pub variant: String,
    #[serde(default)]
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfileCape {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub alias: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfileResponse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<MinecraftProfileSkin>,
    #[serde(default)]
    pub capes: Vec<MinecraftProfileCape>,
    #[serde(default)]
    pub profile_actions: Vec<String>,
}

/// Cache con TTL y coalescing de requests en vuelo para evitar ráfagas de
/// peticiones al endpoint de perfil de Minecraft Services y prevenir 429.
#[derive(Clone)]
struct CachedProfile {
    profile: Result<MinecraftProfileResponse, String>,
    fetched_at: Instant,
}

struct ProfileCacheState {
    entries: HashMap<String, CachedProfile>,
    in_flight: HashMap<String, Arc<tokio::sync::Notify>>,
}

#[derive(Clone)]
struct ProfileCache {
    state: Arc<TokioMutex<ProfileCacheState>>,
}

impl ProfileCache {
    const TTL: Duration = Duration::from_secs(30);

    fn new() -> Self {
        Self {
            state: Arc::new(TokioMutex::new(ProfileCacheState {
                entries: HashMap::new(),
                in_flight: HashMap::new(),
            })),
        }
    }

    /// Borra la entrada de caché de un UUID sin bloquear. Si no puede tomar el
    /// lock inmediatamente, deja que el TTL expire por sí solo.
    fn invalidate(&self, uuid: &str) {
        if let Ok(mut state) = self.state.try_lock() {
            state.entries.remove(uuid);
        }
    }

    async fn get_or_fetch<F, Fut>(
        &self,
        uuid: String,
        fetch: F,
    ) -> Result<MinecraftProfileResponse, String>
    where
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<MinecraftProfileResponse, String>> + Send,
    {
        loop {
            let in_flight_notify = {
                let state = self.state.lock().await;
                if let Some(entry) = state.entries.get(&uuid)
                    && entry.fetched_at.elapsed() < Self::TTL
                {
                    return entry.profile.clone();
                }
                state.in_flight.get(&uuid).cloned()
            };

            if let Some(notify) = in_flight_notify {
                notify.notified().await;
                continue;
            }

            let our_notify = {
                let mut state = self.state.lock().await;
                if let Some(entry) = state.entries.get(&uuid)
                    && entry.fetched_at.elapsed() < Self::TTL
                {
                    return entry.profile.clone();
                }
                if state.in_flight.contains_key(&uuid) {
                    continue;
                }
                let arc = Arc::new(tokio::sync::Notify::new());
                state.in_flight.insert(uuid.clone(), arc.clone());
                arc
            };

            let result = fetch().await;
            let cached = CachedProfile {
                profile: result.clone(),
                fetched_at: Instant::now(),
            };
            {
                let mut state = self.state.lock().await;
                state.entries.insert(uuid.clone(), cached);
                state.in_flight.remove(&uuid);
            }
            our_notify.notify_waiters();
            return result;
        }
    }
}

static PROFILE_CACHE: LazyLock<ProfileCache> = LazyLock::new(ProfileCache::new);

pub(crate) fn mojang_error_message(status: reqwest::StatusCode, body: String) -> String {
    if body.trim().is_empty() {
        return format!("Error de Mojang (HTTP {})", status.as_u16());
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body)
        && let Some(msg) = value
            .get("errorMessage")
            .or_else(|| value.get("message"))
            .or_else(|| value.get("error"))
            .and_then(|v| v.as_str())
    {
        return format!("{} (HTTP {})", msg, status.as_u16());
    }
    format!("{} (HTTP {})", body, status.as_u16())
}

/// Carga el token de Minecraft Services para el usuario indicado.
/// El frontend no recibe `access_token` por seguridad (`#[serde(skip)]`),
/// por lo que lo recuperamos desde el almacenamiento seguro en el backend.
pub(crate) fn load_minecraft_token(uuid: &str) -> Result<String, String> {
    let mut user = SettingsManager::read()
        .user
        .iter()
        .find(|u| u.uuid == uuid)
        .cloned()
        .ok_or_else(|| format!("No se encontró el usuario con UUID {}", uuid))?;

    if user.user_type != AccountType::Microsoft {
        return Err("Solo los usuarios de Microsoft pueden gestionar skins/capes".to_string());
    }

    user.load_tokens()
        .map_err(|e| format!("No se pudieron cargar los tokens del usuario: {}", e))?;

    Ok(user.access_token)
}

/// Refresca el token de Microsoft para el usuario indicado y devuelve el nuevo
/// access token. No requiere que el usuario vuelva a iniciar sesión.
pub(crate) async fn refresh_msa_token(uuid: &str) -> Result<String, String> {
    let user = SettingsManager::read()
        .user
        .iter()
        .find(|u| u.uuid == uuid)
        .cloned()
        .ok_or_else(|| format!("No se encontró el usuario con UUID {}", uuid))?;

    if user.user_type != AccountType::Microsoft {
        return Err("Solo los usuarios de Microsoft pueden gestionar skins/capes".to_string());
    }

    let refreshed = refresh_microsoft_token(user)
        .await
        .map_err(|e| e.to_string())?;

    Ok(refreshed.access_token)
}

/// Envía una petición a Minecraft Services con el token del usuario. Si la API
/// responde 401 (token expirado), refresca el token automáticamente y reintenta
/// una única vez.
pub(crate) async fn send_msa_request(
    uuid: &str,
    build: impl Fn(String) -> reqwest::RequestBuilder,
) -> Result<(reqwest::StatusCode, String), String> {
    let access_token = load_minecraft_token(uuid)?;

    let res = build(access_token.clone())
        .send()
        .await
        .map_err(|e| format!("Error de red: {}", e))?;

    if res.status() == reqwest::StatusCode::UNAUTHORIZED {
        info!("Token de Microsoft expirado para {}, refrescando...", uuid);
        let new_token = refresh_msa_token(uuid).await?;
        let res = build(new_token)
            .send()
            .await
            .map_err(|e| format!("Error de red: {}", e))?;
        let status = res.status();
        let body = res.text().await.map_err(|e| e.to_string())?;
        return Ok((status, body));
    }

    let status = res.status();
    let body = res.text().await.map_err(|e| e.to_string())?;
    Ok((status, body))
}

pub(crate) async fn capture_active_skin_to_closet(uuid: &str) {
    let profile = match get_minecraft_profile_impl(uuid).await {
        Ok(p) => p,
        Err(e) => {
            error!("Error capturando skin activa para el closet: {}", e);
            return;
        }
    };

    let Some(active) = profile.skins.into_iter().find(|s| s.state == "ACTIVE") else {
        return;
    };

    let entries = SkinClosetManager::get_entries(uuid).await;
    if entries.iter().any(|e| e.id == active.id) {
        return;
    }

    let local_path = match SkinClosetManager::store_image(
        uuid,
        &active.id,
        SkinImageSource::Url(active.url.clone()),
    )
    .await
    {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(e) => {
            error!("Error guardando imagen del closet: {}", e);
            return;
        }
    };

    let entry = SkinClosetEntry {
        id: active.id,
        url: active.url,
        local_path,
        variant: active.variant,
        alias: String::new(),
        saved_at: now_ts(),
    };

    if let Err(e) = SkinClosetManager::sync_entry(uuid, entry).await {
        error!("Error sincronizando entrada del closet: {}", e);
    }
}

pub(crate) async fn get_minecraft_profile_impl(
    uuid: &str,
) -> Result<MinecraftProfileResponse, String> {
    let uuid = uuid.to_string();
    PROFILE_CACHE
        .get_or_fetch(uuid.clone(), move || async move {
            info!("Obteniendo perfil de Minecraft Services para {}", uuid);

            let (status, body) =
                send_msa_request(&uuid, |token| HTTP.get(MSA_PROFILE_URL).bearer_auth(token))
                    .await?;

            if !status.is_success() {
                error!("Error obteniendo perfil: HTTP {}", status);
                return Err(mojang_error_message(status, body));
            }

            serde_json::from_str(&body).map_err(|e| {
                error!("Error parseando perfil: {}", e);
                format!("Respuesta inválida: {}", e)
            })
        })
        .await
}

pub(crate) async fn upload_skin_file_impl(
    uuid: &str,
    file_path: &str,
    model: &str,
) -> Result<(), String> {
    info!("Subiendo skin desde archivo para {}", uuid);

    let path = PathBuf::from(file_path);
    if !path.exists() {
        return Err(format!("El archivo '{}' no existe", file_path));
    }

    let bytes = tokio::fs::read(&path).await.map_err(|e| {
        error!("Error leyendo archivo de skin: {}", e);
        format!("Error leyendo archivo: {}", e)
    })?;

    let (status, body) = send_msa_request(uuid, |token| {
        let file_part = reqwest::multipart::Part::bytes(bytes.clone())
            .file_name("skin.png")
            .mime_str("image/png")
            .expect("mime válido");

        let form = reqwest::multipart::Form::new()
            .text("variant", model.to_uppercase())
            .part("file", file_part);

        HTTP.post(MSA_SKIN_URL).bearer_auth(token).multipart(form)
    })
    .await?;

    if !status.is_success() {
        error!("Error subiendo skin: HTTP {}", status);
        return Err(mojang_error_message(status, body));
    }

    info!("Skin subida correctamente");
    Ok(())
}

pub(crate) async fn upload_skin_url_impl(
    uuid: &str,
    skin_url: &str,
    variant: &str,
) -> Result<(), String> {
    info!("Cambiando skin por URL para {}", uuid);

    let body = serde_json::json!({
        "url": skin_url,
        "variant": variant.to_uppercase(),
    });

    let (status, res_body) = send_msa_request(uuid, |token| {
        HTTP.post(MSA_SKIN_URL).bearer_auth(token).json(&body)
    })
    .await?;

    if !status.is_success() {
        error!("Error cambiando skin por URL: HTTP {}", status);
        return Err(mojang_error_message(status, res_body));
    }

    info!("Skin cambiada por URL correctamente");
    Ok(())
}

#[command]
pub async fn get_minecraft_profile(uuid: String) -> Result<MinecraftProfileResponse, String> {
    get_minecraft_profile_impl(&uuid).await
}

#[command]
pub async fn read_skin_preview_data(file_path: String) -> Result<String, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("El archivo '{}' no existe", file_path));
    }

    let bytes = tokio::fs::read(&path).await.map_err(|e| {
        error!("Error leyendo archivo de skin: {}", e);
        format!("Error leyendo archivo: {}", e)
    })?;

    Ok(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(&bytes)
    ))
}

#[command]
pub async fn upload_skin_file(
    uuid: String,
    file_path: String,
    model: String,
) -> Result<(), String> {
    upload_skin_file_impl(&uuid, &file_path, &model).await?;
    PROFILE_CACHE.invalidate(&uuid);
    capture_active_skin_to_closet(&uuid).await;
    Ok(())
}

#[command]
pub async fn upload_skin_url(
    uuid: String,
    skin_url: String,
    variant: String,
) -> Result<(), String> {
    upload_skin_url_impl(&uuid, &skin_url, &variant).await?;
    PROFILE_CACHE.invalidate(&uuid);
    capture_active_skin_to_closet(&uuid).await;
    Ok(())
}

#[command]
pub async fn equip_cape(uuid: String, cape_id: String) -> Result<(), String> {
    info!("Equipando capa {} para {}", cape_id, uuid);

    let body = serde_json::json!({ "capeId": cape_id });

    let (status, res_body) = send_msa_request(&uuid, |token| {
        HTTP.put(MSA_CAPE_URL).bearer_auth(token).json(&body)
    })
    .await?;

    if !status.is_success() {
        error!("Error equipando capa: HTTP {}", status);
        return Err(mojang_error_message(status, res_body));
    }

    PROFILE_CACHE.invalidate(&uuid);
    info!("Capa equipada correctamente");
    Ok(())
}

#[command]
pub async fn unequip_cape(uuid: String) -> Result<(), String> {
    info!("Desequipando capa para {}", uuid);

    let (status, res_body) =
        send_msa_request(&uuid, |token| HTTP.delete(MSA_CAPE_URL).bearer_auth(token)).await?;

    if !status.is_success() {
        error!("Error desequipando capa: HTTP {}", status);
        return Err(mojang_error_message(status, res_body));
    }

    PROFILE_CACHE.invalidate(&uuid);
    info!("Capa desequipada correctamente");
    Ok(())
}
