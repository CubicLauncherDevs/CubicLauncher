// Copyright (C) 2026 Santiagolxx, Notstaff and CubicLauncher contributors
// SPDX-License-Identifier: GPL-3.0-only
#![cfg(feature = "auth")]
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use log::warn;
use rand::{RngCore, rng};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeroize::{Zeroize, Zeroizing};

const SECRET_MAGIC: &[u8; 8] = b"CLSECRET";
const SECRET_VERSION: u8 = 2;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const SECRET_HEADER_LEN: usize = SECRET_MAGIC.len() + 1 + SALT_LEN + NONCE_LEN;

const KEY_MAGIC: &[u8; 7] = b"CLVAULT";
const KEY_VERSION: u8 = 1;
const MASTER_KEY_LEN: usize = 32;
const KEY_FILE_LEN: usize = KEY_MAGIC.len() + 1 + MASTER_KEY_LEN;

const LEGACY_SALT: &[u8] = b"claunch-rs-secure-salt-v1";

static KEY_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
std::thread_local! {
    static TEST_ROOT: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// Almacenamiento local de tokens cifrado con una clave maestra aleatoria.
///
/// La clave maestra se guarda aparte de los secretos y nunca se deriva del
/// UUID ni de información pública de la cuenta.
pub struct SecureStorage {
    secrets_dir: PathBuf,
    key_path: PathBuf,
}

impl SecureStorage {
    pub fn save(uuid: &str, key_name: &str, value: &str) -> crate::Result<()> {
        Self::from_default_paths()?.save_secret(uuid, key_name, value)
    }

    pub fn load(uuid: &str, key_name: &str) -> crate::Result<String> {
        Self::from_default_paths()?.load_secret(uuid, key_name)
    }

    pub fn delete(uuid: &str, key_name: &str) -> crate::Result<()> {
        Self::from_default_paths()?.delete_secret(uuid, key_name)
    }

    fn from_default_paths() -> crate::Result<Self> {
        #[cfg(test)]
        if let Some(root) = TEST_ROOT.with(|slot| slot.borrow().clone()) {
            return Ok(Self::with_root(&root));
        }

        let data_root = dirs::data_dir().or_else(dirs::home_dir).ok_or_else(|| {
            crate::Error::AuthError("Could not determine storage directory".into())
        })?;
        // La clave se guarda fuera de la carpeta de tokens cuando el sistema
        // separa configuración y datos (XDG en Linux).
        let config_root = dirs::config_dir()
            .or_else(dirs::data_dir)
            .or_else(dirs::home_dir)
            .ok_or_else(|| {
                crate::Error::AuthError("Could not determine config directory".into())
            })?;

        Ok(Self {
            secrets_dir: data_root.join("claunch-rs").join("secrets"),
            key_path: config_root.join("claunch-rs").join("vault.key"),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_root(root: &Path) -> Self {
        Self {
            secrets_dir: root.join("secrets"),
            key_path: root.join("vault.key"),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_test_root<T>(root: &Path, f: impl FnOnce() -> T) -> T {
        struct Guard(Option<PathBuf>);
        impl Drop for Guard {
            fn drop(&mut self) {
                TEST_ROOT.with(|slot| *slot.borrow_mut() = self.0.take());
            }
        }

        let previous = TEST_ROOT.with(|slot| slot.borrow_mut().replace(root.to_path_buf()));
        let _guard = Guard(previous);
        f()
    }

    fn save_secret(&self, uuid: &str, key_name: &str, value: &str) -> crate::Result<()> {
        Self::validate(uuid, key_name)?;
        self.ensure_secrets_dir()?;

        let master_key = Zeroizing::new(self.load_or_create_master_key()?);
        let mut salt = [0u8; SALT_LEN];
        rng().fill_bytes(&mut salt);
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rng().fill_bytes(&mut nonce_bytes);

        let key = Zeroizing::new(Self::derive_record_key(&master_key, &salt, uuid, key_name)?);
        let cipher = Aes256Gcm::new_from_slice(&*key)
            .map_err(|e| crate::Error::AuthError(format!("Cipher error: {}", e)))?;
        let aad = Self::build_aad(uuid, key_name);
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: value.as_bytes(),
                    aad: &aad,
                },
            )
            .map_err(|e| crate::Error::AuthError(format!("Encryption error: {}", e)))?;

        let mut data = Vec::with_capacity(SECRET_HEADER_LEN + ciphertext.len());
        data.extend_from_slice(SECRET_MAGIC);
        data.push(SECRET_VERSION);
        data.extend_from_slice(&salt);
        data.extend_from_slice(&nonce_bytes);
        data.extend_from_slice(&ciphertext);

        Self::write_atomic(&self.secret_path(uuid, key_name), &data)
    }

    fn load_secret(&self, uuid: &str, key_name: &str) -> crate::Result<String> {
        Self::validate(uuid, key_name)?;
        let file_path = self.secret_path(uuid, key_name);
        let data = match fs::read(&file_path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(crate::Error::AuthError(format!(
                    "Secret not found for {}:{}",
                    uuid, key_name
                )));
            }
            Err(e) => return Err(e.into()),
        };
        Self::harden_file(&file_path);

        if data.starts_with(SECRET_MAGIC) {
            return self.decrypt_v2(&data, uuid, key_name);
        }

        let plaintext = Self::decrypt_legacy(&data, uuid)?;

        // Migración transparente: si el token antiguo se pudo leer, se
        // reescribe con el formato nuevo. Un fallo de escritura no impide
        // usar el token en esta sesión.
        if let Err(e) = self.save_secret(uuid, key_name, &plaintext) {
            warn!("No se pudo migrar el token a formato v2: {}", e);
        }

        Ok(plaintext)
    }

    fn delete_secret(&self, uuid: &str, key_name: &str) -> crate::Result<()> {
        Self::validate(uuid, key_name)?;
        match fs::remove_file(self.secret_path(uuid, key_name)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    fn decrypt_v2(&self, data: &[u8], uuid: &str, key_name: &str) -> crate::Result<String> {
        if data.len() < SECRET_HEADER_LEN + TAG_LEN {
            return Err(crate::Error::AuthError("Invalid secret file format".into()));
        }
        let version = data[SECRET_MAGIC.len()];
        if version != SECRET_VERSION {
            return Err(crate::Error::AuthError(format!(
                "Unsupported secret version: {}",
                version
            )));
        }

        let salt_start = SECRET_MAGIC.len() + 1;
        let salt_end = salt_start + SALT_LEN;
        let nonce_end = salt_end + NONCE_LEN;
        let salt = &data[salt_start..salt_end];
        let nonce = Nonce::from_slice(&data[salt_end..nonce_end]);
        let ciphertext = &data[nonce_end..];

        let master_key = Zeroizing::new(self.load_master_key()?);
        let key = Zeroizing::new(Self::derive_record_key(&master_key, salt, uuid, key_name)?);
        let cipher = Aes256Gcm::new_from_slice(&*key)
            .map_err(|e| crate::Error::AuthError(format!("Cipher error: {}", e)))?;
        let aad = Self::build_aad(uuid, key_name);
        let plaintext = cipher
            .decrypt(
                nonce,
                Payload {
                    msg: ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|e| crate::Error::AuthError(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| crate::Error::AuthError(format!("UTF-8 error: {}", e)))
    }

    fn decrypt_legacy(data: &[u8], uuid: &str) -> crate::Result<String> {
        if data.len() < NONCE_LEN + TAG_LEN {
            return Err(crate::Error::AuthError("Invalid secret file format".into()));
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let key = Zeroizing::new(Self::legacy_key(uuid));
        let cipher = Aes256Gcm::new_from_slice(&*key)
            .map_err(|e| crate::Error::AuthError(format!("Cipher error: {}", e)))?;
        let result = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|e| crate::Error::AuthError(format!("Decryption failed: {}", e)));

        String::from_utf8(result?)
            .map_err(|e| crate::Error::AuthError(format!("UTF-8 error: {}", e)))
    }

    fn derive_record_key(
        master_key: &[u8; MASTER_KEY_LEN],
        salt: &[u8],
        uuid: &str,
        key_name: &str,
    ) -> crate::Result<[u8; MASTER_KEY_LEN]> {
        let mut info = Vec::with_capacity(22 + 1 + uuid.len() + 1 + key_name.len());
        info.extend_from_slice(b"cubiclaunch-token-v2");
        info.push(0);
        info.extend_from_slice(uuid.as_bytes());
        info.push(0);
        info.extend_from_slice(key_name.as_bytes());

        let hkdf = Hkdf::<Sha256>::new(Some(salt), master_key);
        let mut key = [0u8; MASTER_KEY_LEN];
        hkdf.expand(&info, &mut key)
            .map_err(|e| crate::Error::AuthError(format!("Key derivation failed: {}", e)))?;
        Ok(key)
    }

    fn legacy_key(uuid: &str) -> [u8; MASTER_KEY_LEN] {
        let mut hasher = Sha256::new();
        hasher.update(uuid.as_bytes());
        hasher.update(LEGACY_SALT);
        let result = hasher.finalize();
        let mut key = [0u8; MASTER_KEY_LEN];
        key.copy_from_slice(&result);
        key
    }

    fn build_aad(uuid: &str, key_name: &str) -> Vec<u8> {
        let mut aad = Vec::with_capacity(SECRET_MAGIC.len() + 1 + uuid.len() + 1 + key_name.len());
        aad.extend_from_slice(SECRET_MAGIC);
        aad.push(SECRET_VERSION);
        aad.extend_from_slice(uuid.as_bytes());
        aad.push(0);
        aad.extend_from_slice(key_name.as_bytes());
        aad
    }

    fn load_or_create_master_key(&self) -> crate::Result<[u8; MASTER_KEY_LEN]> {
        let _guard = KEY_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(key) = self.try_load_master_key()? {
            return Ok(key);
        }

        let mut key = [0u8; MASTER_KEY_LEN];
        rng().fill_bytes(&mut key);
        let result = self.create_master_key(&key);
        key.zeroize();
        result?;

        // Puede que otra instancia ya hubiera creado la clave; en ese caso se
        // devuelve la que está realmente en disco.
        self.load_master_key()
    }

    fn load_master_key(&self) -> crate::Result<[u8; MASTER_KEY_LEN]> {
        self.try_load_master_key()?.ok_or_else(|| {
            crate::Error::AuthError(
                "Vault key not found; the saved tokens cannot be decrypted".into(),
            )
        })
    }

    fn try_load_master_key(&self) -> crate::Result<Option<[u8; MASTER_KEY_LEN]>> {
        let data = match fs::read(&self.key_path) {
            Ok(data) => Zeroizing::new(data),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        Self::harden_file(&self.key_path);

        if data.len() != KEY_FILE_LEN
            || !data.starts_with(KEY_MAGIC)
            || data[KEY_MAGIC.len()] != KEY_VERSION
        {
            return Err(crate::Error::AuthError(
                "Vault key file is invalid or corrupt".into(),
            ));
        }

        let mut key = [0u8; MASTER_KEY_LEN];
        key.copy_from_slice(&data[KEY_MAGIC.len() + 1..]);
        Ok(Some(key))
    }

    fn create_master_key(&self, key: &[u8; MASTER_KEY_LEN]) -> crate::Result<()> {
        self.ensure_key_dir()?;

        let mut data = Zeroizing::new(Vec::with_capacity(KEY_FILE_LEN));
        data.extend_from_slice(KEY_MAGIC);
        data.push(KEY_VERSION);
        data.extend_from_slice(key);

        #[allow(unused_mut)]
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let mut file = match options.open(&self.key_path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Otra instancia creó la clave entre la lectura y la creación.
                // El llamador volverá a leer la clave existente.
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        file.write_all(&data)?;
        file.sync_all()?;
        Self::harden_file(&self.key_path);
        Ok(())
    }

    fn secret_path(&self, uuid: &str, key_name: &str) -> PathBuf {
        self.secrets_dir.join(format!("{}_{}.enc", uuid, key_name))
    }

    fn validate(uuid: &str, key_name: &str) -> crate::Result<()> {
        if !matches!(key_name, "access" | "refresh" | "client") {
            return Err(crate::Error::AuthError(format!(
                "Invalid secret name: {}",
                key_name
            )));
        }
        let has_unsafe_chars = uuid.is_empty()
            || uuid == "."
            || uuid == ".."
            || uuid
                .chars()
                .any(|c| matches!(c, '/' | '\\' | ':' | '\0') || c.is_control());
        if has_unsafe_chars || Path::new(uuid).components().count() != 1 {
            return Err(crate::Error::AuthError(
                "Invalid account identifier for secret storage".into(),
            ));
        }
        Ok(())
    }

    fn ensure_secrets_dir(&self) -> crate::Result<()> {
        if !self.secrets_dir.exists() {
            fs::create_dir_all(&self.secrets_dir)?;
        }
        Self::harden_dir(&self.secrets_dir);
        Ok(())
    }

    fn ensure_key_dir(&self) -> crate::Result<()> {
        let dir = self.key_path.parent().ok_or_else(|| {
            crate::Error::AuthError("Could not determine vault key directory".into())
        })?;
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
        Self::harden_dir(dir);
        Ok(())
    }

    fn write_atomic(path: &Path, data: &[u8]) -> crate::Result<()> {
        let dir = path.parent().ok_or_else(|| {
            crate::Error::AuthError("Could not determine secret directory".into())
        })?;
        let tmp = dir.join(format!(".{}.tmp", uuid::Uuid::new_v4()));

        #[allow(unused_mut)]
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let result = (|| -> std::io::Result<()> {
            let mut file = options.open(&tmp)?;
            file.write_all(data)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&tmp, path)?;
            Ok(())
        })();

        if let Err(e) = result {
            let _ = fs::remove_file(&tmp);
            return Err(e.into());
        }

        Self::harden_file(path);
        Ok(())
    }

    #[cfg(unix)]
    fn harden_dir(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700);
            let _ = fs::set_permissions(path, permissions);
        }
    }

    #[cfg(not(unix))]
    fn harden_dir(_path: &Path) {}

    #[cfg(unix)]
    fn harden_file(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            let _ = fs::set_permissions(path, permissions);
        }
    }

    #[cfg(not(unix))]
    fn harden_file(_path: &Path) {}

    #[cfg(test)]
    pub(crate) fn save_legacy(&self, uuid: &str, key_name: &str, value: &str) -> crate::Result<()> {
        Self::validate(uuid, key_name)?;
        self.ensure_secrets_dir()?;

        let key = Zeroizing::new(Self::legacy_key(uuid));
        let cipher = Aes256Gcm::new_from_slice(&*key)
            .map_err(|e| crate::Error::AuthError(format!("Cipher error: {}", e)))?;
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rng().fill_bytes(&mut nonce_bytes);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), value.as_bytes())
            .map_err(|e| crate::Error::AuthError(format!("Encryption error: {}", e)))?;

        let mut data = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        data.extend_from_slice(&nonce_bytes);
        data.extend_from_slice(&ciphertext);

        Self::write_atomic(&self.secret_path(uuid, key_name), &data)
    }
}
