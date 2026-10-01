use crate::errors::AquaError;
use crate::jre::types::{ArchiveFormat, JrePackage};
use crate::utilities::HTTP_CLIENT;
use serde::Deserialize;

/// Base de la API Disco de foojay. Agrega builds de OpenJDK de distintas
/// distribuciones con un formato uniforme, por lo que cada proveedor nuevo
/// solo necesita declarar su identificador y tipo de paquete.
const FOOJAY_API: &str = "https://api.foojay.io/disco/v3.0";

/// Supported JRE vendors. The chain tries each vendor in order until one
/// returns a valid package, providing resilience if a provider is down or
/// does not ship the requested major version for the current platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JreVendor {
    Zulu,
    Temurin,
    Liberica,
    Corretto,
    Microsoft,
    GraalVm,
    Semeru,
}

impl JreVendor {
    /// Orden en el que se listan los proveedores en la interfaz.
    pub const ALL: &[JreVendor] = &[
        Self::Zulu,
        Self::Temurin,
        Self::Liberica,
        Self::GraalVm,
        Self::Corretto,
        Self::Microsoft,
        Self::Semeru,
    ];

    /// Identificador estable usado por los ajustes y la interfaz.
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Zulu => "zulu",
            Self::Temurin => "temurin",
            Self::Liberica => "liberica",
            Self::Corretto => "corretto",
            Self::Microsoft => "microsoft",
            Self::GraalVm => "graalvm",
            Self::Semeru => "semeru",
        }
    }

    /// Resuelve un identificador persistido. Devuelve `None` para valores
    /// desconocidos, de modo que el llamador use la cadena por defecto.
    pub fn from_id(id: &str) -> Option<Self> {
        let id = id.trim();
        Self::ALL
            .iter()
            .copied()
            .find(|vendor| vendor.id().eq_ignore_ascii_case(id))
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Self::Zulu => "Azul Zulu",
            Self::Temurin => "Eclipse Temurin",
            Self::Liberica => "BellSoft Liberica",
            Self::Corretto => "Amazon Corretto",
            Self::Microsoft => "Microsoft Build of OpenJDK",
            Self::GraalVm => "GraalVM",
            Self::Semeru => "IBM Semeru (OpenJ9)",
        }
    }

    /// Identificador de la distribución en la API de foojay. `None` para los
    /// proveedores que se resuelven con su propia API.
    const fn foojay_slug(&self) -> Option<&'static str> {
        match self {
            Self::Zulu | Self::Temurin => None,
            Self::Liberica => Some("liberica"),
            Self::Corretto => Some("corretto"),
            Self::Microsoft => Some("microsoft"),
            Self::GraalVm => Some("graalvm"),
            Self::Semeru => Some("semeru"),
        }
    }

    /// Tipo de paquete publicado por el proveedor. Varios solo ofrecen JDK.
    const fn foojay_package_type(&self) -> &'static str {
        match self {
            Self::Corretto | Self::Microsoft | Self::GraalVm => "jdk",
            _ => "jre",
        }
    }

    /// Reconoce la distribución a partir de la salida de `java -version`.
    /// Los marcadores se comprueban en orden porque algunas salidas incluyen
    /// más de un nombre de proveedor.
    pub fn detect(version_output: &str) -> Option<Self> {
        const MARKERS: &[(&str, JreVendor)] = &[
            ("graalvm", JreVendor::GraalVm),
            ("corretto", JreVendor::Corretto),
            ("microsoft", JreVendor::Microsoft),
            ("temurin", JreVendor::Temurin),
            ("zulu", JreVendor::Zulu),
            ("semeru", JreVendor::Semeru),
            ("openj9", JreVendor::Semeru),
            ("liberica", JreVendor::Liberica),
            ("bellsoft", JreVendor::Liberica),
        ];

        let lower = version_output.to_ascii_lowercase();
        MARKERS
            .iter()
            .find(|(marker, _)| lower.contains(marker))
            .map(|(_, vendor)| *vendor)
    }

    pub async fn resolve(self, major_version: u8) -> Result<JrePackage, AquaError> {
        match self.foojay_slug() {
            Some(_) => resolve_foojay(self, major_version).await,
            None => match self {
                Self::Zulu => resolve_zulu(major_version, current_os(), current_arch()).await,
                Self::Temurin => {
                    resolve_adoptium(major_version, current_os(), current_arch()).await
                }
                _ => Err(AquaError::Other(format!(
                    "Proveedor sin resolver: {}",
                    self.id()
                ))),
            },
        }
    }
}

/// Default provider chain used by the launcher. Tries the fastest/most
/// reliable provider first and falls back to the next one on failure.
pub struct JreProviderChain;

impl JreProviderChain {
    const FALLBACKS: &[JreVendor] = &[JreVendor::Zulu, JreVendor::Temurin];

    pub async fn get_latest_package(major_version: u8) -> Result<JrePackage, AquaError> {
        Self::get_package(major_version, None).await
    }

