//! Selección del proveedor de descargas (servidor oficial vs. espejo BMCLAPI).
//!
//! BMCLAPI (https://bmclapi2.bangbang93.com) es un espejo pensado para jugadores
//! en China, donde los servidores oficiales de Mojang pueden ser muy lentos.
//! No espeja Quilt ni los runtimes de Java, así que esas descargas permanecen en
//! el servidor oficial.

use std::sync::LazyLock;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{Semaphore, SemaphorePermit};

pub const BMCLAPI_ROOT: &str = "https://bmclapi2.bangbang93.com";

/// Máximo de descargas concurrentes permitidas contra el espejo BMCLAPI.
///
/// El WAF de BMCLAPI responde `429` cuando se superan unas pocas peticiones
/// simultáneas, así que limitamos la concurrencia solo para las URLs espejadas.
/// El servidor oficial no está sujeto a este límite.
const MIRROR_MAX_CONCURRENCY: usize = 4;

/// Cooldown por defecto (segundos) cuando el `429` no trae `Retry-After`.
const MIRROR_COOLDOWN_SECS_DEFAULT: u64 = 30;

/// Tope superior del cooldown (segundos) para no bloquear el espejo demasiado.
const MIRROR_COOLDOWN_SECS_MAX: u64 = 120;

/// Proveedor de descargas elegido por el usuario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadProvider {
    /// Servidores oficiales de Mojang/Forge/Fabric/NeoForge/OptiFine.
    Official,
    /// Espejo BMCLAPI, con fallback automático al servidor oficial.
    Bmclapi,
}

impl DownloadProvider {
    /// Convierte un identificador de configuración en un proveedor.
    pub fn from_id(id: &str) -> Self {
        match id {
            "bmclapi" => Self::Bmclapi,
            _ => Self::Official,
        }
    }

    /// Identificador estable usado en la configuración.
    pub fn id(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Bmclapi => "bmclapi",
        }
    }
}

static PROVIDER: AtomicU8 = AtomicU8::new(0);

/// Establece el proveedor global usado por el downloader de `aqua`.
pub fn set_download_provider(provider: DownloadProvider) {
    PROVIDER.store(provider as u8, Ordering::Relaxed);
}

/// Devuelve el proveedor global actual.
pub fn download_provider() -> DownloadProvider {
    match PROVIDER.load(Ordering::Relaxed) {
        1 => DownloadProvider::Bmclapi,
        _ => DownloadProvider::Official,
    }
}

/// Semáforo que limita las descargas concurrentes contra el espejo BMCLAPI.
static MIRROR_SEMAPHORE: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(MIRROR_MAX_CONCURRENCY));

/// Instante (epoch ms) hasta el que el espejo permanece "abierto" por rate-limit.
/// `0` significa que no hay cooldown activo.
static MIRROR_OPEN_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// `true` si la URL apunta al espejo BMCLAPI (y por tanto está sujeta al límite).
pub(crate) fn is_mirror_url(url: &str) -> bool {
    url.starts_with(BMCLAPI_ROOT)
}

/// Indica si el espejo está en cooldown por haber recibido un `429`.
pub(crate) fn mirror_cooldown_active() -> bool {
    now_ms() < MIRROR_OPEN_UNTIL_MS.load(Ordering::Relaxed)
}

/// Adquiere un permiso de concurrencia para el espejo.
///
/// Devuelve `None` si el espejo está en cooldown, de modo que el llamador use
/// directamente el servidor oficial. En otro caso espera a que haya hueco.
pub(crate) async fn acquire_mirror_permit() -> Option<SemaphorePermit<'static>> {
    if mirror_cooldown_active() {
        return None;
    }
    MIRROR_SEMAPHORE.acquire().await.ok()
}

/// Registra un `429` del espejo y abre el circuito durante `retry_after_secs`.
///
/// Un valor `0` (cabecera ausente) aplica el cooldown por defecto.
pub(crate) fn note_mirror_rate_limited(retry_after_secs: u64) {
    let secs = if retry_after_secs == 0 {
        MIRROR_COOLDOWN_SECS_DEFAULT
    } else {
        retry_after_secs.min(MIRROR_COOLDOWN_SECS_MAX)
    };
    let until = now_ms() + secs * 1000;
    MIRROR_OPEN_UNTIL_MS.fetch_max(until, Ordering::Relaxed);
}

