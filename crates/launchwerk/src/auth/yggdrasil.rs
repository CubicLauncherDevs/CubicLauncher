// Copyright (C) 2026 Santiagolxx, Notstaff and CubicLauncher contributors
// SPDX-License-Identifier: GPL-3.0-only
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::{Host, Url};

use super::MinecraftUser;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: usize = 10;

// ── Public types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YggdrasilServerInfo {
    pub server_name: String,
    pub skin_domains: Vec<String>,
    pub signature_publickey: Option<String>,
    pub non_email_login: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YggdrasilProfile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YggdrasilUser {
    pub id: String,
    #[serde(default)]
    pub properties: Vec<YggdrasilProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YggdrasilProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
struct YggdrasilAuthResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "clientToken")]
    client_token: String,
    #[serde(default, rename = "availableProfiles")]
    available_profiles: Vec<YggdrasilProfile>,
    #[serde(rename = "selectedProfile")]
    selected_profile: Option<YggdrasilProfile>,
    #[serde(default)]
    #[allow(dead_code)]
    user: Option<YggdrasilUser>,
}

#[derive(Debug, Clone, Deserialize)]
struct YggdrasilMetadataResponse {
    #[serde(default)]
    skin_domains: Vec<String>,
    #[serde(default, rename = "signaturePublickey")]
    signature_publickey: Option<String>,
    #[serde(default)]
    meta: Option<YggdrasilMeta>,
}

#[derive(Debug, Clone, Deserialize)]
struct YggdrasilMeta {
    #[serde(default, rename = "serverName")]
    server_name: Option<String>,
    #[serde(default)]
    feature: Option<YggdrasilFeatures>,
}

#[derive(Debug, Clone, Deserialize)]
struct YggdrasilFeatures {
    #[serde(default, rename = "non_email_login")]
    non_email_login: bool,
}

// ── YggdrasilAuth ───────────────────────────────────────────────────────────

pub struct YggdrasilAuth {
    client: Client,
}

impl Default for YggdrasilAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl YggdrasilAuth {
    pub fn new() -> Self {
        let client = yggdrasil_client(DEFAULT_TIMEOUT).expect("Failed to create HTTP client");

        Self { client }
    }

    /// Resolve API Location Indication (ALI) — converts a user-provided URL
    /// into the actual Yggdrasil API root.
    pub async fn resolve_api_url(&self, input: &str) -> Result<String, String> {
        let url = ensure_https(input)?;

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to connect to {}: {}", url, e))?;

        // Follow redirects and check for ALI header
        let headers = response.headers().clone();
        if let Some(ali) = headers.get("x-authlib-injector-api-location") {
            let ali_str = ali.to_str().unwrap_or("");
            if !ali_str.is_empty() {
                let resolved = resolve_relative_url(&url, ali_str)?;
                if resolved != url {
                    return Ok(resolved);
                }
            }
        }

        Ok(url)
    }

    /// Fetch API metadata from the Yggdrasil server root.
    pub async fn fetch_metadata(&self, api_root: &str) -> Result<YggdrasilServerInfo, String> {
        validate_api_root(api_root)?;
        let url = normalize_url(api_root);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch metadata from {}: {}", url, e))?;

        let metadata: YggdrasilMetadataResponse = response
            .json()
            .await
            .map_err(|e| format!("Invalid metadata response: {}", e))?;

        let server_name = metadata
            .meta
            .as_ref()
            .and_then(|m| m.server_name.clone())
            .unwrap_or_else(|| "Unknown Server".to_string());

        let non_email_login = metadata
            .meta
            .as_ref()
            .and_then(|m| m.feature.as_ref())
            .map(|f| f.non_email_login)
            .unwrap_or(false);

        Ok(YggdrasilServerInfo {
            server_name,
            skin_domains: metadata.skin_domains,
            signature_publickey: metadata.signature_publickey,
            non_email_login,
        })
    }

