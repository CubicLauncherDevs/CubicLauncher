//! Fuentes de un crash: crash reports, snapshot del anillo en memoria y cola
//! de `latest.log`/`debug.log`.

use super::rules::{self, CrashFinding};
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub(super) const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub(super) const MAX_LINES: usize = 20_000;
/// Por encima de esto sanitizar todas las líneas cuesta más de lo que aporta y
/// se sanea solo la evidencia devuelta.
const SANITIZE_MAX_LINES: usize = 5_000;
/// Las entradas de cabecera (`Description:`, `Minecraft Version:`…) van arriba.
const CONTEXT_MAX_LINES: usize = 500;
/// `Description:` es la primera entrada: no hace falta buscar más abajo.
const DESCRIPTION_MAX_LINES: usize = 100;
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
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            if !name.ends_with(".txt") {
                return None;
            }
            let meta = std::fs::symlink_metadata(entry.path()).ok()?;
            if !meta.file_type().is_file() {
                return None;
            }
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
            if !name.ends_with(".txt") {
                return None;
            }
            let meta = std::fs::symlink_metadata(entry.path()).ok()?;
            if !meta.file_type().is_file() {
                return None;
            }
            let modified = meta.modified().ok()?;
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
    let len = file.metadata()?.len();
    let mut buffer = Vec::with_capacity(len.min(MAX_FILE_BYTES) as usize);
    file.by_ref()
        .take(MAX_FILE_BYTES)
        .read_to_end(&mut buffer)?;
    Ok((
        String::from_utf8_lossy(&buffer).into_owned(),
        len > MAX_FILE_BYTES,
    ))
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

fn sanitize_lines(lines: &[&str]) -> Vec<String> {
    let mut sanitized = crate::services::launcher::sanitize_log_lines(lines.iter().copied());
    // `clave=valor` enmascararía el código de salida, que no es un secreto.
    for line in sanitized.iter_mut() {
        let trimmed = line.trim_start();
        if let Some(value) = trimmed.strip_prefix("Exit Code:") {
            *line = format!(
                "Exit Code: {}",
                value.split_whitespace().next().unwrap_or("")
            );
        }
    }
    sanitized
}

/// `lines` son la cabecera ya saneada y `search` todo el texto saneado: el
/// cargador se anuncia más abajo que los campos de cabecera.
pub(super) fn extract_context(lines: &[&str], search: &[&str]) -> CrashContext {
    fn field(lines: &[&str], prefix: &str) -> Option<String> {
        lines.iter().find_map(|line| {
            let rest = line.trim().strip_prefix(prefix)?;
            let value = rest.trim();
            (!value.is_empty()).then(|| truncate_chars(value, 120))
        })
    }
    // `Description:` va arriba: buscarlo solo en la cabecera evita que un mod
    // imprima el suyo al final y se muestre como causa del crash.
    let head = &lines[..lines.len().min(DESCRIPTION_MAX_LINES)];
    let description = head.iter().find_map(|line| {
        let rest = line.trim().strip_prefix("Description:")?;
        let value = rest.trim();
        (!value.is_empty()).then(|| truncate_chars(value, rules::EVIDENCE_MAX_CHARS))
    });
    let lowered: Vec<String> = search.iter().map(|line| line.to_lowercase()).collect();
    let loader = match rules::Loader::detect(&lowered) {
        rules::Loader::Forge => Some("Forge"),
        rules::Loader::Fabric => Some("Fabric"),
        rules::Loader::NeoForge => Some("NeoForge"),
        rules::Loader::Vanilla | rules::Loader::Unknown => None,
    }
    .map(str::to_string);

    CrashContext {
        description,
        minecraft_version: field(lines, "Minecraft Version:"),
        java_version: field(lines, "Java Version:"),
        exit_code: field(lines, "Exit Code:").and_then(|value| value.parse().ok()),
        loader,
    }
}

