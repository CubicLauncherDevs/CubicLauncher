// Copyright (C) 2026 Santiagolxx, Notstaff and CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later
// src/auth/mod.rs
#![cfg(feature = "auth")]
pub mod microsoft;
pub mod storage;
#[cfg(test)]
#[path = "../tests/auth.rs"]
mod tests;
pub mod yggdrasil;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use storage::SecureStorage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftUser {
    pub username: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(skip)]
    pub access_token: String,
    #[serde(skip)]
    pub refresh_token: Option<String>,
    pub user_type: AccountType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yggdrasil_server_url: Option<String>,
    #[serde(skip)]
    pub client_token: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    Cracked,
    Microsoft,
    Yggdrasil,
}

impl MinecraftUser {
    /// Create a new cracked (offline) user
    pub fn cracked(username: impl Into<String>) -> Self {
        let username = username.into();
        Self {
            uuid: zellkern::offline_uuid(&username).to_string(),
            username,
            access_token: "0".to_string(),
            refresh_token: None,
            user_type: AccountType::Cracked,
            yggdrasil_server_url: None,
            client_token: None,
        }
    }

    /// Repair legacy/offline profiles without changing an existing identity.
    /// Returns whether the profile must be persisted again.
    pub fn ensure_offline_uuid(&mut self) -> bool {
        if self.user_type != AccountType::Cracked
            || uuid::Uuid::try_parse(&self.uuid).is_ok_and(|uuid| !uuid.is_nil())
        {
            return false;
        }
        self.uuid = zellkern::offline_uuid(&self.username).to_string();
        true
    }

    /// Create a new premium user (from Microsoft auth)
    pub fn premium(
        username: String,
        uuid: String,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Self {
        Self {
            username,
            uuid,
            access_token,
            refresh_token,
            user_type: AccountType::Microsoft,
            yggdrasil_server_url: None,
            client_token: None,
        }
    }

    /// Create a new Yggdrasil user
    pub fn yggdrasil(
        username: String,
        uuid: String,
        access_token: String,
        client_token: String,
        server_url: String,
    ) -> Self {
        Self {
            username,
            uuid,
            access_token,
            refresh_token: None,
            user_type: AccountType::Yggdrasil,
            yggdrasil_server_url: Some(server_url),
            client_token: Some(client_token),
        }
    }

    /// Expiración (epoch en segundos) del token de Minecraft, si es un JWT.
    ///
    /// Minecraft entrega un JWT con ~24 h de validez, así que leer su `exp`
    /// permite saltarse el refresco completo (cinco peticiones HTTP encadenadas)
    /// cuando el token guardado todavía sirve.
    pub fn access_token_expiry(&self) -> Option<i64> {
        let payload = self.access_token.split('.').nth(1)?;
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload)
            .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
            .ok()?;
        let claims: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
        claims.get("exp").and_then(serde_json::Value::as_i64)
    }

    /// Indica si el token de acceso guardado sigue siendo utilizable durante
    /// los próximos `margin_secs` segundos.
    ///
    /// Devuelve `false` cuando el token no es un JWT o no declara `exp`: en ese
    /// caso el llamador debe refrescar como siempre.
    pub fn access_token_is_fresh(&self, margin_secs: i64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs() as i64)
            .unwrap_or(0);
        self.access_token_expiry()
            .is_some_and(|exp| exp.saturating_sub(now) > margin_secs)
    }

    /// Save tokens to secure storage
    pub fn save_tokens(&self) -> crate::Result<()> {
        match self.user_type {
            AccountType::Microsoft => {
                SecureStorage::save(&self.uuid, "access", &self.access_token)?;
                if let Some(refresh) = &self.refresh_token {
                    SecureStorage::save(&self.uuid, "refresh", refresh)?;
                }
            }
            AccountType::Yggdrasil => {
                SecureStorage::save(&self.uuid, "access", &self.access_token)?;
                if let Some(client) = &self.client_token {
                    SecureStorage::save(&self.uuid, "client", client)?;
                }
            }
            AccountType::Cracked => {}
        }
        Ok(())
    }

    /// Load tokens from secure storage
    pub fn load_tokens(&mut self) -> crate::Result<()> {
        match self.user_type {
            AccountType::Microsoft => {
                self.access_token = SecureStorage::load(&self.uuid, "access")?;
                if let Ok(token) = SecureStorage::load(&self.uuid, "refresh") {
                    self.refresh_token = Some(token);
                }
            }
            AccountType::Yggdrasil => {
                self.access_token = SecureStorage::load(&self.uuid, "access")?;
                if let Ok(token) = SecureStorage::load(&self.uuid, "client") {
                    self.client_token = Some(token);
                }
            }
            AccountType::Cracked => {}
        }
        Ok(())
    }

    /// Delete tokens from secure storage
    pub fn delete_tokens(&self) -> crate::Result<()> {
        match self.user_type {
            AccountType::Microsoft => {
                SecureStorage::delete(&self.uuid, "access")?;
                SecureStorage::delete(&self.uuid, "refresh")?;
            }
            AccountType::Yggdrasil => {
                SecureStorage::delete(&self.uuid, "access")?;
                SecureStorage::delete(&self.uuid, "client")?;
            }
            AccountType::Cracked => {}
        }
        Ok(())
    }
}
