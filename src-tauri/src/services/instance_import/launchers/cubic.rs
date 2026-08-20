//! Provider para importar instancias exportadas por CubicLauncher.
//!
//! Detecta el archivo `cubic-manifest.json` generado por el exportador propio y
//! reconstruye la instancia conservando loader, versión, memoria, Java e icono.

use super::super::{ImportError, InstanceImportPlan, InstanceImporter};
use super::multimc::migrate::{migrate_game_data, resolve_game_dir};
use crate::core::{AppEvent, emit};
use crate::services::instance_import::sanitize_instance_name;
use crate::services::instance_manager::data::RamOverrides;
use crate::services::{DownloadQueue, InstOverrides, InstanceDto, InstanceManager};
use serde::Deserialize;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use tracing::{info, warn};

/// Manifiesto escrito por `services::instance_export`.
#[derive(Debug, Clone, Deserialize)]
pub struct CubicManifest {
    pub format_version: u8,
    #[allow(dead_code)]
    pub exported_by: String,
    #[allow(dead_code)]
    pub uuid: String,
    pub name: String,
    pub version_id: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub min_memory: u32,
    pub max_memory: u32,
    pub overrides: Option<InstOverrides>,
}

/// Provider para ZIPs exportados por CubicLauncher.
pub struct CubicProvider;

impl InstanceImporter for CubicProvider {
    fn id(&self) -> &'static str {
        "cubic"
    }

    fn display_name(&self) -> &'static str {
        "CubicLauncher"
    }

    fn detect(&self, preview_dir: &Path) -> bool {
        preview_dir.join("cubic-manifest.json").is_file()
    }

    fn preview(&self, preview_dir: &Path) -> Result<InstanceImportPlan, ImportError> {
        let manifest = read_manifest(preview_dir)?;
        let sanitized = sanitize_instance_name(&manifest.name);

        Ok(InstanceImportPlan {
            format_id: self.id(),
            format_name: self.display_name(),
            original_name: manifest.name,
            sanitized_name: sanitized,
            minecraft_version: Some(manifest.mc_version),
            loader: Some(manifest.loader),
            loader_version: manifest.loader_version,
            warnings: Vec::new(),
            preview_token: String::new(),
        })
    }

    fn import<'a>(
        &'a self,
        preview_dir: &'a Path,
        target_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<InstanceDto, ImportError>> + Send + 'a>> {
        Box::pin(import_cubic_instance(preview_dir, target_name))
    }
}

fn read_manifest(preview_dir: &Path) -> Result<CubicManifest, ImportError> {
    let path = preview_dir.join("cubic-manifest.json");
    let content = std::fs::read_to_string(&path).map_err(|e| {
        ImportError::InvalidArchive(format!("No se pudo leer cubic-manifest.json: {e}"))
    })?;

    let manifest: CubicManifest = serde_json::from_str(&content)
        .map_err(|e| ImportError::InvalidArchive(format!("cubic-manifest.json inválido: {e}")))?;

    if manifest.format_version != 1 {
        warn!(
            "cubic-manifest.json con format_version {} (esperado 1)",
            manifest.format_version
        );
    }

    Ok(manifest)
}

async fn import_cubic_instance(
    preview_dir: &Path,
    target_name: &str,
) -> Result<InstanceDto, ImportError> {
    let manifest = read_manifest(preview_dir)?;
    let final_name = if target_name.trim().is_empty() {
        sanitize_instance_name(&manifest.name)
    } else {
        target_name.to_string()
    };

    info!(
        "Importando instancia CubicLauncher desde {:?} como '{}'",
        preview_dir, final_name
    );

    let handle = InstanceManager::get()
        .create_instance(final_name, manifest.version_id.clone(), None)
        .await
        .map_err(|e| match e {
            crate::core::errors::instance::InstanceError::AlreadyExists => {
                ImportError::ProviderError {
                    provider: "CubicLauncher".into(),
                    message: "Ya existe una instancia con ese nombre".into(),
                }
            }
            other => ImportError::Instance(other),
        })?;

    let instance_dir = handle.get_instance_dir().await;
    let source_game_dir = resolve_game_dir(preview_dir);

    if source_game_dir.exists() {
        migrate_game_data(&source_game_dir, &instance_dir).await?;
    }

    let icon_src = preview_dir.join("icon.png");
    if icon_src.is_file() {
        let dest_icon = instance_dir.join("icon.png");
        if let Err(e) = tokio::fs::copy(&icon_src, &dest_icon).await {
            warn!("No se pudo copiar el icono {:?}: {}", icon_src, e);
        } else {
            handle
                .set_icon(Some(dest_icon.to_string_lossy().to_string()))
                .await;
        }
    }

    let java_version = manifest.overrides.as_ref().and_then(|o| o.java_version);
    let memory = Some(RamOverrides {
        min_mem: manifest.min_memory,
        max_mem: manifest.max_memory,
    });
    handle
        .set_overrides(Some(InstOverrides {
            java_version,
            memory,
        }))
        .await;

    handle
        .save_if_dirty()
        .await
        .map_err(|e| ImportError::ProviderError {
            provider: "CubicLauncher".into(),
            message: format!("Error guardando la instancia importada: {e}"),
        })?;

    DownloadQueue::get()
        .enqueue(manifest.version_id.clone())
        .await;

    emit(AppEvent::InstanceCreated {
        id: handle.uuid.to_string().into(),
        dto: handle.to_dto().await,
    });

    info!(
        "Instancia CubicLauncher importada exitosamente: uuid={}",
        handle.uuid
    );
    Ok(handle.to_dto().await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::instance_import::sanitize_instance_name;

    #[test]
    fn test_detect_cubic_format() {
        let temp =
            std::env::temp_dir().join(format!("cubic_import_detect_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("cubic-manifest.json"), r#"{"format_version":1,"exported_by":"CubicLauncher","uuid":"u","name":"Test","version_id":"1.21","mc_version":"1.21","loader":"Vanilla","loader_version":null,"min_memory":512,"max_memory":2048,"overrides":null}"#).unwrap();

        let provider = CubicProvider;
        assert!(provider.detect(&temp));

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_preview_cubic_instance() {
        let temp = std::env::temp_dir().join(format!(
            "cubic_import_preview_test_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("cubic-manifest.json"), r#"{"format_version":1,"exported_by":"CubicLauncher","uuid":"u","name":"Mi Instancia","version_id":"fabric-loader-0.15.0-1.21","mc_version":"1.21","loader":"Fabric","loader_version":"0.15.0","min_memory":1024,"max_memory":4096,"overrides":null}"#).unwrap();

        let provider = CubicProvider;
        let plan = provider.preview(&temp).unwrap();

        assert_eq!(plan.format_id, "cubic");
        assert_eq!(plan.minecraft_version.as_deref(), Some("1.21"));
        assert_eq!(plan.loader.as_deref(), Some("Fabric"));
        assert_eq!(plan.loader_version.as_deref(), Some("0.15.0"));
        assert_eq!(plan.sanitized_name, sanitize_instance_name("Mi Instancia"));

        let _ = std::fs::remove_dir_all(&temp);
    }
}
