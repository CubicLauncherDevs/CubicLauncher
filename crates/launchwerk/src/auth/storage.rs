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
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zellkern::path_security::ConfinedDir;
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
///
/// Todas las operaciones de disco pasan por [`ConfinedDir`], que resuelve cada
/// componente relativo a un descriptor abierto y rechaza enlaces simbólicos,
/// junctions y puntos de reanálisis de Windows. Los ficheros temporales se
/// crean dentro del mismo directorio confinado y se publican con `rename`
/// relativo al descriptor, de modo que un ataque TOCTOU no puede redirigir la
/// escritura fuera del almacén.
///
/// Limitaciones conocidas:
/// - En Windows los permisos son ACL heredadas del directorio; este crate no
///   crea una DACL explícita. En Unix se aplican `0600`/`0700`.
/// - Los ancestros configurados (por ejemplo las raíces XDG) son de confianza:
///   un enlace simbólico allí puede reubicar todo el almacén, igual que en
///   cualquier ruta de datos de la aplicación.
/// - Los hardlinks dentro del almacén no se rechazan. Las escrituras publican
///   siempre un inodo nuevo, así que un hardlink nunca se sobrescribe a través
///   del almacén; un hardlink malicioso solo podría hacer que la lectura
///   devuelva datos que no descifran.
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
        let secrets = self.open_secrets(true)?;
        let (key_dir, key_file) = self.open_key_dir(true)?;

        let master_key = Zeroizing::new(self.load_or_create_master_key(&key_dir, &key_file)?);
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

        secrets.write_private(Path::new(&Self::secret_file_name(uuid, key_name)), &data)?;
        Ok(())
    }

    fn load_secret(&self, uuid: &str, key_name: &str) -> crate::Result<String> {
        Self::validate(uuid, key_name)?;
        let secrets = match self.open_secrets_io(false) {
            Ok(dir) => dir,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(Self::not_found(uuid, key_name));
            }
            Err(e) => return Err(e.into()),
        };
        let file_name = Self::secret_file_name(uuid, key_name);
        let data = match secrets.read(Path::new(&file_name)) {
            Ok(data) => data,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(Self::not_found(uuid, key_name));
            }
            Err(e) => return Err(e.into()),
        };

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
        let secrets = match self.open_secrets_io(false) {
            Ok(dir) => dir,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        match secrets.remove_file(Path::new(&Self::secret_file_name(uuid, key_name))) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    fn not_found(uuid: &str, key_name: &str) -> crate::Error {
        crate::Error::AuthError(format!("Secret not found for {}:{}", uuid, key_name))
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

        let (key_dir, key_file) = match self.open_key_dir(false) {
            Ok(location) => location,
            Err(crate::Error::Io(e)) if e.kind() == io::ErrorKind::NotFound => {
                return Err(crate::Error::AuthError(
                    "Vault key not found; the saved tokens cannot be decrypted".into(),
                ));
            }
            Err(e) => return Err(e),
        };
        let master_key = Zeroizing::new(self.load_master_key(&key_dir, &key_file)?);
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

    fn load_or_create_master_key(
        &self,
        key_dir: &ConfinedDir,
        key_file: &Path,
    ) -> crate::Result<[u8; MASTER_KEY_LEN]> {
        let _guard = KEY_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(key) = self.try_load_master_key(key_dir, key_file)? {
            return Ok(key);
        }

        let mut key = [0u8; MASTER_KEY_LEN];
        rng().fill_bytes(&mut key);
        let result = self.create_master_key(key_dir, key_file, &key);
        key.zeroize();
        result?;

        // Puede que otra instancia ya hubiera creado la clave; en ese caso se
        // devuelve la que está realmente en disco.
        self.load_master_key(key_dir, key_file)
    }

    fn load_master_key(
        &self,
        key_dir: &ConfinedDir,
        key_file: &Path,
    ) -> crate::Result<[u8; MASTER_KEY_LEN]> {
        self.try_load_master_key(key_dir, key_file)?.ok_or_else(|| {
            crate::Error::AuthError(
                "Vault key not found; the saved tokens cannot be decrypted".into(),
            )
        })
    }

    fn try_load_master_key(
        &self,
        key_dir: &ConfinedDir,
        key_file: &Path,
    ) -> crate::Result<Option<[u8; MASTER_KEY_LEN]>> {
        let data = match key_dir.read(key_file) {
            Ok(data) => Zeroizing::new(data),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

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

    fn create_master_key(
        &self,
        key_dir: &ConfinedDir,
        key_file: &Path,
        key: &[u8; MASTER_KEY_LEN],
    ) -> crate::Result<()> {
        let mut data = Zeroizing::new(Vec::with_capacity(KEY_FILE_LEN));
        data.extend_from_slice(KEY_MAGIC);
        data.push(KEY_VERSION);
        data.extend_from_slice(key);

        match key_dir.create_new_private(key_file, &data) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                // Otra instancia creó la clave entre la lectura y la creación.
                // El llamador volverá a leer la clave existente.
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    fn open_secrets_io(&self, create: bool) -> io::Result<ConfinedDir> {
        // Use the application directory as the capability root and open the
        // final "secrets" component relative to it. This rejects a link at
        // either level instead of trusting the "claunch-rs" ancestor.
        let parent = self.secrets_dir.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Could not determine secure storage parent",
            )
        })?;
        let name = self.secrets_dir.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Could not determine secure storage directory name",
            )
        })?;
        let root = if create {
            ConfinedDir::open_or_create(parent)?
        } else {
            ConfinedDir::open_existing(parent)?
        };
        if create {
            root.create_dir_all(Path::new(name))?;
        }
        let dir = root.open_dir(Path::new(name))?;
        dir.restrict();
        Ok(dir)
    }

    fn open_secrets(&self, create: bool) -> crate::Result<ConfinedDir> {
        self.open_secrets_io(create).map_err(|e| {
            crate::Error::AuthError(format!("Secure storage directory is not safe: {e}"))
        })
    }

    fn open_key_dir(&self, create: bool) -> crate::Result<(ConfinedDir, PathBuf)> {
        let parent = self.key_path.parent().ok_or_else(|| {
            crate::Error::AuthError("Could not determine vault key directory".into())
        })?;
        let name = self
            .key_path
            .file_name()
            .ok_or_else(|| crate::Error::AuthError("Could not determine vault key file".into()))?;
        let dir = if create {
            ConfinedDir::open_or_create(parent)
        } else {
            ConfinedDir::open_existing(parent)
        }?;
        Ok((dir, PathBuf::from(name)))
    }

    fn secret_file_name(uuid: &str, key_name: &str) -> String {
        format!("{}_{}.enc", uuid, key_name)
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

    #[cfg(test)]
    pub(crate) fn save_legacy(&self, uuid: &str, key_name: &str, value: &str) -> crate::Result<()> {
        Self::validate(uuid, key_name)?;
        let secrets = self.open_secrets(true)?;

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

        secrets.write_private(Path::new(&Self::secret_file_name(uuid, key_name)), &data)?;
        Ok(())
    }
}
