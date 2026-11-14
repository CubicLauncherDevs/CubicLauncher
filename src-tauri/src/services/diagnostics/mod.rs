//! Diagnóstico local de instancias: comprobaciones de salud y análisis de
//! crashes. Nunca modifica la instancia; solo devuelve acciones sugeridas.

pub(crate) mod crash;
pub(crate) mod health;
pub(crate) mod rules;

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

impl DiagnosticSeverity {
    fn rank(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warning => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DiagnosticAction {
    ReinstallVersion { version: String },
    InstallJava { version: u8 },
    OpenMods,
    OpenWorlds,
}

/// El texto se resuelve en el frontend a partir de `id` y `params`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticCheck {
    pub id: String,
    pub severity: DiagnosticSeverity,
    pub params: BTreeMap<String, String>,
    pub details: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<DiagnosticAction>,
}

impl DiagnosticCheck {
    pub(super) fn new(id: &str, severity: DiagnosticSeverity) -> Self {
        Self {
            id: id.to_string(),
            severity,
            params: BTreeMap::new(),
            details: Vec::new(),
            action: None,
        }
    }

    pub(super) fn param(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.insert(key.to_string(), value.into());
        self
    }

    pub(super) fn detail(mut self, value: impl Into<String>) -> Self {
        self.details.push(value.into());
        self
    }

    pub(super) fn action(mut self, action: DiagnosticAction) -> Self {
        self.action = Some(action);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub generated_at: u64,
    pub checks: Vec<DiagnosticCheck>,
    pub errors: usize,
    pub warnings: usize,
    pub launch_ready: bool,
}

impl DiagnosticReport {
    pub(super) fn from_checks(mut checks: Vec<DiagnosticCheck>) -> Self {
        checks.sort_by_key(|check| check.severity.rank());
        let errors = checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Error)
            .count();
        let warnings = checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Warning)
            .count();
        Self {
            generated_at: now_millis(),
            checks,
            errors,
            warnings,
            launch_ready: errors == 0,
        }
    }
}

pub(super) fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "../../tests/services/diagnostics.rs"]
mod tests;
