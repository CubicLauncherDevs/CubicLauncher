//! Comandos de diagnóstico de instancias.

use super::launch::validate_uuid;
use crate::core::errors::InstanceError;
use crate::services::diagnostics::DiagnosticReport;
use crate::services::diagnostics::crash::{self, CrashAnalysis, CrashReportEntry};
use crate::services::diagnostics::health;
use crate::services::{InstanceHandle, InstanceManager};
use tracing::info;

async fn handle_and_dir(id: &str) -> Result<(InstanceHandle, std::path::PathBuf), String> {
    let handle = InstanceManager::get()
        .get_handle(id)
        .await
        .ok_or_else(|| InstanceError::NotFound.to_string())?;
    let dir = handle.get_instance_dir().await;
    Ok((handle, dir))
}

#[tauri::command]
pub async fn diagnose_instance(instance_id: String) -> Result<DiagnosticReport, String> {
    validate_uuid(&instance_id)?;
    info!("Diagnóstico de la instancia {}", instance_id);
    let (handle, _dir) = handle_and_dir(&instance_id).await?;
    let context = health::InstanceContext::for_handle(&handle).await;
    tokio::task::spawn_blocking(move || health::run(&handle, context))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn analyze_instance_crash(
    instance_id: String,
    report: Option<String>,
) -> Result<CrashAnalysis, String> {
    validate_uuid(&instance_id)?;
    let (_handle, dir) = handle_and_dir(&instance_id).await?;
    if let Some(report) = report {
        return tokio::task::spawn_blocking(move || crash::analyze_report(&dir, &report))
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(crash::analyze_instance(&instance_id, &dir).await)
}

#[tauri::command]
pub async fn list_instance_crash_reports(
    instance_id: String,
) -> Result<Vec<CrashReportEntry>, String> {
    validate_uuid(&instance_id)?;
    let (_handle, dir) = handle_and_dir(&instance_id).await?;
    tokio::task::spawn_blocking(move || crash::list_crash_reports(&dir))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_instance_log_tail(instance_id: String) -> Result<Vec<String>, String> {
    validate_uuid(&instance_id)?;
    let (_handle, dir) = handle_and_dir(&instance_id).await?;
    tokio::task::spawn_blocking(move || crash::log_tail(&dir))
        .await
        .map_err(|e| e.to_string())
}
