//! Último crash de la instancia tal como lo contó el launcher: salida del
//! proceso, cuántos arranques seguidos fallaron y las causas que las reglas
//! encuentran en sus logs.

use super::crash;
use super::rules::{CrashFinding, EVIDENCE_MAX_CHARS};
use crate::services::launcher::{crash_log_snapshot_text, crash_run_history};
use std::path::Path;
use std::time::Duration;

/// Un crash más viejo que esto ya no cuenta como "la instancia se rompe":
/// los chequeos de salud dejan de señalarlo.
pub(crate) const STALE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
/// Crashes más separados que esto son intentos de otra sesión, no un bucle.
const RUN_WINDOW: Duration = Duration::from_secs(10 * 60);
const MAX_FINDINGS: usize = 4;

#[derive(Clone, Debug)]
pub(crate) struct CrashFacts {
    /// Arranques que se encadenaron fallando hasta el último crash.
    pub runs: usize,
    pub repeated: bool,
    /// ms desde epoch del primer y último crash de la racha.
    pub first_at: Option<u64>,
    pub last_at: Option<u64>,
    pub exit_code: Option<i32>,
    pub findings: Vec<CrashFinding>,
}

pub(crate) async fn collect(
    instance_id: &str,
    instance_dir: &Path,
    last_launch_ms: u64,
) -> Option<CrashFacts> {
    let now = super::now_millis();
    // Solo los crashes de este arranque: uno anterior ya lo resolvió el
    // usuario o lo explicaba otra sesión.
    let runs: Vec<_> = crash_run_history(instance_id)
        .into_iter()
        .filter(|run| run.at >= last_launch_ms)
        .filter(|run| now >= run.at && now - run.at <= STALE_AFTER.as_millis() as u64)
        .collect();
    let &last = runs.last()?;
    // Un único crash alcanza para avisar; varios seguidos explican un bucle.
    let streak = runs
        .iter()
        .rev()
        .take_while(|run| now.saturating_sub(run.at) <= RUN_WINDOW.as_millis() as u64)
        .count()
        .max(1);

    // El launcher conserva el snapshot del último crash, así que el análisis
    // de ahora describe justo este arranque.
    let analysis = crash::analyze_recent_crash(instance_id, instance_dir, true).await;
    let has_evidence = analysis.is_some() || crash_log_snapshot_text(instance_id).await.is_some();
    if streak == 1 && !has_evidence {
        return None;
    }
    let findings = analysis
        .map(|analysis| pick_findings(analysis.findings))
        .unwrap_or_default();

    Some(CrashFacts {
        runs: streak,
        repeated: streak > 1,
        first_at: runs.get(runs.len() - streak).map(|run| run.at),
        last_at: Some(last.at),
        exit_code: last.exit_code,
        findings,
    })
}

/// Causas que pueden explicar un crash, ordenadas por gravedad. `unknown` solo
/// queda si no hay nada conocido, y `content_pack_invalid` no tumba el juego.
fn pick_findings(mut findings: Vec<CrashFinding>) -> Vec<CrashFinding> {
    findings.retain(|finding| finding.code != "content_pack_invalid");
    let has_known = findings.iter().any(|finding| finding.code != "unknown");
    if has_known {
        findings.retain(|finding| finding.code != "unknown");
    }
    findings.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.code.cmp(&b.code)));
    findings.truncate(MAX_FINDINGS);
    findings
}

/// Evidencia del hallazgo en una sola línea, para `details` del chequeo.
pub(crate) fn evidence_summary(finding: &CrashFinding) -> Option<String> {
    let text = &finding.evidence.first()?.text;
    if text.chars().count() <= EVIDENCE_MAX_CHARS {
        return Some(text.clone());
    }
    let mut out: String = text.chars().take(EVIDENCE_MAX_CHARS).collect();
    out.push('…');
    Some(out)
}
