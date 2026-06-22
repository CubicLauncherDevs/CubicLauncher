use std::path::Path;
use std::sync::LazyLock;

use log::{debug, error, warn};
use reqwest::Client;
use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;

#[cfg(test)]
use zellkern::is_native_file;

use crate::AquaError;

pub static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .user_agent("Cubic Proton/2.0")
        .build()
        .expect("Failed to build reqwest client")
});

const MAX_DOWNLOAD_ATTEMPTS: usize = 3;

pub async fn download_file(url: &str, path: &Path, expected_hash: &str) -> Result<(), AquaError> {
    if url.is_empty() {
        return Err(AquaError::Other("Empty download URL".into()));
    }

    // Verify existing file
    if path.exists() {
        if !expected_hash.is_empty() {
            match verify_file_hash(path, expected_hash).await {
                Ok(true) => {
                    debug!("File OK (hash match): {:?}", path);
                    return Ok(());
                }
                Ok(false) => {
                    warn!("Hash mismatch, re-downloading: {:?}", path);
                    let _ = tokio::fs::remove_file(path).await;
                }
                Err(e) => {
                    warn!("Verify failed, re-downloading: {}", e);
                    let _ = tokio::fs::remove_file(path).await;
                }
            }
        } else {
            debug!("File exists (no hash to verify): {:?}", path);
            return Ok(());
        }
    }

    let temp_file = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));

    for attempt in 1..=MAX_DOWNLOAD_ATTEMPTS {
        // Ensure parent dir exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Clean temp from previous attempts
        let _ = tokio::fs::remove_file(&temp_file).await;

        let response = match HTTP_CLIENT.get(url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                warn!(
                    "HTTP {} on attempt {}/{}",
                    r.status(),
                    attempt,
                    MAX_DOWNLOAD_ATTEMPTS
                );
                if attempt == MAX_DOWNLOAD_ATTEMPTS {
                    return Err(AquaError::Other(format!("HTTP {}", r.status())));
                }
                tokio::time::sleep(std::time::Duration::from_millis(100 * (1 << (attempt - 1))))
                    .await;
                continue;
            }
            Err(e) => {
                warn!(
                    "Request failed attempt {}/{}: {}",
                    attempt, MAX_DOWNLOAD_ATTEMPTS, e
                );
                if attempt == MAX_DOWNLOAD_ATTEMPTS {
                    return Err(AquaError::RequestError(e));
                }
                tokio::time::sleep(std::time::Duration::from_millis(100 * (1 << (attempt - 1))))
                    .await;
                continue;
            }
        };

        let mut file = match tokio::fs::File::create(&temp_file).await {
            Ok(f) => f,
            Err(e) => return Err(AquaError::IoError(e)),
        };

        let mut hasher = Sha1::new();
        let mut stream = response.bytes_stream();

        use futures::StreamExt;
        let mut write_ok = true;
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    hasher.update(&chunk);
                    if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await {
                        error!("Write error: {}", e);
                        write_ok = false;
                        break;
                    }
                }
                Err(e) => {
                    warn!("Stream error on attempt {}: {}", attempt, e);
                    write_ok = false;
                    break;
                }
            }
        }

        if !write_ok {
            let _ = tokio::fs::remove_file(&temp_file).await;
            if attempt == MAX_DOWNLOAD_ATTEMPTS {
                return Err(AquaError::Other("Download stream failed".into()));
            }
            continue;
        }

        file.flush().await?;

        // Verify hash if provided
        if !expected_hash.is_empty() {
            let actual_hash = hex_encode(hasher.finalize().as_slice());
            if actual_hash != expected_hash {
                warn!(
                    "Hash mismatch attempt {}/{}: expected={}, got={}",
                    attempt, MAX_DOWNLOAD_ATTEMPTS, expected_hash, actual_hash
                );
                let _ = tokio::fs::remove_file(&temp_file).await;
                if attempt == MAX_DOWNLOAD_ATTEMPTS {
                    return Err(AquaError::HashMismatch {
                        expected: expected_hash.to_string(),
                        actual: actual_hash,
                    });
                }
                continue;
            }
        }

        // Atomic rename
        tokio::fs::rename(&temp_file, path).await?;
        debug!("Downloaded: {:?}", path);
        return Ok(());
    }

    Err(AquaError::Other("Download failed after retries".into()))
}

