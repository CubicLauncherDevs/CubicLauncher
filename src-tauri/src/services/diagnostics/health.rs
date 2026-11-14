//! Comprobaciones locales del estado de una instancia. Ninguna escribe nada.

use super::{DiagnosticAction, DiagnosticCheck, DiagnosticReport, DiagnosticSeverity};
use crate::services::java_resolution;
use crate::services::{InstanceHandle, SettingsManager};
use std::path::{Path, PathBuf};
use tracing::warn;

/// Leído en el runtime async y pasado a [`run`].
pub struct InstanceContext {
    pub version: String,
    pub instance_dir: PathBuf,
    pub min_memory: u32,
    pub max_memory: u32,
    pub java_override: Option<u8>,
    pub shared_dir: PathBuf,
}

impl InstanceContext {
    pub async fn for_handle(handle: &InstanceHandle) -> Self {
        let instance_dir = handle.get_instance_dir().await;
        let version = handle.get_version().await.to_string();
        let overrides = handle.get_overrides().await;
        Self {
            version,
            instance_dir,
            min_memory: handle.get_min_memory().await,
            max_memory: handle.get_max_memory().await,
            java_override: overrides
                .as_ref()
                .and_then(|overrides| overrides.java_version),
            shared_dir: crate::core::PathManager::get().get_shared_dir(),
        }
    }
}

/// Debe llamarse dentro de `spawn_blocking`.
pub fn run(handle: &InstanceHandle, context: InstanceContext) -> DiagnosticReport {
    let mut checks = Vec::new();

    checks.extend(check_version_and_java(&context));
    check_memory(&context, &mut checks);

    match handle.try_lock_files() {
        Ok(_guard) => checks.extend(check_files(&context)),
        Err(message) => {
            if message.contains("actualización pendiente") {
                checks.push(
                    DiagnosticCheck::new("recovery_pending", DiagnosticSeverity::Error)
                        .detail(message),
                );
            } else {
                checks.push(
                    DiagnosticCheck::new("instance_busy", DiagnosticSeverity::Warning)
                        .detail(message),
                );
            }
        }
    }

    DiagnosticReport::from_checks(checks)
}

#[cfg(test)]
pub(super) fn check_memory_for_tests(context: &InstanceContext, checks: &mut Vec<DiagnosticCheck>) {
    check_memory(context, checks)
}

fn check_version_and_java(context: &InstanceContext) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    let missing = crate::services::version_installation::missing_dependencies(
        &context.shared_dir,
        &context.version,
    );
    if !missing.is_empty() {
        let mut check = DiagnosticCheck::new("version_missing", DiagnosticSeverity::Error)
            .param("version", context.version.clone())
            .action(DiagnosticAction::ReinstallVersion {
                version: context.version.clone(),
            });
        for dependency in missing.iter().take(10) {
            check = check.detail(dependency.clone());
        }
        checks.push(check);
    }

    let settings = SettingsManager::launch_snapshot();
    let java = java_resolution::resolve(
        &settings,
        &context.shared_dir,
        &context.version,
        context.java_override,
    );
    if !java.exists() {
        let version = java.major;
        checks.push(
            DiagnosticCheck::new("java_missing", DiagnosticSeverity::Error)
                .param("version", version.to_string())
                .param("requested", java.requested.to_string())
                .detail(java.path.to_string_lossy().into_owned())
                .action(DiagnosticAction::InstallJava { version }),
        );
    }

    checks
}

fn check_memory(context: &InstanceContext, checks: &mut Vec<DiagnosticCheck>) {
    let min = context.min_memory;
    let max = context.max_memory;
    if max < 1024 || min > max {
        checks.push(
            DiagnosticCheck::new("memory_low", DiagnosticSeverity::Warning)
                .param("min", min.to_string())
                .param("max", max.to_string()),
        );
    }
}

/// El llamador ya tiene el lock de la instancia.
pub(super) fn check_files(context: &InstanceContext) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    if let Some(check) = check_worlds(&context.instance_dir) {
        checks.push(check);
    }
    checks.extend(check_mods(&context.instance_dir));
    checks
}

fn check_worlds(instance_dir: &Path) -> Option<DiagnosticCheck> {
    let saves = crate::services::world_manager::saves_dir(instance_dir, false).ok()?;
    if !saves.exists() {
        return None;
    }
    let worlds = crate::services::world_manager::list_worlds(&saves)
        .inspect_err(|error| warn!("Diagnóstico: no se pudieron listar mundos: {error}"))
        .unwrap_or_default();
    let broken: Vec<_> = worlds
        .into_iter()
        .filter(|world| world.metadata_error)
        .collect();
    if broken.is_empty() {
        return None;
    }
    let mut check = DiagnosticCheck::new("worlds_metadata_error", DiagnosticSeverity::Warning)
        .param("count", broken.len().to_string())
        .action(DiagnosticAction::OpenWorlds);
    for world in broken.iter().take(10) {
        check = check.detail(world.folder.clone());
    }
    Some(check)
}

fn check_mods(instance_dir: &Path) -> Vec<DiagnosticCheck> {
    let mods: Vec<_> =
        crate::commands::instance::mod_catalog::list_cached(&instance_dir.join("mods"))
            .into_iter()
            .filter(|entry| entry.enabled)
            .collect();
    if mods.is_empty() {
        return Vec::new();
    }

    let mut checks = Vec::new();

    let mut by_project: std::collections::HashMap<&str, Vec<&str>> = Default::default();
    for entry in &mods {
        if let Some(project) = entry.project_id.as_deref() {
            by_project.entry(project).or_default().push(&entry.filename);
        }
    }
    let duplicates: Vec<_> = by_project
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(project, files)| (project.to_string(), files))
        .collect();
    if !duplicates.is_empty() {
        let mut check = DiagnosticCheck::new("mods_duplicate_project", DiagnosticSeverity::Warning)
            .param("count", duplicates.len().to_string())
            .action(DiagnosticAction::OpenMods);
        for (_, files) in duplicates.iter().take(5) {
            check = check.detail(files.join(", "));
        }
        checks.push(check);
    }

    let mut by_sha1: std::collections::HashMap<&str, Vec<&str>> = Default::default();
    for entry in &mods {
        if let Some(sha1) = entry.sha1.as_deref().filter(|sha1| !sha1.is_empty()) {
            by_sha1.entry(sha1).or_default().push(&entry.filename);
        }
    }
    let same_hash: Vec<_> = by_sha1
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(sha1, files)| (sha1.to_string(), files))
        .collect();
    if !same_hash.is_empty() {
        let mut check = DiagnosticCheck::new("mods_duplicate_sha1", DiagnosticSeverity::Warning)
            .param("count", same_hash.len().to_string())
            .action(DiagnosticAction::OpenMods);
        for (_, files) in same_hash.iter().take(5) {
            check = check.detail(files.join(", "));
        }
        checks.push(check);
    }

    checks
}
