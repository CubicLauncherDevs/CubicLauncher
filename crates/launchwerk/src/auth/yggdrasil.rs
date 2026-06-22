// Copyright (C) 2026 Santiagolxx, Notstaff and CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::MinecraftUser;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

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
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent("CubicLauncher/2.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Resolve API Location Indication (ALI) — converts a user-provided URL
    /// into the actual Yggdrasil API root.
    pub async fn resolve_api_url(&self, input: &str) -> Result<String, String> {
        let url = ensure_https(input);

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
                let resolved = resolve_relative_url(&url, ali_str);
                if resolved != url {
                    return Ok(resolved);
                }
            }
        }

        Ok(url)
    }

    /// Fetch API metadata from the Yggdrasil server root.
    pub async fn fetch_metadata(&self, api_root: &str) -> Result<YggdrasilServerInfo, String> {
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

fn ensure_https(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed)
    }
}

fn normalize_url(url: &str) -> String {
    let mut s = url.to_string();
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

fn resolve_relative_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    // Relative URL — resolve against base
    let base_clean = base.trim_end_matches('/');
    if relative.starts_with('/') {
        // Absolute path relative to origin
        if let Some(origin_end) = base_clean.find("://")
            && let Some(slash_pos) = base_clean[origin_end + 3..].find('/')
        {
            let origin = &base_clean[..origin_end + 3 + slash_pos];
            return format!("{}{}", origin, relative);
        }
        format!("{}{}", base_clean, relative)
    } else {
        format!("{}/{}", base_clean, relative)
    }
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
    "https://api.github.com/repos/CubicLauncher/authlib-injector/releases/latest";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    browser_download_url: String,
    digest: Option<String>,
}

/// Download authlib-injector.jar to the shared directory if not already cached.
/// Returns the path to the JAR file.
pub async fn download_authlib_injector(shared_dir: &Path) -> Result<PathBuf, String> {
    let cache_dir = shared_dir.join("authlib-injector");
    let jar_path = cache_dir.join("authlib-injector.jar");

    // Check if already cached
    if jar_path.exists() {
        log::info!(
            "authlib-injector.jar already cached at {}",
            jar_path.display()
        );
        return Ok(jar_path);
    }

    log::info!("Downloading authlib-injector from CubicLauncher fork...");
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("CubicLauncher/2.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Fetch latest release from GitHub
    let release: GitHubRelease = client
        .get(AUTHLIB_INJECTOR_RELEASES_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch latest release: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Invalid release response: {}", e))?;

    let asset = release
        .assets
        .into_iter()
        .find(|a| a.browser_download_url.ends_with("authlib-injector.jar"))
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
    std::fs::write(&jar_path, &bytes)
        .map_err(|e| format!("Failed to write authlib-injector.jar: {}", e))?;

    log::info!("authlib-injector.jar downloaded and cached");
    Ok(jar_path)
}

/// Fetch the Yggdrasil API metadata and return it as a Base64 string
/// suitable for the `-Dauthlibinjector.yggdrasil.prefetched` JVM argument.
pub async fn fetch_metadata_prefetch(api_root: &str) -> Result<String, String> {
    let url = normalize_url(api_root);
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("CubicLauncher/2.0")
        .build()
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
