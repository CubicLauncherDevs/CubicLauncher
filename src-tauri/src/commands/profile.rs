use crate::core::http_client::HTTP;
use crate::services::SettingsManager;
use launchwerk::auth::AccountType;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::command;
use tracing::{error, info};

const MSA_SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const MSA_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MSA_CAPE_URL: &str = "https://api.minecraftservices.com/minecraft/profile/capes/active";

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

fn mojang_error_message(status: reqwest::StatusCode, body: String) -> String {
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
fn load_minecraft_token(uuid: &str) -> Result<String, String> {
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

#[command]
pub async fn get_minecraft_profile(uuid: String) -> Result<MinecraftProfileResponse, String> {
    info!("Obteniendo perfil de Minecraft Services para {}", uuid);

    let access_token = load_minecraft_token(&uuid)?;

    let res = HTTP
        .get(MSA_PROFILE_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| {
            error!("Error de red obteniendo perfil: {}", e);
            format!("Error de red: {}", e)
        })?;

    let status = res.status();
    let body = res.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        error!("Error obteniendo perfil: HTTP {}", status);
        return Err(mojang_error_message(status, body));
    }

    serde_json::from_str(&body).map_err(|e| {
        error!("Error parseando perfil: {}", e);
        format!("Respuesta inválida: {}", e)
    })
}

#[command]
pub async fn upload_skin_file(
    uuid: String,
    file_path: String,
    model: String,
) -> Result<(), String> {
    info!("Subiendo skin desde archivo para {}", uuid);

    let access_token = load_minecraft_token(&uuid)?;

    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("El archivo '{}' no existe", file_path));
    }

    let bytes = tokio::fs::read(&path).await.map_err(|e| {
        error!("Error leyendo archivo de skin: {}", e);
        format!("Error leyendo archivo: {}", e)
    })?;

    let file_part = reqwest::multipart::Part::bytes(bytes)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .text("variant", model.to_uppercase())
        .part("file", file_part);

    let res = HTTP
        .post(MSA_SKIN_URL)
        .bearer_auth(access_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Error de red: {}", e))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        error!("Error subiendo skin: HTTP {}", status);
        return Err(mojang_error_message(status, body));
    }

    info!("Skin subida correctamente");
    Ok(())
}

#[command]
pub async fn upload_skin_url(
    uuid: String,
    skin_url: String,
    variant: String,
) -> Result<(), String> {
    info!("Cambiando skin por URL para {}", uuid);

    let access_token = load_minecraft_token(&uuid)?;

    let body = serde_json::json!({
        "url": skin_url,
        "variant": variant.to_uppercase(),
    });

    let res = HTTP
        .post(MSA_SKIN_URL)
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error de red: {}", e))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        error!("Error cambiando skin por URL: HTTP {}", status);
        return Err(mojang_error_message(status, body));
    }

    info!("Skin cambiada por URL correctamente");
    Ok(())
}

#[command]
pub async fn equip_cape(uuid: String, cape_id: String) -> Result<(), String> {
    info!("Equipando capa {} para {}", cape_id, uuid);

    let access_token = load_minecraft_token(&uuid)?;

    let body = serde_json::json!({ "capeId": cape_id });

    let res = HTTP
        .put(MSA_CAPE_URL)
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error de red: {}", e))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        error!("Error equipando capa: HTTP {}", status);
        return Err(mojang_error_message(status, body));
    }

    info!("Capa equipada correctamente");
    Ok(())
}

#[command]
pub async fn unequip_cape(uuid: String) -> Result<(), String> {
    info!("Desequipando capa para {}", uuid);

    let access_token = load_minecraft_token(&uuid)?;

    let res = HTTP
        .delete(MSA_CAPE_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Error de red: {}", e))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        error!("Error desequipando capa: HTTP {}", status);
        return Err(mojang_error_message(status, body));
    }

    info!("Capa desequipada correctamente");
    Ok(())
}