    /// Resolves the package for the preferred vendor and, when it fails or is
    /// not provided, falls back to the default chain.
    pub async fn get_package(
        major_version: u8,
        preferred: Option<JreVendor>,
    ) -> Result<JrePackage, AquaError> {
        let mut last_err = None;
        for vendor in preferred.into_iter().chain(Self::FALLBACKS.iter().copied()) {
            match vendor.resolve(major_version).await {
                Ok(pkg) => return Ok(pkg),
                Err(e) => {
                    log::warn!("JRE provider {} failed: {}", vendor.name(), e);
                    last_err = Some(e);
                }
            }
        }
        Err(last_err
            .unwrap_or_else(|| AquaError::Other("No JRE provider returned a package".into())))
    }

    /// Proveedores que publican un build descargable para la versión y
    /// plataforma actuales. Los proveedores con API propia se consideran
    /// siempre disponibles.
    pub async fn available_vendors(major_version: u8) -> Result<Vec<JreVendor>, AquaError> {
        let url = format!(
            "{FOOJAY_API}/distributions?version={}&operating_system={}&architecture={}&include_versions=false",
            major_version,
            current_os(),
            foojay_arch()
        );

        log::info!("Querying foojay distributions: {}", url);

        let distributions: FoojayDistributions = HTTP_CLIENT
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(JreVendor::ALL
            .iter()
            .copied()
            .filter(|vendor| {
                vendor.foojay_slug().is_none_or(|slug| {
                    distributions
                        .result
                        .iter()
                        .any(|distribution| distribution.api_parameter == slug)
                })
            })
            .collect())
    }
}

fn current_os() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    }
}

/// Nomenclatura de arquitecturas de foojay (distinta de la que usan las APIs
/// de Zulu y Adoptium).
fn foojay_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    }
}

fn foojay_archive_type() -> &'static str {
    if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    }
}

fn foojay_packages_url(vendor: JreVendor, major_version: u8) -> String {
    format!(
        "{FOOJAY_API}/packages?version={}&operating_system={}&architecture={}&package_type={}&distribution={}&archive_type={}&latest=available&directly_downloadable=true&release_status=ga&javafx_bundled=false",
        major_version,
        current_os(),
        foojay_arch(),
        vendor.foojay_package_type(),
        vendor.foojay_slug().unwrap_or_default(),
        foojay_archive_type()
    )
}

#[derive(Debug, Deserialize)]
struct FoojayPackages {
    result: Vec<FoojayPackage>,
}

#[derive(Debug, Deserialize)]
struct FoojayPackage {
    java_version: String,
    archive_type: String,
    #[serde(default)]
    lib_c_type: Option<String>,
    filename: String,
    #[serde(default)]
    size: Option<u64>,
    links: FoojayLinks,
}

impl FoojayPackage {
    fn is_musl(&self) -> bool {
        matches!(self.lib_c_type.as_deref(), Some("musl") | Some("musl_libc"))
    }
}

#[derive(Debug, Deserialize)]
struct FoojayLinks {
    pkg_info_uri: String,
    #[serde(default)]
    pkg_download_redirect: String,
}

#[derive(Debug, Deserialize)]
struct FoojayInfo {
    result: Vec<FoojayInfoEntry>,
}

#[derive(Debug, Deserialize)]
struct FoojayInfoEntry {
    #[serde(default)]
    direct_download_uri: String,
    #[serde(default)]
    checksum: Option<String>,
    #[serde(default)]
    checksum_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FoojayDistributions {
    result: Vec<FoojayDistribution>,
}

#[derive(Debug, Deserialize)]
struct FoojayDistribution {
    api_parameter: String,
}

/// Prefiere los builds glibc sobre los musl, que comparten plataforma y
/// versión pero no son compatibles con todas las distribuciones de Linux.
fn select_foojay_package(mut packages: Vec<FoojayPackage>) -> Option<FoojayPackage> {
    packages.sort_by_key(|package| u8::from(package.is_musl()));
    packages.into_iter().next()
}

fn archive_format(value: &str) -> ArchiveFormat {
    if value == "zip" {
        ArchiveFormat::Zip
    } else {
        ArchiveFormat::TarGz
    }
}

async fn resolve_foojay(vendor: JreVendor, major_version: u8) -> Result<JrePackage, AquaError> {
    let url = foojay_packages_url(vendor, major_version);

    log::info!("Querying foojay packages: {}", url);

    let packages: FoojayPackages = HTTP_CLIENT
        .get(&url)
        .header("accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let package = select_foojay_package(packages.result).ok_or_else(|| {
        AquaError::Other(format!(
            "No {} build found for Java {}",
            vendor.name(),
            major_version
        ))
    })?;

    let (download_url, sha256_hash) = fetch_foojay_download(&package).await?;

    Ok(JrePackage {
        major_version,
        java_version: package.java_version.clone(),
        download_url,
        filename: package.filename.clone(),
        distro_version: Vec::new(),
        sha256_hash,
        vendor: vendor.name(),
        archive_format: archive_format(&package.archive_type),
        size: package.size,
    })
}

