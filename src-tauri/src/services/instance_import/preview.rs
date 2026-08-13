//! Gestión de sesiones de preview para importación de instancias.
//!
//! Cada detección de ZIP crea una sesión temporal identificada por un token opaco.
//! El frontend debe devolver ese token para ejecutar la importación o cancelarla.
//! Si una sesión no se cierra explícitamente, su `Drop` intenta limpiar el directorio
//! temporal de forma best-effort.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tokio::fs;
use tracing::{info, warn};

/// Sesión de preview para una instancia pendiente de importar.
pub struct PreviewSession {
    token: String,
    preview_dir: PathBuf,
    cleaned: bool,
}

impl PreviewSession {
    fn new(preview_dir: PathBuf) -> Self {
        Self {
            token: uuid::Uuid::new_v4().to_string(),
            preview_dir,
            cleaned: false,
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn preview_dir(&self) -> &Path {
        &self.preview_dir
    }

    /// Borra el directorio temporal y marca la sesión como limpia.
    pub async fn cleanup(mut self) {
        if self.cleaned {
            return;
        }
        if self.preview_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&self.preview_dir).await {
                warn!(
                    "No se pudo borrar el directorio preview {:?}: {}",
                    self.preview_dir, e
                );
            } else {
                info!("Directorio preview {:?} borrado", self.preview_dir);
            }
        }
        self.cleaned = true;
    }
}

impl Drop for PreviewSession {
    fn drop(&mut self) {
        if self.cleaned {
            return;
        }
        let dir = self.preview_dir.clone();
        std::thread::spawn(move || {
            if let Err(e) = std::fs::remove_dir_all(&dir) {
                warn!(
                    "No se pudo borrar el directorio preview en Drop {:?}: {}",
                    dir, e
                );
            }
        });
    }
}

type SessionMap = Mutex<HashMap<String, PreviewSession>>;

fn sessions() -> &'static SessionMap {
    static SESSIONS: OnceLock<SessionMap> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Registra un directorio preview y devuelve un token opaco para recuperarlo.
pub fn register_preview(preview_dir: PathBuf) -> String {
    let session = PreviewSession::new(preview_dir);
    let token = session.token().to_string();
    sessions().lock().unwrap().insert(token.clone(), session);
    token
}

/// Recupera y remueve una sesión de preview por su token.
pub fn take_preview(token: &str) -> Option<PreviewSession> {
    sessions().lock().unwrap().remove(token)
}

/// Cancela una sesión de preview, limpiando sus recursos.
pub async fn cancel_preview(token: &str) -> bool {
    if let Some(session) = take_preview(token) {
        session.cleanup().await;
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_preview_session_lifecycle() {
        let dir = std::env::temp_dir().join(format!("cubic_preview_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("instance.cfg"), "name=Test\n").unwrap();

        let token = register_preview(dir.clone());
        assert!(dir.exists());

        let session = take_preview(&token).expect("token should exist");
        assert_eq!(session.preview_dir(), dir);

        session.cleanup().await;
        assert!(!dir.exists());
    }

    #[tokio::test]
    async fn test_cancel_preview() {
        let dir = std::env::temp_dir().join(format!("cubic_preview_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let token = register_preview(dir.clone());
        assert!(cancel_preview(&token).await);
        assert!(!dir.exists());
        assert!(!cancel_preview(&token).await);
    }
}
