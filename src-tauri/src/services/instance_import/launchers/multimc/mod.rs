//! Provider para instancias exportadas por MultiMC, Prism Launcher y forks compatibles.

use super::super::InstanceImporter;
use super::super::{ImportError, InstanceImportPlan};
use crate::core::{AppEvent, emit};
use crate::services::instance_manager::data::RamOverrides;
use crate::services::{DownloadQueue, InstOverrides, InstanceDto, InstanceManager};
use migrate::{migrate_game_data, resolve_game_dir, resolve_icon_path};
use parser::parse_multimc_instance;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use tracing::{info, warn};

mod migrate;
mod parser;

/// Provider para ZIPs de MultiMC / Prism Launcher.
pub struct MultimcProvider;

impl InstanceImporter for MultimcProvider {
    fn id(&self) -> &'static str {
        "multimc"
    }

    fn display_name(&self) -> &'static str {
        "MultiMC / Prism"
    }

    fn detect(&self, preview_dir: &Path) -> bool {
        preview_dir.join("instance.cfg").is_file()
    }

    fn preview(&self, preview_dir: &Path) -> Result<InstanceImportPlan, ImportError> {
        let meta = parse_multimc_instance(preview_dir)?;

        Ok(InstanceImportPlan {
            format_id: self.id(),
            format_name: self.display_name(),
            original_name: meta.original_name.clone(),
            sanitized_name: meta.sanitized_name.clone(),
            minecraft_version: meta.game_version.as_ref().map(|gv| gv.mc_version.clone()),
            loader: meta
                .game_version
                .as_ref()
                .map(|gv| gv.loader.name().to_string()),
            loader_version: meta
                .game_version
                .as_ref()
                .and_then(|gv| gv.loader.version().map(|s| s.to_string())),
            warnings: build_warnings(&meta),
            preview_token: String::new(),
        })
    }

    fn import<'a>(
        &'a self,
        preview_dir: &'a Path,
        target_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<InstanceDto, ImportError>> + Send + 'a>> {
        Box::pin(import_multimc_instance(preview_dir, target_name))
    }
}

fn build_warnings(meta: &parser::MultiMcInstanceMeta) -> Vec<String> {
    let mut warnings = Vec::new();
    if !meta.unsupported_loaders.is_empty() {
        warnings.push(format!(
            "Loaders no soportados detectados: {}. Se importará como Vanilla + archivos.",
            meta.unsupported_loaders.join(", ")
        ));
    }
    warnings
}

async fn import_multimc_instance(
    preview_dir: &Path,
    target_name: &str,
) -> Result<InstanceDto, ImportError> {
    let meta = parse_multimc_instance(preview_dir)?;
    let final_name = if target_name.trim().is_empty() {
        meta.sanitized_name.clone()
    } else {
        target_name.to_string()
    };

    info!(
        "Importando instancia MultiMC/Prism desde {:?} como '{}'",
        preview_dir, final_name
    );

    let version_id = meta
        .version_id()
        .ok_or(ImportError::UnknownMinecraftVersion)?;

    let handle = InstanceManager::get()
        .create_instance(final_name, version_id.clone(), None)
        .await
        .map_err(|e| match e {
            crate::core::errors::instance::InstanceError::AlreadyExists => {
                ImportError::ProviderError {
                    provider: "MultiMC".into(),
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

    if let Some(icon_path) = resolve_icon_path(preview_dir, &meta.icon_key) {
        let dest_icon = instance_dir.join("icon.png");
        if let Err(e) = tokio::fs::copy(&icon_path, &dest_icon).await {
            warn!("No se pudo copiar el icono {:?}: {}", icon_path, e);
        } else {
            handle
                .set_icon(Some(dest_icon.to_string_lossy().to_string()))
                .await;
        }
    }

    if meta.min_memory.is_some() || meta.max_memory.is_some() {
        let overrides = InstOverrides {
            java_version: None,
            memory: Some(RamOverrides {
                min_mem: meta.min_memory.unwrap_or(512),
                max_mem: meta.max_memory.unwrap_or(2048),
            }),
        };
        handle.set_overrides(Some(overrides)).await;
    }

    handle
        .save_if_dirty()
        .await
        .map_err(|e| ImportError::ProviderError {
            provider: "MultiMC".into(),
            message: format!("Error guardando la instancia importada: {e}"),
        })?;

    DownloadQueue::get().enqueue(version_id.clone()).await;

    emit(AppEvent::InstanceCreated {
        id: handle.uuid.to_string().into(),
        dto: handle.to_dto().await,
    });

    info!("Instancia importada exitosamente: uuid={}", handle.uuid);
    Ok(handle.to_dto().await)
}