/// Reglas de espejo: (`prefijo oficial`, `prefijo BMCLAPI`).
///
/// El remanente de la URL tras el prefijo se concatena al destino. Se ordenan
/// de la más específica a la más genérica. El host oficial no incluye barra
/// final para que el remanente siempre empiece por `/`.
const MIRROR_RULES: &[(&str, &str)] = &[
    (
        "https://resources.download.minecraft.net",
        "https://bmclapi2.bangbang93.com/assets",
    ),
    (
        "https://libraries.minecraft.net",
        "https://bmclapi2.bangbang93.com/libraries",
    ),
    (
        "https://maven.neoforged.net/releases",
        "https://bmclapi2.bangbang93.com/maven",
    ),
    (
        "https://maven.minecraftforge.net",
        "https://bmclapi2.bangbang93.com/maven",
    ),
    (
        "https://files.minecraftforge.net/maven",
        "https://bmclapi2.bangbang93.com/maven",
    ),
    (
        "http://files.minecraftforge.net/maven",
        "https://bmclapi2.bangbang93.com/maven",
    ),
    (
        "https://maven.fabricmc.net",
        "https://bmclapi2.bangbang93.com/maven",
    ),
    (
        "https://meta.fabricmc.net",
        "https://bmclapi2.bangbang93.com/fabric-meta",
    ),
    (
        "https://piston-meta.mojang.com",
        "https://bmclapi2.bangbang93.com",
    ),
    (
        "https://piston-data.mojang.com",
        "https://bmclapi2.bangbang93.com",
    ),
    (
        "https://launchermeta.mojang.com",
        "https://bmclapi2.bangbang93.com",
    ),
    (
        "https://launcher.mojang.com",
        "https://bmclapi2.bangbang93.com",
    ),
];

/// Traduce una URL oficial a su equivalente en BMCLAPI, si existe una regla.
///
/// Devuelve `None` para URLs que BMCLAPI no espeja (p. ej. Quilt, foojay, JRE).
pub fn mirror_url(url: &str) -> Option<String> {
    for (source, target) in MIRROR_RULES {
        if let Some(rest) = url.strip_prefix(source) {
            return Some(format!("{target}{rest}"));
        }
    }
    None
}

/// Devuelve las URLs a intentar, en orden, según el proveedor activo.
///
/// Con BMCLAPI se intenta primero el espejo y luego el oficial como respaldo;
/// con el proveedor oficial solo se devuelve la URL original.
pub fn preferred_urls(url: &str) -> Vec<String> {
    if download_provider() == DownloadProvider::Bmclapi
        && !mirror_cooldown_active()
        && let Some(mirror) = mirror_url(url)
        && mirror != url
    {
        return vec![mirror, url.to_string()];
    }
    vec![url.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_mojang_and_loader_hosts() {
        assert_eq!(
            mirror_url("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"),
            Some("https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json".into())
        );
        assert_eq!(
            mirror_url("https://resources.download.minecraft.net/ab/abcdef"),
            Some("https://bmclapi2.bangbang93.com/assets/ab/abcdef".into())
        );
        assert_eq!(
            mirror_url("https://libraries.minecraft.net/net/foo/bar.jar"),
            Some("https://bmclapi2.bangbang93.com/libraries/net/foo/bar.jar".into())
        );
        assert_eq!(
            mirror_url("https://maven.minecraftforge.net/net/minecraftforge/forge/x-installer.jar"),
            Some(
                "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/x-installer.jar"
                    .into()
            )
        );
        assert_eq!(
            mirror_url("https://maven.neoforged.net/releases/net/neoforged/neoforge/x.jar"),
            Some("https://bmclapi2.bangbang93.com/maven/net/neoforged/neoforge/x.jar".into())
        );
        assert_eq!(
            mirror_url("https://meta.fabricmc.net/v2/versions/loader/1.20.1"),
            Some("https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/1.20.1".into())
        );
        assert_eq!(
            mirror_url("https://maven.fabricmc.net/net/fabricmc/fabric-loader/x.jar"),
            Some("https://bmclapi2.bangbang93.com/maven/net/fabricmc/fabric-loader/x.jar".into())
        );
    }

    #[test]
    fn leaves_unsupported_hosts_untouched() {
        assert_eq!(
            mirror_url("https://meta.quiltmc.org/v3/versions/loader"),
            None
        );
        assert_eq!(mirror_url("https://api.foojay.io/disco/v3.0"), None);
        assert_eq!(mirror_url("https://api.adoptium.net/v3/assets"), None);
    }

    #[test]
    fn provider_ids_roundtrip() {
        assert_eq!(DownloadProvider::from_id("bmclapi").id(), "bmclapi");
        assert_eq!(DownloadProvider::from_id("official").id(), "official");
        assert_eq!(DownloadProvider::from_id("").id(), "official");
        assert_eq!(DownloadProvider::from_id("auto").id(), "official");
    }

    #[test]
    fn detects_mirror_urls() {
        assert!(is_mirror_url(
            "https://bmclapi2.bangbang93.com/assets/ab/abcdef"
        ));
        assert!(!is_mirror_url(
            "https://resources.download.minecraft.net/ab/abcdef"
        ));
        assert!(!is_mirror_url(""));
    }

    #[test]
    fn cooldown_falls_back_to_official_only() {
        let asset = "https://resources.download.minecraft.net/ab/abcdef";

        set_download_provider(DownloadProvider::Bmclapi);
        assert_eq!(
            preferred_urls(asset),
            vec![
                "https://bmclapi2.bangbang93.com/assets/ab/abcdef".to_string(),
                asset.to_string()
            ]
        );

        note_mirror_rate_limited(30);
        assert!(mirror_cooldown_active());
        assert_eq!(preferred_urls(asset), vec![asset.to_string()]);

        MIRROR_OPEN_UNTIL_MS.store(0, Ordering::Relaxed);
        set_download_provider(DownloadProvider::Official);
        assert!(!mirror_cooldown_active());
        assert_eq!(preferred_urls(asset), vec![asset.to_string()]);
    }
}