pub async fn verify_file_hash(path: &Path, expected_hash: &str) -> Result<bool, AquaError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 8192];

    loop {
        let n = tokio::io::AsyncReadExt::read(&mut file, &mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let actual_hash = hex_encode(hasher.finalize().as_slice());
    Ok(actual_hash == expected_hash)
}

#[cfg(feature = "extract-natives")]
pub(crate) fn extract_native_jar_sync(jar_path: &Path, destino: &Path) -> Result<(), AquaError> {
    if !jar_path.exists() {
        return Err(AquaError::Other(format!(
            "Native JAR not found: {:?}",
            jar_path
        )));
    }
    std::fs::create_dir_all(destino)?;
    zellkern::extract_jar(jar_path, destino)
        .map_err(|e| AquaError::Other(format!("Failed to extract native JAR: {e}")))
}

#[cfg(feature = "extract-natives")]
pub async fn extract_native(jar_path: &Path, destino: &Path) -> Result<(), AquaError> {
    let jar_path = jar_path.to_path_buf();
    let destino = destino.to_path_buf();

    tokio::task::spawn_blocking(move || extract_native_jar_sync(&jar_path, &destino)).await?
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

// ─── Forge utilities ──────────────────────────────────────────────────────────

/// Extract a ZIP file to a destination directory (all files, no filtering).
pub fn extract_zip_to_dir(zip_path: &Path, dest_dir: &Path) -> Result<(), AquaError> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| AquaError::ForgeExtract(format!("Cannot open ZIP: {e}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AquaError::ForgeExtract(format!("Invalid ZIP: {e}")))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AquaError::ForgeExtract(format!("Cannot read ZIP entry: {e}")))?;

        let name = entry.name().to_string();
        if name.ends_with('/') {
            continue;
        }

        let out_path = dest_dir.join(&name);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AquaError::ForgeExtract(format!("Cannot create dir: {e}")))?;
        }

        let mut out_file = std::fs::File::create(&out_path)
            .map_err(|e| AquaError::ForgeExtract(format!("Cannot create file: {e}")))?;
        std::io::copy(&mut entry, &mut out_file)
            .map_err(|e| AquaError::ForgeExtract(format!("Cannot extract file: {e}")))?;
    }

    Ok(())
}

/// Read the Main-Class attribute from a JAR file's META-INF/MANIFEST.MF.
pub fn read_jar_main_class(jar_path: &Path) -> Result<String, AquaError> {
    let file = std::fs::File::open(jar_path).map_err(|e| AquaError::ForgeProcessor {
        processor: jar_path.display().to_string(),
        detail: format!("Cannot open JAR: {e}"),
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| AquaError::ForgeProcessor {
        processor: jar_path.display().to_string(),
        detail: format!("Invalid JAR: {e}"),
    })?;

    let manifest =
        archive
            .by_name("META-INF/MANIFEST.MF")
            .map_err(|e| AquaError::ForgeProcessor {
                processor: jar_path.display().to_string(),
                detail: format!("No MANIFEST.MF: {e}"),
            })?;

    let mut content = String::new();
    std::io::Read::read_to_string(&mut std::io::BufReader::new(manifest), &mut content).map_err(
        |e| AquaError::ForgeProcessor {
            processor: jar_path.display().to_string(),
            detail: format!("Cannot read MANIFEST.MF: {e}"),
        },
    )?;

    for line in content.lines() {
        if let Some(mc) = line.strip_prefix("Main-Class:") {
            return Ok(mc.trim().to_string());
        }
    }

    Err(AquaError::ForgeProcessor {
        processor: jar_path.display().to_string(),
        detail: "No Main-Class in MANIFEST.MF".into(),
    })
}

/// Run a Java process synchronously (blocking). Used for Forge post-processors.
pub async fn run_java_process(
    java_path: &Path,
    classpath: &str,
    main_class: &str,
    args: Vec<String>,
    processor_name: &str,
) -> Result<(), AquaError> {
    let java_path = java_path.to_path_buf();
    let classpath = classpath.to_string();
    let main_class = main_class.to_string();
    let processor_name = processor_name.to_string();

    let status = tokio::process::Command::new(&java_path)
        .arg("-cp")
        .arg(&classpath)
        .arg(&main_class)
        .args(&args)
        .status()
        .await
        .map_err(|e| AquaError::ForgeProcessor {
            processor: processor_name.clone(),
            detail: format!("Failed to start Java: {e}"),
        })?;

    if !status.success() {
        return Err(AquaError::ForgeProcessor {
            processor: processor_name,
            detail: format!("Process exited with code {}", status.code().unwrap_or(-1)),
        });
    }

    Ok(())
}

/// Compute SHA1 hash of a file (synchronous, for post-processor output verification).
pub fn compute_sha1_sync(path: &Path) -> Result<String, AquaError> {
    use sha1::{Digest, Sha1};

    let mut file = std::fs::File::open(path).map_err(|e| AquaError::ForgeOutputVerification {
        file: path.display().to_string(),
        expected: "N/A".into(),
        actual: format!("Cannot open file: {e}"),
    })?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf).map_err(|e| {
            AquaError::ForgeOutputVerification {
                file: path.display().to_string(),
                expected: "N/A".into(),
                actual: format!("Read error: {e}"),
            }
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_encode(hasher.finalize().as_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[0xab, 0xcd]), "abcd");
        assert_eq!(hex_encode(&[0x00, 0xff]), "00ff");
    }

    #[cfg(feature = "extract-natives")]
    #[test]
    fn test_is_native_file() {
        assert!(is_native_file("liblwjgl.so"));
        assert!(is_native_file("opengl32.dll"));
        assert!(is_native_file("libglfw.dylib"));
        assert!(!is_native_file("META-INF/MANIFEST.MF"));
        assert!(!is_native_file("some/path/"));
    }
}
