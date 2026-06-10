// Copyright (C) 2026 Santiagolxx, Notstaff and CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::auth::MinecraftUser;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub const DEFAULT_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb"; // Minecraft Launcher Client ID
const SCOPE: &str = "XboxLive.SignIn XboxLive.offline_access";

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
struct MicrosoftTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Serialize)]
struct XblRequest {
    #[serde(rename = "Properties")]
    properties: XblProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Serialize)]
struct XblProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: String,
    #[serde(rename = "SiteName")]
    site_name: String,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Deserialize)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Serialize)]
struct XstsRequest {
    #[serde(rename = "Properties")]
    properties: XstsProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Serialize)]
struct XstsProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

#[derive(Serialize)]
struct MinecraftLoginRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
}

/// Gestor de autenticación con Microsoft y Xbox Live.
/// Permite configurar un `client_id` personalizado para el flujo de OAuth2.
///
/// Puedes usar `MicrosoftAuth::default()` para usar el Client ID oficial de Minecraft,
/// o instanciarlo con tu propio ID usando `MicrosoftAuth::new("tu-client-id".to_string())`.
#[derive(Debug, Clone)]
pub struct MicrosoftAuth {
    pub client_id: String,
}

impl Default for MicrosoftAuth {
    /// Crea una instancia utilizando el Client ID oficial del Minecraft Launcher.
    fn default() -> Self {
        Self {
            client_id: DEFAULT_CLIENT_ID.to_string(),
        }
    }
}

impl MicrosoftAuth {
    /// Crea una nueva instancia de autenticación con un Client ID personalizado.
    /// Útil si has registrado tu propia aplicación en el portal de Azure.
    pub fn new(client_id: String) -> Self {
        Self { client_id }
    }

    pub fn get_device_code(&self) -> crate::Result<DeviceCodeResponse> {
        let client = reqwest::blocking::Client::new();
        let params = [("client_id", self.client_id.as_str()), ("scope", SCOPE)];

        let res = client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
            .form(&params)
            .send()?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().unwrap_or_default();
            return Err(crate::Error::AuthError(format!(
                "Failed to get device code: {} - {}",
                status, err_body
            )));
        }

        let device_res = res.json::<DeviceCodeResponse>().map_err(|e| {
            crate::Error::AuthError(format!("Failed to decode device code response: {}", e))
        })?;

        Ok(device_res)
    }

    pub fn authenticate_with_device_code(
        &self,
        device_code: &str,
        interval: u64,
        expires_in: u64,
    ) -> crate::Result<MinecraftUser> {
        let client = reqwest::blocking::Client::new();
        let start = Instant::now();
        let interval = Duration::from_secs(interval);
        let expires_in = Duration::from_secs(expires_in);

        loop {
            if start.elapsed() >= expires_in {
                return Err(crate::Error::AuthError("Device code expired".into()));
            }

            let params = [
                ("client_id", self.client_id.as_str()),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ];

            let res = client
                .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
                .form(&params)
                .send()?;

            if res.status().is_success() {
                let token_res = res.json::<MicrosoftTokenResponse>()?;
                return Self::complete_login(&token_res.access_token, token_res.refresh_token);
            } else {
                // Check if we should continue polling
                let err_json: serde_json::Value = res.json()?;
                if err_json["error"] != "authorization_pending" {
                    return Err(crate::Error::AuthError(format!(
                        "Auth failed: {}",
                        err_json["error"]
                    )));
                }
            }

            std::thread::sleep(interval);
        }
    }

    pub fn refresh_token(&self, refresh_token: &str) -> crate::Result<MinecraftUser> {
        let client = reqwest::blocking::Client::new();
        let params = [
            ("client_id", self.client_id.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let res = client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .form(&params)
            .send()?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().unwrap_or_default();
            return Err(crate::Error::AuthError(format!(
                "Failed to refresh token: {} - {}",
                status, err_body
            )));
        }

        let token_res = res.json::<MicrosoftTokenResponse>().map_err(|e| {
            crate::Error::AuthError(format!("Failed to decode refresh token response: {}", e))
        })?;

        Self::complete_login(&token_res.access_token, token_res.refresh_token)
    }

    fn complete_login(
        ms_token: &str,
        refresh_token: Option<String>,
    ) -> crate::Result<MinecraftUser> {
        let client = reqwest::blocking::Client::new();

        // 1. XBL
        let xbl_req = XblRequest {
            properties: XblProperties {
                auth_method: "RPS".to_string(),
                site_name: "user.auth.xboxlive.com".to_string(),
                rps_ticket: format!("d={}", ms_token),
            },
            relying_party: "http://auth.xboxlive.com".to_string(),
            token_type: "JWT".to_string(),
        };

        let xbl_res = client
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .json(&xbl_req)
            .send()?
            .json::<XblResponse>()?;

        let user_hash = xbl_res.display_claims.xui[0].uhs.clone();

        // 2. XSTS
        let xsts_req = XstsRequest {
            properties: XstsProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbl_res.token],
            },
            relying_party: "rp://api.minecraftservices.com/".to_string(),
            token_type: "JWT".to_string(),
        };

        let xsts_res = client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .json(&xsts_req)
            .send()?
            .json::<XblResponse>()?; // XSTS response has same structure as XBL for what we need

        // 3. MC Login
        let mc_login_req = MinecraftLoginRequest {
            identity_token: format!("XBL3.0 x={};{}", user_hash, xsts_res.token),
        };

        let res = client
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .json(&mc_login_req)
            .send()?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().unwrap_or_default();
            return Err(crate::Error::AuthError(format!(
                "Minecraft login failed: {} - {}",
                status, err_body
            )));
        }

        let mc_res = res.json::<MinecraftLoginResponse>().map_err(|e| {
            crate::Error::AuthError(format!("Failed to decode Minecraft login response: {}", e))
        })?;

        // 4. MC Profile
        let res = client
            .get("https://api.minecraftservices.com/minecraft/profile")
            .bearer_auth(&mc_res.access_token)
            .send()?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().unwrap_or_default();
            return Err(crate::Error::AuthError(format!(
                "Failed to get Minecraft profile: {} - {}",
                status, err_body
            )));
        }

        let profile_res = res.json::<MinecraftProfileResponse>().map_err(|e| {
            crate::Error::AuthError(format!(
                "Failed to decode Minecraft profile response: {}",
                e
            ))
        })?;

        Ok(MinecraftUser::premium(
            profile_res.name,
            profile_res.id,
            mc_res.access_token,
            refresh_token,
        ))
    }
}
