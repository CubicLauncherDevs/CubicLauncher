//! Fuentes de un crash: crash reports, snapshot del anillo en memoria y cola
//! de `latest.log`/`debug.log`.

use super::rules::{self, CrashFinding};
use crate::core::sanitize::redact_log_line;
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub(super) const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub(super) const MAX_LINES: usize = 20_000;
const LOG_TAIL_BYTES: u64 = 512 * 1024;
pub(super) const LOG_TAIL_LINES: usize = 1000;

const CRASH_REPORTS_DIR: &str = "crash-reports";
const LOGS_DIR: &str = "logs";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashSource {
    CrashReport,
    Ring,
    LatestLog,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashContext {
    pub description: Option<String>,
    pub minecraft_version: Option<String>,
    pub java_version: Option<String>,
    pub exit_code: Option<i32>,
    pub loader: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashAnalysis {
    pub source: CrashSource,
    pub report_name: Option<String>,
    pub exit_code: Option<i32>,
    pub context: CrashContext,
    pub findings: Vec<CrashFinding>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReportEntry {
    pub name: String,
    pub modified: u64,
    pub size: u64,
}

pub(super) fn regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_file())
        .unwrap_or(false)
}

fn safe_file(base: &Path, name: &str) -> Option<PathBuf> {
    crate::core::validate_filename(name).ok()?;
    let path = base.join(name);
    regular_file(&path).then_some(path)
}

pub(super) fn crash_reports_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join(CRASH_REPORTS_DIR)
}

fn logs_dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join(LOGS_DIR)
}

pub(super) fn validate_report_name(name: &str) -> Result<(), String> {
    crate::core::validate_filename(name)?;
    if !name.ends_with(".txt") {
        return Err("El crash report debe ser un archivo .txt".into());
    }
    Ok(())
}

pub fn list_crash_reports(instance_dir: &Path) -> Vec<CrashReportEntry> {
    let dir = crash_reports_dir(instance_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut reports: Vec<CrashReportEntry> = entries
        .flatten()
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            if !name.ends_with(".txt") {
                return None;
            }
            let meta = entry.metadata().ok()?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0);
            Some(CrashReportEntry {
                name,
                modified,
                size: meta.len(),
            })
        })
        .collect();
    reports.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
    reports
}

pub(super) fn newest_report_since(
    instance_dir: &Path,
    since: Option<SystemTime>,
) -> Option<PathBuf> {
    let dir = crash_reports_dir(instance_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return None;
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            if !name.ends_with(".txt") || !regular_file(&entry.path()) {
                return None;
            }
            let modified = entry
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())?;
            // Un reporte de un crash anterior explica otro arranque, no este.
            if let Some(since) = since
                && modified
                    < since
                        .checked_sub(Duration::from_secs(1))
                        .unwrap_or(SystemTime::UNIX_EPOCH)
            {
                return None;
            }
            Some((modified, entry.path()))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    text.chars().take(max).collect::<String>() + "…"
}

fn read_head(path: &Path) -> std::io::Result<(String, bool)> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_FILE_BYTES)
        .read_to_end(&mut buffer)?;
    let truncated = std::fs::metadata(path)
        .map(|meta| meta.len() > MAX_FILE_BYTES)
        .unwrap_or(false);
    Ok((String::from_utf8_lossy(&buffer).into_owned(), truncated))
}

fn read_tail(path: &Path, bytes: u64) -> std::io::Result<(String, bool)> {
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(bytes);
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = Vec::new();
    file.take(bytes).read_to_end(&mut buffer)?;
    // Se corta por byte, así que se descarta la primera línea parcial.
    let text = String::from_utf8_lossy(&buffer).into_owned();
    let text = if start > 0 {
        match text.find('\n') {
            Some(index) => text[index + 1..].to_string(),
            None => String::new(),
        }
    } else {
        text
    };
    Ok((text, start > 0))
}

/// El anillo de logs ya aplica el filtro del usuario al entrar; sin esto aquí,
/// una línea larga podría saltarse el patrón `token=valor`.
fn sanitize_all(lines: Vec<&str>) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| {
            let generic = redact_log_line(line);
            crate::services::launcher::sanitize_with_user(&generic)
        })
        .collect()
}

pub(super) fn sanitized_lines(text: &str) -> (Vec<String>, bool) {
    let all: Vec<&str> = text.lines().collect();
    let truncated = all.len() > MAX_LINES;
    let lines = sanitize_all(all.into_iter().take(MAX_LINES).collect());
    (lines, truncated)
}