/// El listado de paquetes no incluye el enlace directo ni el checksum: se
/// obtienen del detalle de cada paquete.
async fn fetch_foojay_download(
    package: &FoojayPackage,
) -> Result<(String, Option<String>), AquaError> {
    let info: FoojayInfo = HTTP_CLIENT
        .get(&package.links.pkg_info_uri)
        .header("accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let entry = info.result.into_iter().next().ok_or_else(|| {
        AquaError::Other(format!("No download link found for {}", package.filename))
    })?;

    let download_url = if entry.direct_download_uri.is_empty() {
        package.links.pkg_download_redirect.clone()
    } else {
        entry.direct_download_uri
    };

    // El descargador verifica SHA-256; se descarta cualquier otro algoritmo.
    let sha256_hash = match (entry.checksum, entry.checksum_type.as_deref()) {
        (Some(checksum), Some("sha256")) if !checksum.is_empty() => Some(checksum),
        _ => None,
    };

    Ok((download_url, sha256_hash))
}

// ─── Azul Zulu provider ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ZuluPackage {
    #[serde(default)]
    distro_version: Vec<u32>,
    download_url: String,
    java_version: Vec<u32>,
    name: String,
    #[serde(default)]
    sha256_hash: Option<String>,
}

async fn resolve_zulu(major_version: u8, os: &str, arch: &str) -> Result<JrePackage, AquaError> {
    let url = format!(
        "https://api.azul.com/metadata/v1/zulu/packages/?java_version={}&os={}&arch={}&java_package_type=jre&javafx_bundled=false&release_status=ga&availability_types=CA&page_size=10",
        major_version, os, arch
    );

    log::info!("Querying Azul Zulu packages: {}", url);

    let packages: Vec<ZuluPackage> = HTTP_CLIENT
        .get(&url)
        .header("accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let pkg = packages
        .into_iter()
        .find(|p| {
            if cfg!(target_os = "windows") {
                p.name.ends_with(".zip")
            } else {
                p.name.ends_with(".tar.gz")
            }
        })
        .ok_or_else(|| {
            AquaError::Other(format!(
                "No Zulu JRE ({}) found for Java {}",
                if cfg!(target_os = "windows") {
                    "zip"
                } else {
                    "tar.gz"
                },
                major_version
            ))
        })?;

    let java_ver = pkg
        .java_version
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(".");

    let archive_format = if pkg.name.ends_with(".tar.gz") {
        ArchiveFormat::TarGz
    } else {
        ArchiveFormat::Zip
    };

    Ok(JrePackage {
        major_version,
        java_version: java_ver,
        download_url: pkg.download_url,
        filename: pkg.name,
        distro_version: pkg.distro_version,
        sha256_hash: pkg.sha256_hash,
        vendor: JreVendor::Zulu.name(),
        archive_format,
        size: None,
    })
}

// ─── Eclipse Adoptium provider ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AdoptiumRelease {
    binaries: Vec<AdoptiumBinary>,
    #[serde(rename = "version_data")]
    version: AdoptiumVersion,
}

#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    name: String,
    link: String,
    checksum: Option<String>,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct AdoptiumVersion {
    #[serde(rename = "openjdk_version")]
    openjdk_version: Option<String>,
    major: u32,
}

fn adoptium_os(os: &str) -> &str {
    match os {
        "macos" => "mac",
        other => other,
    }
}

async fn resolve_adoptium(
    major_version: u8,
    os: &str,
    arch: &str,
) -> Result<JrePackage, AquaError> {
    let url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?architecture={}&heap_size=normal&image_type=jre&jvm_impl=hotspot&os={}&page=0&page_size=1&project=jdk",
        major_version,
        arch,
        adoptium_os(os)
    );

    log::info!("Querying Eclipse Adoptium packages: {}", url);

    let mut releases: Vec<AdoptiumRelease> = HTTP_CLIENT
        .get(&url)
        .header("accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let release = releases.pop().ok_or_else(|| {
        AquaError::Other(format!("No Adoptium JRE found for Java {}", major_version))
    })?;

    let binary = release.binaries.into_iter().next().ok_or_else(|| {
        AquaError::Other(format!(
            "No Adoptium binary found for Java {}",
            major_version
        ))
    })?;

    let archive_format = if binary.package.name.ends_with(".tar.gz") {
        ArchiveFormat::TarGz
    } else if binary.package.name.ends_with(".zip") {
        ArchiveFormat::Zip
    } else {
        ArchiveFormat::TarGz
    };

    let java_version = release
        .version
        .openjdk_version
        .unwrap_or_else(|| format!("{}", release.version.major));

    Ok(JrePackage {
        major_version,
        java_version,
        download_url: binary.package.link,
        filename: binary.package.name,
        distro_version: vec![release.version.major],
        sha256_hash: binary.package.checksum,
        vendor: JreVendor::Temurin.name(),
        archive_format,
        size: Some(binary.package.size),
    })
}

#[cfg(test)]
#[path = "../tests/jre/providers.rs"]
mod tests;