    /// Authenticate with username and password.
    pub async fn authenticate(
        &self,
        api_root: &str,
        username: &str,
        password: &str,
    ) -> Result<MinecraftUser, String> {
        validate_api_root(api_root)?;
        let url = format!("{}authserver/authenticate", normalize_url(api_root));

        let body = serde_json::json!({
            "agent": {
                "name": "Minecraft",
                "version": 1
            },
            "username": username,
            "password": password,
            "requestUser": true
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Authentication request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let error_msg = parse_yggdrasil_error(&error_text)
                .unwrap_or_else(|| format!("HTTP {}: {}", status, error_text));
            return Err(error_msg);
        }

        let auth_response: YggdrasilAuthResponse = response
            .json()
            .await
            .map_err(|e| format!("Invalid auth response: {}", e))?;

        build_user_from_response(auth_response, api_root)
    }

    /// Refresh an existing access token.
    pub async fn refresh(
        &self,
        api_root: &str,
        access_token: &str,
        client_token: &str,
        profile_id: &str,
        profile_name: &str,
    ) -> Result<MinecraftUser, String> {
        validate_api_root(api_root)?;
        let url = format!("{}authserver/refresh", normalize_url(api_root));

        let body = serde_json::json!({
            "accessToken": access_token,
            "clientToken": client_token,
            "selectedProfile": {
                "id": profile_id,
                "name": profile_name
            },
            "requestUser": true
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Refresh request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let error_msg = parse_yggdrasil_error(&error_text)
                .unwrap_or_else(|| format!("HTTP {}: {}", status, error_text));
            return Err(error_msg);
        }

        let auth_response: YggdrasilAuthResponse = response
            .json()
            .await
            .map_err(|e| format!("Invalid refresh response: {}", e))?;

        build_user_from_response(auth_response, api_root)
    }

    /// Validate an access token.
    pub async fn validate(&self, api_root: &str, access_token: &str, client_token: &str) -> bool {
        if let Err(e) = validate_api_root(api_root) {
            log::warn!("Rejected Yggdrasil API root: {}", e);
            return false;
        }
        let url = format!("{}authserver/validate", normalize_url(api_root));

        let body = serde_json::json!({
            "accessToken": access_token,
            "clientToken": client_token
        });

        self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn yggdrasil_client(timeout: Duration) -> reqwest::Result<Client> {
    Client::builder()
        .timeout(timeout)
        .user_agent("CubicLauncher/2.0")
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > MAX_REDIRECTS {
                return attempt.error("too many redirects");
            }
            if attempt.url().scheme() == "http"
                && attempt
                    .previous()
                    .last()
                    .is_some_and(|previous| previous.scheme() == "https")
            {
                return attempt.error("refusing HTTPS to HTTP redirect");
            }
            if validate_url_policy(attempt.url()).is_err() {
                return attempt.error("refusing insecure redirect");
            }
            attempt.follow()
        }))
        .build()
}

fn ensure_https(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Yggdrasil server URL cannot be empty".to_string());
    }

    let parsed = Url::parse(trimmed).ok().filter(Url::has_host);
    let url = match parsed {
        Some(url) => url,
        None => Url::parse(&format!("https://{}", trimmed))
            .map_err(|e| format!("Invalid Yggdrasil server URL '{}': {}", trimmed, e))?,
    };

    validate_url_policy(&url)?;
    Ok(url.to_string())
}

fn validate_api_root(api_root: &str) -> Result<(), String> {
    let url = Url::parse(api_root)
        .map_err(|e| format!("Invalid Yggdrasil API root '{}': {}", api_root, e))?;
    validate_url_policy(&url)
}

fn validate_url_policy(url: &Url) -> Result<(), String> {
    match url.scheme() {
        "https" => Ok(()),
        "http" if is_allowed_insecure_host(url) => Ok(()),
        "http" => Err(format!(
            "Yggdrasil server '{}' uses plain HTTP; use HTTPS or a local/private address",
            url
        )),
        scheme => Err(format!(
            "Unsupported Yggdrasil server URL scheme '{}'",
            scheme
        )),
    }
}

fn is_allowed_insecure_host(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            domain == "localhost" || domain.ends_with(".localhost")
        }
        Some(Host::Ipv4(ip)) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        Some(Host::Ipv6(ip)) => {
            ip.is_loopback()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip
                    .to_ipv4_mapped()
                    .is_some_and(|ip| ip.is_loopback() || ip.is_private() || ip.is_link_local())
        }
        _ => false,
    }
}

