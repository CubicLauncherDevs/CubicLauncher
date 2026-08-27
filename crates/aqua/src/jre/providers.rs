use crate::errors::AquaError;
use crate::jre::types::{ArchiveFormat, JrePackage};
use crate::utilities::HTTP_CLIENT;
use serde::Deserialize;

/// Supported JRE vendors. The chain tries each vendor in order until one
/// returns a valid package, providing resilience if a provider is down or
/// does not ship the requested major version for the current platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JreVendor {
    Zulu,
    Adoptium,
}

impl JreVendor {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Zulu => "Azul Zulu",
            Self::Adoptium => "Eclipse Adoptium",
        }
    }

    pub async fn resolve(self, major_version: u8) -> Result<JrePackage, AquaError> {
        let os = current_os();
        let arch = current_arch();
        match self {
            Self::Zulu => resolve_zulu(major_version, os, arch).await,
            Self::Adoptium => resolve_adoptium(major_version, os, arch).await,
        }
    }
}

/// Default provider chain used by the launcher. Tries the fastest/most
/// reliable provider first and falls back to the next one on failure.
pub struct JreProviderChain;

impl JreProviderChain {
    const VENDORS: &[JreVendor] = &[JreVendor::Zulu, JreVendor::Adoptium];

    pub async fn get_latest_package(major_version: u8) -> Result<JrePackage, AquaError> {
        let mut last_err = None;
        for vendor in Self::VENDORS {
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
        vendor: JreVendor::Adoptium.name(),
        archive_format,
        size: Some(binary.package.size),
    })
}
