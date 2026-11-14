//! Reglas de detección de causas de crash sobre líneas ya saneadas.

use super::DiagnosticAction;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CrashSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashEvidence {
    /// 1-based dentro de la fuente analizada.
    pub line: u32,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashFinding {
    pub code: String,
    pub severity: CrashSeverity,
    pub params: std::collections::BTreeMap<String, String>,
    pub evidence: Vec<CrashEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<DiagnosticAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Needle {
    All(&'static [&'static str]),
    Any(&'static [&'static str]),
}

struct Rule {
    code: &'static str,
    severity: CrashSeverity,
    needles: &'static [Needle],
    action: Option<fn() -> DiagnosticAction>,
}

const RULES: &[Rule] = &[
    Rule {
        code: "java_unsupported_class",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "UnsupportedClassVersionError",
            "compiled by a more recent version",
        ])],
        action: Some(|| DiagnosticAction::InstallJava { version: 21 }),
    },
    Rule {
        code: "memory_out_of_memory",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "OutOfMemoryError",
            "Java heap space",
            "insufficient memory",
            "GC overhead limit exceeded",
        ])],
        action: None,
    },
    Rule {
        code: "java_missing_main",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "Could not find or load main class",
            "JAVA_HOME",
            "Error: could not open",
        ])],
        action: None,
    },
    Rule {
        code: "mods_mixin_failed",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "Mixin apply failed",
            "MixinTransformerError",
            "mixin.invalid",
        ])],
        action: Some(|| DiagnosticAction::OpenMods),
    },
    Rule {
        code: "mods_duplicate",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&["Duplicate mods:", "Found duplicate mod"])],
        action: Some(|| DiagnosticAction::OpenMods),
    },
    Rule {
        code: "mods_missing_dependency",
        severity: CrashSeverity::Error,
        needles: &[
            Needle::Any(&[
                "Missing or unsupported mandatory dependencies",
                "mandatory dependency",
            ]),
            Needle::All(&["Mod file", "requires", "currently missing"]),
        ],
        action: Some(|| DiagnosticAction::OpenMods),
    },
    Rule {
        code: "mods_incompatible",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "NoSuchMethodError",
            "NoClassDefFoundError",
            "Incompatible mod set",
            "No functional mod set",
        ])],
        action: Some(|| DiagnosticAction::OpenMods),
    },
    Rule {
        code: "graphics_opengl",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "GLFW error",
            "Pixel format not accelerated",
            "Failed to create window",
            "OpenGL 3.2",
        ])],
        action: None,
    },
    Rule {
        code: "network_download",
        severity: CrashSeverity::Warning,
        needles: &[Needle::Any(&[
            "Failed to download",
            "UnknownHostException",
            "Connection reset",
            "Connection refused",
        ])],
        action: None,
    },
    Rule {
        code: "world_corrupt",
        severity: CrashSeverity::Error,
        needles: &[
            Needle::All(&["level.dat", "corrupt"]),
            Needle::All(&["Chunk file", "corrupt"]),
        ],
        action: Some(|| DiagnosticAction::OpenWorlds),
    },
    Rule {
        code: "auth_session",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "Invalid session",
            "Failed to verify username",
            "User not authenticated",
        ])],
        action: None,
    },
];

const EVIDENCE_MAX_CHARS: usize = 240;
const EVIDENCE_MAX_PER_FINDING: usize = 3;

fn haystack(line: &str) -> String {
    line.to_lowercase()
}

fn matches(needle: &Needle, line: &str) -> bool {
    let line = haystack(line);
    match needle {
        Needle::All(phrases) => phrases
            .iter()
            .all(|phrase| line.contains(&haystack(phrase))),
        Needle::Any(phrases) => phrases
            .iter()
            .any(|phrase| line.contains(&haystack(phrase))),
    }
}

fn excerpt(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.chars().count() <= EVIDENCE_MAX_CHARS {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(EVIDENCE_MAX_CHARS).collect();
    out.push('…');
    out
}

/// `lines` debe venir ya saneado; aquí no se vuelve a sanear.
pub fn analyze_lines(lines: &[&str]) -> Vec<CrashFinding> {
    let mut findings: Vec<CrashFinding> = Vec::new();

    for rule in RULES {
        let mut evidence = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if evidence.len() >= EVIDENCE_MAX_PER_FINDING {
                break;
            }
            if rule.needles.iter().any(|needle| matches(needle, line)) {
                evidence.push(CrashEvidence {
                    line: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    text: excerpt(line),
                });
            }
        }
        if evidence.is_empty() {
            continue;
        }
        findings.push(CrashFinding {
            code: rule.code.to_string(),
            severity: rule.severity,
            params: Default::default(),
            evidence,
            action: rule.action.map(|build| build()),
        });
    }

    // Una incompatibilidad genérica no aporta nada si ya hay una causa más
    // concreta sobre mods (mixin, duplicados o dependencias faltantes).
    let has_specific_mods_cause = findings.iter().any(|finding| {
        matches!(
            finding.code.as_str(),
            "mods_mixin_failed" | "mods_duplicate" | "mods_missing_dependency"
        )
    });
    if has_specific_mods_cause {
        findings.retain(|finding| finding.code != "mods_incompatible");
    }

    findings.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.code.cmp(&b.code)));
    findings
}

pub fn crash_description(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        let rest = line.trim().strip_prefix("Description:")?;
        let value = rest.trim();
        (!value.is_empty()).then(|| excerpt(value))
    })
}

pub fn unknown_finding(description: Option<&str>) -> Option<CrashFinding> {
    let description = description?;
    let mut params = std::collections::BTreeMap::new();
    params.insert("description".to_string(), description.to_string());
    Some(CrashFinding {
        code: "unknown".to_string(),
        severity: CrashSeverity::Info,
        params,
        evidence: Vec::new(),
        action: None,
    })
}
