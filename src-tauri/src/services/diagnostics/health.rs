//! Comprobaciones locales del estado de una instancia. Ninguna escribe nada.

use super::rules::CrashSeverity;
use super::{DiagnosticAction, DiagnosticCheck, DiagnosticReport, DiagnosticSeverity, last_crash};
use crate::services::instance_manager::InstanceStatus;
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
    /// ms desde epoch del último arranque lanzado (0 si nunca).
    pub launch_ms: u64,
}

impl InstanceContext {
    pub async fn for_handle(handle: &InstanceHandle) -> Self {
        let snapshot = handle.diagnostics_snapshot().await;
        // El guard de settings no es `Send`: se copian los valores antes del
        // `await` siguiente en vez de arrastrarlo por la función.
        let (default_min, default_max) =
            if snapshot.min_memory.is_none() || snapshot.max_memory.is_none() {
                let settings = SettingsManager::read();
                (settings.get_min_memory(), settings.get_max_memory())
            } else {
                (0, 0)
            };
        Self {
            version: snapshot.version.to_string(),
            instance_dir: snapshot.instance_dir,
            min_memory: snapshot.min_memory.unwrap_or(default_min),
            max_memory: snapshot.max_memory.unwrap_or(default_max),
            java_override: snapshot
                .overrides
                .as_ref()
                .and_then(|overrides| overrides.java_version),
            shared_dir: crate::core::PathManager::get().get_shared_dir(),
            launch_ms: handle.last_launch_ms().await,
        }
    }
}

/// Debe llamarse dentro de `spawn_blocking`.
///
/// `crash` sale de [`super::last_crash::collect`]: lo último que vio el
/// launcher al morir el proceso, que ningún chequeo de archivos ve.
pub fn run(
    handle: &InstanceHandle,
    context: InstanceContext,
    crash: Option<last_crash::CrashFacts>,
) -> DiagnosticReport {
    let mut checks = Vec::new();

    match (crash, handle.get_status()) {
        (Some(crash), _) => checks.push(check_last_crash(&crash)),
        // El arranque falló antes de correr el juego: el error lo dejó el
        // launcher, no un crash report. Un error viejo no cuenta.
        (None, InstanceStatus::Error(reason)) if recent_launch_error(&context, &reason) => checks
            .push(
                DiagnosticCheck::new("crash_launch_error", DiagnosticSeverity::Error)
                    .detail(reason),
            ),
        _ => {}
    }
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

#[cfg(test)]
pub(super) fn recent_launch_error_for_tests(context: &InstanceContext, reason: &str) -> bool {
    recent_launch_error(context, reason)
}

/// `Estado: Error` puede venir de un arranque de hace días. Solo cuenta si el
/// mensaje lleva la marca de hora del último intento.
fn recent_launch_error(context: &InstanceContext, reason: &str) -> bool {
    const WINDOW: std::time::Duration = std::time::Duration::from_secs(60 * 60);
    const PREFIX: &str = "[launch ";
    let Some(rest) = reason.strip_prefix(PREFIX) else {
        return false;
    };
    let Some((stamp, _)) = rest.split_once(']') else {
        return false;
    };
    let Ok(at) = stamp.parse::<u64>() else {
        return false;
    };
    at >= context.launch_ms && super::now_millis().saturating_sub(at) <= WINDOW.as_millis() as u64
}

fn check_last_crash(crash: &last_crash::CrashFacts) -> DiagnosticCheck {
    let cause = crash
        .findings
        .iter()
        .find(|finding| finding.code != "unknown")
        .or_else(|| crash.findings.first());
    let id = match cause {
        Some(_) if crash.repeated => "crash_repeated",
        Some(_) => "crash_recent",
        None => "crash_without_report",
    };
    let severity = cause
        .map(|finding| match finding.severity {
            CrashSeverity::Error => DiagnosticSeverity::Error,
            CrashSeverity::Warning | CrashSeverity::Info => DiagnosticSeverity::Warning,
        })
        .unwrap_or(DiagnosticSeverity::Warning);
    let mut check = DiagnosticCheck::new(id, severity)
        .param("runs", crash.runs.to_string())
        .param("count", crash.findings.len().to_string());
    if let Some(code) = cause {
        check = check.param("cause", code.code.clone());
    }
    if let Some(exit) = crash.exit_code {
        check = check.param("exitCode", exit.to_string());
    }
    if let Some(first) = crash.first_at {
        check = check.param("first", first.to_string());
    }
    if let Some(last) = crash.last_at {
        check = check.param("last", last.to_string());
    }
    for finding in &crash.findings {
        if let Some(evidence) = last_crash::evidence_summary(finding) {
            check = check.detail(evidence);
        }
    }
    if let Some(action) = cause.and_then(|finding| finding.action.clone()) {
        check = check.action(action);
    }
    check
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
