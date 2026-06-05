use std::path::Path;
use std::sync::LazyLock;

#[cfg(feature = "extract-natives")]
use log::debug;
use log::{error, info, warn};
use reqwest::Client;
use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;

use crate::ProtonError;

pub static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .user_agent("Cubic Proton/2.0")
        .build()
        .expect("Failed to build reqwest client")
});

const MAX_DOWNLOAD_ATTEMPTS: usize = 3;

pub async fn download_file(url: &str, path: &Path, expected_hash: &str) -> Result<(), ProtonError> {
    if url.is_empty() {
        return Err(ProtonError::Other("Empty download URL".into()));
    }

    // Verify existing file
    if path.exists() {
        if !expected_hash.is_empty() {
            match verify_file_hash(path, expected_hash).await {
                Ok(true) => {
                    info!("File OK (hash match): {:?}", path);
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
            info!("File exists (no hash to verify): {:?}", path);
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
                    return Err(ProtonError::Other(format!("HTTP {}", r.status())));
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
                    return Err(ProtonError::RequestError(e));
                }
                tokio::time::sleep(std::time::Duration::from_millis(100 * (1 << (attempt - 1))))
                    .await;
                continue;
            }
        };

        let mut file = match tokio::fs::File::create(&temp_file).await {
            Ok(f) => f,
            Err(e) => return Err(ProtonError::IoError(e)),
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
                return Err(ProtonError::Other("Download stream failed".into()));
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
                    return Err(ProtonError::HashMismatch {
                        expected: expected_hash.to_string(),
                        actual: actual_hash,
                    });
                }
                continue;
            }
        }

        // Atomic rename
        tokio::fs::rename(&temp_file, path).await?;
        info!("Downloaded: {:?}", path);
        return Ok(());
    }

    Err(ProtonError::Other("Download failed after retries".into()))
}

pub async fn verify_file_hash(path: &Path, expected_hash: &str) -> Result<bool, ProtonError> {
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
pub(crate) fn extract_native_jar_sync(jar_path: &Path, destino: &Path) -> Result<(), ProtonError> {
    if !jar_path.exists() {
        return Err(ProtonError::Other(format!(
            "Native JAR not found: {:?}",
            jar_path
        )));
    }

    std::fs::create_dir_all(destino)?;

    let file = std::fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ProtonError::Other(format!("Failed to open ZIP: {}", e)))?;

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(e) => {
                warn!("Skipping entry {}: {}", i, e);
                continue;
            }
        };

        let name = entry.name().to_string();
        if !is_native_file(&name) {
            continue;
        }

        let file_name = Path::new(&name)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if file_name.is_empty() {
            continue;
        }

        let out_path = destino.join(&file_name);
        if out_path.exists() && out_path.metadata().map(|m| m.len()).unwrap_or(0) == entry.size() {
            debug!("Native already extracted: {}", out_path.display());
            continue;
        }

        let mut out_file = std::fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
        info!("Extracted native: {} -> {}", file_name, destino.display());
    }

    Ok(())
}

#[cfg(feature = "extract-natives")]
pub async fn extract_native(jar_path: &Path, destino: &Path) -> Result<(), ProtonError> {
    let jar_path = jar_path.to_path_buf();
    let destino = destino.to_path_buf();

    tokio::task::spawn_blocking(move || extract_native_jar_sync(&jar_path, &destino)).await?
}

#[cfg(feature = "extract-natives")]
pub(crate) fn is_native_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.starts_with("meta-inf") || lower.ends_with('/') {
        return false;
    }
    lower.ends_with(".so")
        || lower.ends_with(".dll")
        || lower.ends_with(".dylib")
        || lower.ends_with(".jnilib")
        || lower.contains(".so.")
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
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
