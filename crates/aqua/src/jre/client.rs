use crate::errors::ProtonError;
use crate::jre::types::{JrePackage, ZuluPackage};
use crate::utilities::HTTP_CLIENT;
use std::path::Path;

pub struct ZuluApi;

impl ZuluApi {
    pub async fn get_latest_package(java_version: u8) -> Result<JrePackage, ProtonError> {
        let os = current_os();
        let arch = current_arch();
        let is_windows = cfg!(target_os = "windows");

        let url = format!(
            "https://api.azul.com/metadata/v1/zulu/packages/?java_version={}&os={}&arch={}&java_package_type=jre&javafx_bundled=false&release_status=ga&availability_types=CA&page_size=10",
            java_version, os, arch
        );

        log::info!("Fetching Zulu package from: {}", url);

        let packages: Vec<ZuluPackage> = HTTP_CLIENT
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await?
            .json()
            .await?;

        let pkg = packages
            .into_iter()
            .find(|p| {
                if is_windows {
                    p.name.ends_with(".zip")
                } else {
                    p.name.ends_with(".tar.gz")
                }
            })
            .ok_or_else(|| ProtonError::Other(format!(
                "No Zulu JRE ({}) found for Java {}",
                if is_windows { "zip" } else { "tar.gz" },
                java_version
            )))?;

        let java_ver = pkg
            .java_version
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".");

        Ok(JrePackage {
            major_version: java_version,
            java_version: java_ver,
            download_url: pkg.download_url,
            filename: pkg.name,
            distro_version: pkg.distro_version,
        })
    }

    pub async fn download_and_extract(
        pkg: &JrePackage,
        dest_dir: &Path,
    ) -> Result<(), ProtonError> {
        let extract_dir = dest_dir.parent().unwrap_or(dest_dir);
        tokio::fs::create_dir_all(extract_dir).await?;

        let response = HTTP_CLIENT
            .get(&pkg.download_url)
            .send()
            .await?
            .error_for_status()?;

        let ext = if pkg.is_tar_gz() {
            "tar.gz"
        } else if pkg.is_zip() {
            "zip"
        } else {
            return Err(ProtonError::Other(format!(
                "Unknown archive format: {}",
                pkg.filename
            )));
        };

        let archive_path = extract_dir.join(format!("jre{}_tmp.{}", pkg.major_version, ext));

        {
            let mut file = tokio::fs::File::create(&archive_path).await?;
            let mut stream = response.bytes_stream();
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                use tokio::io::AsyncWriteExt;
                file.write_all(&chunk?).await?;
            }
        }

        if pkg.is_tar_gz() {
            extract_tar_gz(&archive_path, extract_dir, &pkg.filename).await?;
        } else {
            extract_zip(&archive_path, extract_dir, &pkg.filename).await?;
        }

        tokio::fs::remove_file(&archive_path).await?;

        let extracted_name = pkg.filename.trim_end_matches(".tar.gz").trim_end_matches(".zip");
        let extracted_dir = extract_dir.join(extracted_name);
        if extracted_dir.exists() {
            if dest_dir.exists() {
                tokio::fs::remove_dir_all(dest_dir).await?;
            }
            tokio::fs::rename(&extracted_dir, dest_dir).await?;
        }

        Ok(())
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

async fn extract_tar_gz(
    archive: &Path,
    dest: &Path,
    _filename: &str,
) -> Result<(), ProtonError> {
    let archive = archive.to_path_buf();
    let dest = dest.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive)?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&dest)?;
        Ok::<_, ProtonError>(())
    })
    .await?
}

async fn extract_zip(archive: &Path, dest: &Path, _filename: &str) -> Result<(), ProtonError> {
    let dest = dest.to_path_buf();
    let archive = archive.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| ProtonError::Other(format!("Failed to open ZIP: {}", e)))?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| ProtonError::Other(format!("Failed to read ZIP entry: {}", e)))?;

            let name = entry.name().to_string();
            if name.ends_with('/') {
                std::fs::create_dir_all(dest.join(&name))?;
                continue;
            }

            let out_path = dest.join(&name);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }

        Ok::<_, ProtonError>(())
    })
    .await?
}