fn normalize_url(url: &str) -> String {
    let mut s = url.to_string();
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

fn resolve_relative_url(base: &str, relative: &str) -> Result<String, String> {
    let base =
        Url::parse(base).map_err(|e| format!("Invalid Yggdrasil API root '{}': {}", base, e))?;
    let resolved = base
        .join(relative)
        .map_err(|e| format!("Invalid Yggdrasil API location '{}': {}", relative, e))?;

    if base.scheme() == "https" && resolved.scheme() == "http" {
        return Err("Refusing to downgrade Yggdrasil connection from HTTPS to HTTP".to_string());
    }

    validate_url_policy(&resolved)?;
    Ok(resolved.to_string())
}

fn parse_yggdrasil_error(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct YggError {
        error: Option<String>,
        #[serde(rename = "errorMessage")]
        error_message: Option<String>,
    }

    let err: YggError = serde_json::from_str(body).ok()?;
    match err.error_message {
        Some(msg) => Some(msg),
        None => err.error,
    }
}

fn build_user_from_response(
    resp: YggdrasilAuthResponse,
    api_root: &str,
) -> Result<MinecraftUser, String> {
    let profile = resp
        .selected_profile
        .as_ref()
        .or(resp.available_profiles.first())
        .ok_or_else(|| "No profiles available on this account".to_string())?;

    Ok(MinecraftUser::yggdrasil(
        profile.name.clone(),
        profile.id.clone(),
        resp.access_token,
        resp.client_token,
        api_root.to_string(),
    ))
}

// ── Authlib Injector Download ───────────────────────────────────────────────

const AUTHLIB_INJECTOR_RELEASES_URL: &str =
    "https://api.github.com/repos/yushijinhun/authlib-injector/releases/latest";
const AUTHLIB_UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    browser_download_url: String,
    digest: Option<String>,
}

/// Descarga o actualiza authlib-injector.jar y devuelve su ruta.
pub async fn download_authlib_injector(shared_dir: &Path) -> Result<PathBuf, String> {
    let cache_dir = shared_dir.join("authlib-injector");
    let jar_path = cache_dir.join("authlib-injector.jar");
    let version_path = cache_dir.join("authlib-injector.version");
    let checked_path = cache_dir.join("authlib-injector.checked");

    let cached_version = std::fs::read_to_string(&version_path).ok();
    let last_checked = std::fs::read_to_string(&checked_path)
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    let recently_checked = last_checked.is_some_and(|timestamp| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .is_ok_and(|now| {
                now.as_secs().saturating_sub(timestamp) < AUTHLIB_UPDATE_INTERVAL.as_secs()
            })
    });

    if jar_path.exists() && cached_version.is_some() && recently_checked {
        return Ok(jar_path);
    }

    match fetch_latest_authlib_injector(shared_dir, cached_version.as_deref()).await {
        Ok(version) => {
            let _ = std::fs::write(&version_path, &version);
            if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
                let _ = std::fs::write(&checked_path, now.as_secs().to_string());
            }
            log::info!("Versión de authlib-injector lista: {version}");
            Ok(jar_path)
        }
        Err(error) if jar_path.exists() => {
            log::warn!(
                "No se pudo actualizar authlib-injector; se usará la copia instalada: {error}"
            );
            Ok(jar_path)
        }
        Err(error) => Err(error),
    }
}

