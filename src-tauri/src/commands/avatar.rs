use crate::commands::profile::get_minecraft_profile_impl;
use crate::core::http_client::HTTP;
use crate::services::SettingsManager;
use base64::Engine as _;
use base64::engine::general_purpose;
use image::{ImageBuffer, Rgba};
use launchwerk::auth::AccountType;
use reqwest::StatusCode;
use serde::Deserialize;
use std::io::Cursor;
use tracing::{error, info};

const DEFAULT_AVATAR_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" fill="#8a8a8a"/><circle cx="32" cy="24" r="10" fill="#c2c2c2"/><ellipse cx="32" cy="52" rx="18" ry="12" fill="#c2c2c2"/></svg>"##;

#[tauri::command]
pub async fn get_avatar_svg(uuid: String) -> Result<String, String> {
    let user = SettingsManager::read()
        .user
        .iter()
        .find(|u| u.uuid == uuid)
        .cloned()
        .ok_or_else(|| format!("No se encontró el usuario con UUID {}", uuid))?;

    match user.user_type {
        AccountType::Microsoft => render_microsoft_avatar(&uuid).await,
        AccountType::Yggdrasil => {
            let server_url = user
                .yggdrasil_server_url
                .ok_or_else(|| "URL del servidor Yggdrasil no configurada".to_string())?;
            render_yggdrasil_avatar(&uuid, &server_url).await
        }
        AccountType::Cracked => Ok(DEFAULT_AVATAR_SVG.to_string()),
    }
}

async fn render_microsoft_avatar(uuid: &str) -> Result<String, String> {
    let profile = get_minecraft_profile_impl(uuid).await?;
    let active_skin = profile
        .skins
        .into_iter()
        .find(|s| s.state == "ACTIVE")
        .ok_or_else(|| "El perfil no tiene skin activa".to_string())?;
    render_head_svg(&active_skin.url).await
}

#[derive(Debug, Deserialize)]
struct YggdrasilTextureProfile {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[serde(default)]
    properties: Vec<YggdrasilProperty>,
}

#[derive(Debug, Deserialize)]
struct YggdrasilProperty {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct YggdrasilTexturesPayload {
    #[serde(default)]
    textures: std::collections::HashMap<String, YggdrasilTexture>,
}

#[derive(Debug, Deserialize)]
struct YggdrasilTexture {
    url: String,
}

async fn render_yggdrasil_avatar(uuid: &str, server_url: &str) -> Result<String, String> {
    let auth = launchwerk::auth::yggdrasil::YggdrasilAuth::new();
    let api_root = auth
        .resolve_api_url(server_url)
        .await
        .map_err(|e| format!("Error resolviendo API Yggdrasil: {}", e))?;

    let normalized = if !api_root.ends_with('/') {
        format!("{}/", api_root)
    } else {
        api_root.clone()
    };

    let profile_url = format!(
        "{}sessionserver/session/minecraft/profile/{}",
        normalized,
        uuid.replace("-", "")
    );

    info!("Obteniendo perfil Yggdrasil para avatar: {}", profile_url);

    let response = HTTP
        .get(&profile_url)
        .send()
        .await
        .map_err(|e| format!("Error de red al obtener perfil Yggdrasil: {}", e))?;

    if response.status() == StatusCode::NO_CONTENT {
        return Ok(DEFAULT_AVATAR_SVG.to_string());
    }

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Error leyendo respuesta Yggdrasil: {}", e))?;

    if !status.is_success() {
        error!(
            "Error obteniendo perfil Yggdrasil: HTTP {} - {}",
            status, body
        );
        return Ok(DEFAULT_AVATAR_SVG.to_string());
    }

    let profile: YggdrasilTextureProfile = serde_json::from_str(&body)
        .map_err(|e| format!("Respuesta de perfil Yggdrasil inválida: {}", e))?;

    let textures_b64 = profile
        .properties
        .into_iter()
        .find(|p| p.name == "textures")
        .map(|p| p.value);

    let Some(textures_b64) = textures_b64 else {
        return Ok(DEFAULT_AVATAR_SVG.to_string());
    };

    let decoded = general_purpose::STANDARD
        .decode(textures_b64)
        .map_err(|e| format!("Error decodificando texturas Yggdrasil: {}", e))?;

    let payload: YggdrasilTexturesPayload = serde_json::from_slice(&decoded)
        .map_err(|e| format!("Error parseando texturas Yggdrasil: {}", e))?;

    let skin_url = payload
        .textures
        .get("SKIN")
        .map(|t| t.url.clone())
        .ok_or_else(|| "El perfil Yggdrasil no tiene skin".to_string())?;

    render_head_svg(&skin_url).await
}

async fn render_head_svg(skin_url: &str) -> Result<String, String> {
    let response = HTTP
        .get(skin_url)
        .send()
        .await
        .map_err(|e| format!("Error descargando skin: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Error descargando skin: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Error leyendo skin: {}", e))?;

    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("Error decodificando imagen de skin: {}", e))?
        .to_rgba8();

    let (width, _height) = (img.width(), img.height());
    if width == 0 || width % 64 != 0 {
        return Err("La imagen de skin tiene dimensiones inesperadas".to_string());
    }

    let scale = width / 64;
    let head_size = 8 * scale;

    let head_x = 8 * scale;
    let head_y = 8 * scale;
    let hat_x = 40 * scale;
    let hat_y = 8 * scale;

    let mut head = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(head_size, head_size);

    for y in 0..head_size {
        for x in 0..head_size {
            let px = img.get_pixel(head_x + x, head_y + y);
            head.put_pixel(x, y, *px);
        }
    }

    for y in 0..head_size {
        for x in 0..head_size {
            let px = img.get_pixel(hat_x + x, hat_y + y);
            if px[3] > 0 {
                head.put_pixel(x, y, *px);
            }
        }
    }

    let output_size = 128u32;
    let scaled = image::imageops::resize(&head, output_size, output_size, image::imageops::Nearest);

    let mut png_buf = Cursor::new(Vec::new());
    scaled
        .write_to(&mut png_buf, image::ImageFormat::Png)
        .map_err(|e| format!("Error codificando PNG del avatar: {}", e))?;

    let b64 = general_purpose::STANDARD.encode(png_buf.into_inner());

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" width="100%" height="100%"><image href="data:image/png;base64,{b64}" width="{size}" height="{size}"/></svg>"#,
        size = output_size,
        b64 = b64
    ))
}
