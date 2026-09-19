use crate::{core::http_client::HTTP, services::SettingsManager};
use base64::{Engine as _, engine::general_purpose};
use image::ImageDecoder;
use launchwerk::auth::AccountType;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

const MAX_SKIN_BYTES: usize = 8 * 1024 * 1024;

#[derive(Serialize)]
pub struct ElySkinProfile {
    skin_url: Option<String>,
    model: &'static str,
}

#[derive(Default, Deserialize)]
struct Textures {
    #[serde(rename = "SKIN")]
    skin: Option<SkinTexture>,
}

#[derive(Deserialize)]
struct SkinTexture {
    #[serde(default)]
    metadata: SkinMetadata,
}

#[derive(Default, Deserialize)]
struct SkinMetadata {
    model: Option<String>,
}

fn is_ely_server(server: &str) -> bool {
    let server = server.trim();
    let normalized = if server.contains("://") {
        server.to_string()
    } else {
        format!("https://{server}")
    };
    url::Url::parse(&normalized).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.port().is_none()
            && matches!(
                url.host_str(),
                Some("ely.by" | "www.ely.by" | "authserver.ely.by" | "account.ely.by")
            )
    })
}

async fn download(url: &str, limit: usize) -> Result<Option<Vec<u8>>, String> {
    let mut response = HTTP
        .get(url)
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .send()
        .await
        .map_err(|e| format!("Error de conexión con Ely.by: {e}"))?;
    if matches!(
        response.status(),
        StatusCode::NO_CONTENT | StatusCode::NOT_FOUND
    ) {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(format!("Error de Ely.by (HTTP {})", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|len| len > limit as u64)
    {
        return Err("La respuesta de Ely.by supera el tamaño permitido".to_string());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if chunk.len() > limit - bytes.len() {
            return Err("La respuesta de Ely.by supera el tamaño permitido".to_string());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(Some(bytes))
}

fn skin_data_url(bytes: &[u8]) -> Result<String, String> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(64);
    limits.max_image_height = Some(64);
    limits.max_alloc = Some(1024 * 1024);
    let decoder = image::codecs::png::PngDecoder::with_limits(Cursor::new(bytes), limits)
        .map_err(|e| format!("Skin de Ely.by inválida: {e}"))?;
    if !matches!(decoder.dimensions(), (64, 32) | (64, 64)) {
        return Err("La skin de Ely.by debe medir 64x32 o 64x64".to_string());
    }
    // Decode once to reject incomplete/corrupt images before giving them to the viewer.
    image::DynamicImage::from_decoder(decoder)
        .map_err(|e| format!("Skin de Ely.by inválida: {e}"))?;
    Ok(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(bytes)
    ))
}

#[tauri::command]
pub async fn get_ely_skin_profile(uuid: String) -> Result<ElySkinProfile, String> {
    let user = SettingsManager::read()
        .user
        .iter()
        .find(|u| u.uuid == uuid)
        .cloned()
        .ok_or_else(|| "No se encontró la cuenta".to_string())?;
    if user.user_type != AccountType::Yggdrasil
        || !user
            .yggdrasil_server_url
            .as_deref()
            .is_some_and(is_ely_server)
    {
        return Err("Esta cuenta no pertenece a Ely.by".to_string());
    }

    // These are public, documented endpoints. Minecraft session tokens are not
    // website sessions and must not be sent to the skins catalog.
    let name = urlencoding::encode(&user.username);
    let textures = download(
        &format!("https://skinsystem.ely.by/textures/{name}"),
        64 * 1024,
    )
    .await?;
    let textures: Textures = match textures {
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Respuesta de Ely.by inválida: {e}"))?,
        None => Textures::default(),
    };
    let model = if textures
        .skin
        .as_ref()
        .and_then(|skin| skin.metadata.model.as_deref())
        == Some("slim")
    {
        "slim"
    } else {
        "classic"
    };
    // Fetch through Ely.by's fixed endpoint, avoiding remote texture URLs and CORS.
    let skin_url = if textures.skin.is_some() {
        download(
            &format!("https://skinsystem.ely.by/skins/{name}.png"),
            MAX_SKIN_BYTES,
        )
        .await?
        .map(|bytes| skin_data_url(&bytes))
        .transpose()?
    } else {
        None
    };
    Ok(ElySkinProfile { skin_url, model })
}

#[cfg(test)]
#[path = "../tests/commands/ely_profile.rs"]
mod tests;
