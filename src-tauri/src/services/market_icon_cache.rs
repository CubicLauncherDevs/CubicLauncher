//! `cubic-market-icon://` sirve los iconos del market desde una caché en disco.
//!
//! El WebView ya cachea respuestas HTTP, pero esa caché no tiene un tope
//! explícito ni garantiza disponibilidad entre reinicios. Acá se guarda una
//! copia acotada a 4 MB o 512 iconos (se descarta lo más antiguo por fecha de
//! modificación) y se responde desde disco cuando existe. La URL remota viaja
//! como único segmento del path, percent-encoded, y solo se aceptan los CDN
//! conocidos: así el protocolo no puede usarse como proxy hacia la red local.

use crate::core::PathManager;
use crate::core::http_client::HTTP;
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::http::{Request, Response, StatusCode, Uri, header};
use tokio::sync::Semaphore;

pub(crate) const SCHEME: &str = "cubic-market-icon";

/// Keep in sync with `CACHEABLE_HOSTS` in `src/lib/utils/marketIcons.ts`.
const ALLOWED_HOSTS: [&str; 3] = [
    "cdn.modrinth.com",
    "media.forgecdn.net",
    "edge.forgecdn.net",
];

/// Un icono de proyecto real pesa entre 2 y 20 KB; el tope solo cubre outliers.
const MAX_ICON_BYTES: usize = 1024 * 1024;
/// Presupuesto de la caché: ~800 iconos típicos o 512 entradas, lo que llegue
/// primero. Es descartable, así que conviene que no crezca de más.
const MAX_CACHE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CACHE_ENTRIES: usize = 512;
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);
const HASH_BYTES: usize = 16;
/// Revisar el presupuesto en cada descarga obliga a recorrer el directorio;
/// una vez por minuto alcanza y deja el costo de inserción en O(1).
const PRUNE_INTERVAL_SECS: u64 = 60;

/// Los pedidos de la grilla llegan en ráfaga; limitar las descargas evita
/// abrir decenas de conexiones al mismo CDN.
static FETCH_WORKERS: Semaphore = Semaphore::const_new(4);
static LAST_PRUNE: AtomicU64 = AtomicU64::new(0);

/// Resuelve un pedido del protocolo: disco primero, descarga y guarda si falta.
pub(crate) async fn respond(request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Some(remote) = parse(request.uri()) else {
        return not_found();
    };
    let Some(path) = cache_path(&remote) else {
        return not_found();
    };

    if let Some(bytes) = read_cached(&path).await {
        return serve(bytes);
    }

    let Ok(_permit) = FETCH_WORKERS.acquire().await else {
        return not_found();
    };
    let Ok(bytes) = fetch(&remote).await else {
        return not_found();
    };
    // Solo se cachean imágenes: una página de error del CDN no debe quedar
    // guardada y repetirse para siempre.
    if image_mime(&bytes).is_none() {
        return not_found();
    }
    // Un fallo de escritura no debe ocultar el icono recién descargado.
    if let Err(error) = store(&path, &bytes).await {
        tracing::warn!("No se pudo cachear el icono {}: {}", remote, error);
    }
    serve(bytes)
}

/// La URL remota es el único segmento del path, tal como la arma
/// `convertFileSrc(icon, "cubic-market-icon")`.
fn parse(uri: &Uri) -> Option<String> {
    let encoded = uri.path().trim_start_matches('/');
    if encoded.is_empty() || encoded.contains('/') {
        return None;
    }
    let remote = urlencoding::decode(encoded).ok()?.into_owned();
    let parsed = url::Url::parse(&remote).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    ALLOWED_HOSTS
        .contains(&parsed.host_str()?)
        .then_some(remote)
}

fn cache_path(remote: &str) -> Option<PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(remote.as_bytes());
    let digest = hasher.finalize();
    let mut name = String::with_capacity(HASH_BYTES * 2);
    for byte in &digest[..HASH_BYTES] {
        write!(&mut name, "{byte:02x}").ok()?;
    }
    Some(
        PathManager::get()
            .get_cache_dir()
            .join("market-icons")
            .join(name),
    )
}

async fn read_cached(path: &Path) -> Option<Vec<u8>> {
    tokio::fs::read(path).await.ok()
}

async fn fetch(remote: &str) -> Result<Vec<u8>, ()> {
    let response = HTTP
        .get(remote)
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .map_err(|_| ())?;
    if !response.status().is_success() {
        return Err(());
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_ICON_BYTES as u64)
    {
        return Err(());
    }
    let bytes = response.bytes().await.map_err(|_| ())?;
    if bytes.is_empty() || bytes.len() > MAX_ICON_BYTES {
        return Err(());
    }
    Ok(bytes.to_vec())
}

async fn store(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let Some(dir) = path.parent() else {
        return Ok(());
    };
    tokio::fs::create_dir_all(dir).await?;
    // Escritura atómica: un icono a medio guardar nunca se sirve.
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await?;
    maybe_prune(dir).await;
    Ok(())
}

/// Evita recorrer el directorio en cada icono: como mucho una poda por minuto.
async fn maybe_prune(dir: &Path) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let last = LAST_PRUNE.load(Ordering::Relaxed);
    if now.saturating_sub(last) < PRUNE_INTERVAL_SECS {
        return;
    }
    // Solo un pedido gana la poda; el resto sigue sin bloquearse.
    if LAST_PRUNE
        .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        prune(dir).await;
    }
}

/// Descarta los archivos más antiguos hasta cumplir ambos topes.
async fn prune(dir: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return;
    };
    let mut files: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    let mut total = 0u64;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let Ok(metadata) = entry.metadata().await else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        total += metadata.len();
        files.push((
            metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            metadata.len(),
            entry.path(),
        ));
    }
    if total <= MAX_CACHE_BYTES && files.len() <= MAX_CACHE_ENTRIES {
        return;
    }
    files.sort_unstable_by_key(|(modified, _, _)| *modified);
    let mut count = files.len();
    for (_, len, path) in files {
        if total <= MAX_CACHE_BYTES && count <= MAX_CACHE_ENTRIES {
            break;
        }
        if tokio::fs::remove_file(&path).await.is_ok() {
            total -= len;
            count -= 1;
        }
    }
}

/// El tipo se detecta por contenido: lo que no sea una imagen no se cachea ni
/// se sirve, así una página de error del CDN no termina como icono roto.
fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF8") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn serve(bytes: Vec<u8>) -> Response<Vec<u8>> {
    let Some(mime) = image_mime(&bytes) else {
        return not_found();
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=604800")
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .body(bytes)
        .unwrap_or_else(|_| not_found())
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CACHE_CONTROL, "no-store")
        .body(Vec::new())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

#[cfg(test)]
#[path = "../tests/services/market_icon_cache.rs"]
mod tests;