async fn fetch_latest_authlib_injector(
    shared_dir: &Path,
    cached_version: Option<&str>,
) -> Result<String, String> {
    let cache_dir = shared_dir.join("authlib-injector");
    let jar_path = cache_dir.join("authlib-injector.jar");

    log::info!("Downloading the latest authlib-injector release...");
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("CubicLauncher/2.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Fetch latest release from GitHub
    let response = client
        .get(AUTHLIB_INJECTOR_RELEASES_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch latest release: {}", e))?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch latest authlib-injector release: HTTP {}",
            response.status()
        ));
    }
    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Invalid release response: {}", e))?;

    let version = release.tag_name;
    if jar_path.exists() && cached_version.is_some_and(|cached| cached.trim() == version) {
        return Ok(version);
    }

    let asset = release
        .assets
        .into_iter()
        .find(|asset| {
            asset
                .browser_download_url
                .rsplit('/')
                .next()
                .is_some_and(|name| {
                    name.starts_with("authlib-injector-")
                        && name.ends_with(".jar")
                        && !name.ends_with("-sources.jar")
                })
        })
        .ok_or_else(|| "No authlib-injector.jar found in release".to_string())?;

    // Parse SHA-256 from digest field ("sha256:HASH" format)
    let expected_sha256 = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .map(String::from);

    log::info!(
        "Downloading authlib-injector from {}",
        asset.browser_download_url
    );

    // Download the JAR
    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download authlib-injector: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download authlib-injector: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read authlib-injector response: {}", e))?;

    // Verify checksum
    let hash = sha2::Sha256::digest(&bytes);
    let hash_hex = format!("{:x}", hash);
    if let Some(ref expected) = expected_sha256 {
        if hash_hex != *expected {
            return Err(format!(
                "authlib-injector checksum mismatch: expected {}, got {}",
                expected, hash_hex
            ));
        }
        log::info!("authlib-injector SHA-256 verified: {}", hash_hex);
    } else {
        log::warn!("No SHA-256 digest in release asset, skipping checksum verification");
    }

    // Save to disk
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;
    let temp_path = cache_dir.join(format!("authlib-injector-{}.jar.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&temp_path, &bytes)
        .map_err(|e| format!("Failed to write temporary authlib-injector.jar: {}", e))?;
    if let Err(error) = std::fs::rename(&temp_path, &jar_path) {
        if !jar_path.exists() {
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!("Failed to install authlib-injector.jar: {}", error));
        }

        // Windows no permite reemplazar un archivo existente con rename.
        let backup_path = cache_dir.join(format!("authlib-injector-{}.bak", uuid::Uuid::new_v4()));
        std::fs::rename(&jar_path, &backup_path)
            .map_err(|e| format!("Failed to preserve cached authlib-injector.jar: {}", e))?;
        if let Err(error) = std::fs::rename(&temp_path, &jar_path) {
            let _ = std::fs::rename(&backup_path, &jar_path);
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!("Failed to replace authlib-injector.jar: {}", error));
        }
        let _ = std::fs::remove_file(backup_path);
    }

    log::info!("authlib-injector.jar downloaded and cached");
    Ok(version)
}

/// Fetch the Yggdrasil API metadata and return it as a Base64 string
/// suitable for the `-Dauthlibinjector.yggdrasil.prefetched` JVM argument.
pub async fn fetch_metadata_prefetch(api_root: &str) -> Result<String, String> {
    validate_api_root(api_root)?;
    let url = normalize_url(api_root);
    let client = yggdrasil_client(Duration::from_secs(15))
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch metadata from {}: {}", url, e))?;

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read metadata response: {}", e))?;

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(body.as_bytes());
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_hosts_default_to_https() {
        assert_eq!(ensure_https("example.com").unwrap(), "https://example.com/");
        assert_eq!(
            ensure_https("HTTPS://example.com").unwrap(),
            "https://example.com/"
        );
    }

    #[test]
    fn http_is_allowed_for_loopback_and_private_hosts() {
        for input in [
            "http://localhost",
            "http://localhost:8080",
            "http://auth.localhost",
            "http://127.0.0.1",
            "http://10.0.0.5",
            "http://172.16.4.2",
            "http://192.168.1.10",
            "http://169.254.10.20",
            "http://[::1]",
            "http://[fd00::1]",
            "http://[fe80::1]",
            "http://[::ffff:192.168.1.1]",
        ] {
            assert!(ensure_https(input).is_ok(), "{input} should be allowed");
        }
    }

    #[test]
    fn http_is_rejected_for_public_hosts() {
        for input in [
            "http://example.com",
            "http://8.8.8.8",
            "http://[2001:db8::1]",
            "ftp://example.com",
            "",
        ] {
            assert!(ensure_https(input).is_err(), "{input} should be rejected");
        }
    }

    #[test]
    fn ali_resolution_rejects_public_http_and_downgrades() {
        assert!(resolve_relative_url("https://example.com/api/", "v2").is_ok());
        assert!(resolve_relative_url("https://example.com/api/", "https://other.example/").is_ok());
        assert!(resolve_relative_url("https://example.com/api/", "http://example.com/").is_err());
        assert!(resolve_relative_url("https://example.com/api/", "http://127.0.0.1:8080").is_err());
        assert!(resolve_relative_url("http://127.0.0.1:8080/", "http://127.0.0.1:9090/").is_ok());
    }

    #[test]
    fn api_root_validation_rejects_legacy_insecure_urls() {
        assert!(validate_api_root("https://example.com").is_ok());
        assert!(validate_api_root("http://192.168.1.2").is_ok());
        assert!(validate_api_root("http://example.com").is_err());
        assert!(validate_api_root("not a url").is_err());
    }
}