pub(super) fn extract_context(lines: &[String]) -> CrashContext {
    fn field(lines: &[String], prefix: &str) -> Option<String> {
        lines.iter().find_map(|line| {
            let rest = line.trim().strip_prefix(prefix)?;
            let value = rest.trim();
            (!value.is_empty()).then(|| truncate_chars(value, 120))
        })
    }
    let loader = ["--fml.forgeVersion", "Forge Mod Loader version"]
        .iter()
        .find(|needle| lines.iter().any(|line| line.contains(**needle)))
        .map(|_| "Forge".to_string())
        .or_else(|| {
            lines
                .iter()
                .any(|line| line.contains("Fabric Loader"))
                .then(|| "Fabric".to_string())
        })
        .or_else(|| {
            lines
                .iter()
                .any(|line| line.contains("neoforge"))
                .then(|| "NeoForge".to_string())
        });

    CrashContext {
        description: rules::crash_description(
            &lines.iter().map(String::as_str).collect::<Vec<_>>(),
        ),
        minecraft_version: field(lines, "Minecraft Version:"),
        java_version: field(lines, "Java Version:"),
        exit_code: field(lines, "Exit Code:").and_then(|value| value.parse().ok()),
        loader,
    }
}

/// `clave=valor` enmascararía el código de salida, que no es un secreto.
fn strip_exit_code(lines: &mut [String]) {
    for line in lines.iter_mut() {
        let trimmed = line.trim_start();
        if let Some(value) = trimmed.strip_prefix("Exit Code:") {
            *line = format!(
                "Exit Code: {}",
                value.split_whitespace().next().unwrap_or("")
            );
        }
    }
}

pub(super) fn analysis_from_text(
    text: &str,
    extra_truncated: bool,
    source: CrashSource,
    report_name: Option<String>,
) -> CrashAnalysis {
    let (mut lines, truncated) = sanitized_lines(text);
    let context = extract_context(&lines);
    if context.exit_code.is_some() {
        strip_exit_code(&mut lines);
    }
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let findings = rules::analyze_lines(&refs);
    let findings = if findings.is_empty() {
        rules::unknown_finding(context.description.as_deref())
            .into_iter()
            .collect()
    } else {
        findings
    };
    CrashAnalysis {
        source,
        report_name,
        exit_code: context.exit_code,
        context,
        findings,
        truncated: truncated || extra_truncated,
    }
}

pub fn analyze_report(instance_dir: &Path, name: &str) -> Result<CrashAnalysis, String> {
    validate_report_name(name)?;
    let base = crash_reports_dir(instance_dir);
    let path = safe_file(&base, name).ok_or("Crash report no encontrado")?;
    let (text, truncated) = read_head(&path).map_err(|e| e.to_string())?;
    Ok(analysis_from_text(
        &text,
        truncated,
        CrashSource::CrashReport,
        Some(name.to_string()),
    ))
}

/// Prioridad: crash report reciente > snapshot del anillo > `latest.log`.
pub async fn analyze_instance(instance_id: &str, instance_dir: &Path) -> CrashAnalysis {
    let crashed_at = crate::services::launcher::crash_log_snapshot_time(instance_id);

    if let Some(path) = newest_report_since(instance_dir, crashed_at)
        && let Ok((text, truncated)) = read_head(&path)
    {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string);
        return analysis_from_text(&text, truncated, CrashSource::CrashReport, name);
    }

    if let Some(lines) = crate::services::launcher::crash_log_snapshot_lines(instance_id).await
        && !lines.is_empty()
    {
        let text = lines
            .iter()
            .map(|line| line.text.as_ref())
            .collect::<Vec<_>>()
            .join("\n");
        return analysis_from_text(&text, false, CrashSource::Ring, None);
    }

    let base = logs_dir(instance_dir);
    for name in ["latest.log", "debug.log"] {
        let Some(path) = safe_file(&base, name) else {
            continue;
        };
        if let Ok((text, truncated)) = read_tail(&path, LOG_TAIL_BYTES) {
            return analysis_from_text(&text, truncated, CrashSource::LatestLog, None);
        }
    }

    analysis_from_text("", false, CrashSource::LatestLog, None)
}

/// Cola saneada de `logs/latest.log`.
pub fn log_tail(instance_dir: &Path) -> Vec<String> {
    let base = logs_dir(instance_dir);
    let Some(path) = safe_file(&base, "latest.log") else {
        return Vec::new();
    };
    let Ok((text, _)) = read_tail(&path, LOG_TAIL_BYTES) else {
        return Vec::new();
    };
    let (lines, _) = sanitized_lines(&text);
    let skip = lines.len().saturating_sub(LOG_TAIL_LINES);
    lines.into_iter().skip(skip).collect()
}