/// `exit_code` es la salida observada por el launcher; solo se usa si el texto
/// no la trae (`Exit Code:` de un crash report).
pub(super) fn analysis_from_text(
    text: &str,
    extra_truncated: bool,
    source: CrashSource,
    report_name: Option<String>,
    exit_code: Option<i32>,
) -> CrashAnalysis {
    let all: Vec<&str> = text.lines().collect();
    let truncated = all.len() > MAX_LINES;
    let lines = &all[..all.len().min(MAX_LINES)];
    let (mut context, mut findings) = if lines.len() <= SANITIZE_MAX_LINES {
        // Ruta normal: se sanea una vez y el contexto y las reglas reutilizan
        // las mismas líneas.
        let sanitized = sanitize_lines(lines);
        let refs: Vec<&str> = sanitized.iter().map(String::as_str).collect();
        (extract_context(&refs, &refs), rules::analyze_lines(&refs))
    } else {
        // Reporte enorme: el contexto sale de la cabecera y las reglas sanean
        // solo la evidencia que devuelven.
        (
            extract_context(&lines[..lines.len().min(CONTEXT_MAX_LINES)], lines),
            rules::analyze_lines_sanitized(lines),
        )
    };
    // El snapshot del anillo y `latest.log` no imprimen la salida del proceso;
    // la reporta el launcher al detectar el crash.
    if context.exit_code.is_none() {
        context.exit_code = exit_code;
    }
    // Un texto vacío no es un crash: el diagnóstico comprueba la salud aunque
    // no haya nada que analizar.
    if findings.is_empty() && all.iter().any(|line| !line.trim().is_empty()) {
        findings = rules::unknown_finding(context.description.as_deref())
            .into_iter()
            .collect();
    }
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
        None,
    ))
}

/// Prioridad: crash report reciente > snapshot del anillo > `latest.log`.
pub async fn analyze_instance(instance_id: &str, instance_dir: &Path) -> CrashAnalysis {
    match analyze_recent_crash(instance_id, instance_dir, false).await {
        Some(analysis) => analysis,
        None => analysis_from_text("", false, CrashSource::LatestLog, None, None),
    }
}

/// Prioridad: crash report reciente > snapshot del anillo > `latest.log`.
///
/// En el último recurso, `newest_first` prefiere `debug.log`: el diagnóstico
/// necesita el final del log más reciente para comprobar la salud, mientras
/// que un crash sin reporte ya dejó su contenido en el snapshot (y la ventana
/// de logs sigue prefiriendo `latest.log`, el mismo que muestra).
pub(super) async fn analyze_recent_crash(
    instance_id: &str,
    instance_dir: &Path,
    newest_first: bool,
) -> Option<CrashAnalysis> {
    let crashed_at = crate::services::launcher::crash_log_snapshot_time(instance_id);
    let dir = instance_dir.to_path_buf();
    let report = tokio::task::spawn_blocking(move || {
        let path = newest_report_since(&dir, crashed_at)?;
        let (text, truncated) = read_head(&path).ok()?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string);
        Some(analysis_from_text(
            &text,
            truncated,
            CrashSource::CrashReport,
            name,
            None,
        ))
    })
    .await
    .ok()
    .flatten();
    if let Some(analysis) = report {
        return Some(analysis);
    }

    let exit_code = crate::services::launcher::crash_log_snapshot_exit_code(instance_id);
    if let Some(snapshot) = crate::services::launcher::crash_log_snapshot_text(instance_id).await {
        return Some(analysis_from_text(
            &snapshot,
            false,
            CrashSource::Ring,
            None,
            exit_code,
        ));
    }
    let dir = instance_dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        // Minecraft rota `latest.log` en cada arranque: `debug.log` se llena
        // después y, en un crash sin reporte, es donde queda la salida real.
        let names = if newest_first {
            ["debug.log", "latest.log"]
        } else {
            ["latest.log", "debug.log"]
        };
        for name in names {
            let Some(path) = safe_file(&logs_dir(&dir), name) else {
                continue;
            };
            let Ok((text, truncated)) = read_tail(&path, LOG_TAIL_BYTES) else {
                continue;
            };
            return Some(analysis_from_text(
                &text,
                truncated,
                CrashSource::LatestLog,
                None,
                exit_code,
            ));
        }
        None
    })
    .await
    .ok()
    .flatten()
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
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(LOG_TAIL_LINES);
    sanitize_lines(&lines[start..])
}
