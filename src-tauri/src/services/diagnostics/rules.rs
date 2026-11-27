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

/// Formato del arranque. Separa causas que comparten firma: el `NoSuchMethodError`
/// de un mod sin actualizar y el de un shader sin actualizar son el mismo texto.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Loader {
    #[default]
    Unknown,
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
}

impl Loader {
    pub(super) fn detect(lowered: &[String]) -> Self {
        // Los mods se cargan antes que el mundo: basta con mirar el arranque para
        // no confundir un `forge` mencionado por un mod con el cargador real.
        let head = &lowered[..lowered.len().min(1_500)];
        let has = |phrases: &[&str]| {
            head.iter()
                .any(|line| phrases.iter().any(|phrase| line.contains(phrase)))
        };

        if has(&[
            "--launchtarget neoforgedclient",
            "neoforge version",
            "neoforged",
        ]) {
            Self::NeoForge
        } else if has(&[
            "--fml.forgeversion",
            "forgemodloader",
            "forge mod loader version",
        ]) {
            Self::Forge
        } else if has(&["fabric-loader", "fabric loader version", "--fabric."]) {
            Self::Fabric
        } else if has(&[
            "[main/info]: setting user:",
            "backend library: lwjgl",
            "loading for game",
        ]) {
            Self::Vanilla
        } else {
            // El extracto puede empezar después del arranque.
            Self::Unknown
        }
    }

    /// ¿El arranque está en manos de un cargador de mods?
    fn is_modded(self) -> bool {
        matches!(self, Self::Fabric | Self::Forge | Self::NeoForge)
    }
}

/// Contexto que cambia cómo se lee un hallazgo. El texto del log lo controlan
/// mods y packs de recursos, así que una causa ajena al juego puede coincidir
/// con la evidencia de otra: estas señales descartan el falso positivo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContextSignal {
    /// Ruta externa al juego: `.minecraft/resourcepacks/…` y similares.
    ExternalAssets,
    /// `[ErrorHandler]` de los fixes de mods de Forge.
    ErrorHandlerMod,
    /// Un fix de shaders/recursos de Forge abre un `StackParseError`.
    GraphicsLibraryLoad,
}

impl ContextSignal {
    fn detect(self, lowered: &[String]) -> bool {
        match self {
            Self::ExternalAssets => {
                const DIVERGENCES: &[&str] = &[
                    "resourcepacks",
                    "shaderpacks",
                    "texturepacks",
                    "mods/",
                    "config/",
                ];
                const READS: &[&str] = &[
                    "couldn't read",
                    "could not read",
                    "failed to read",
                    "unable to read",
                    "not a valid ",
                    "malformed ",
                ];
                lowered.iter().any(|line| {
                    DIVERGENCES.iter().any(|path| line.contains(path))
                        && READS.iter().any(|read| line.contains(read))
                })
            }
            Self::ErrorHandlerMod => lowered.iter().any(|line| line.contains("[errorhandler]")),
            Self::GraphicsLibraryLoad => lowered.iter().any(|line| {
                const ERRORS: &[&str] = &[
                    "stack parse error",
                    "couldn't parse stack",
                    "could not parse stack",
                ];
                line.contains("unsatisfiedlinkerror")
                    && line.contains("native lwjgl")
                    && ERRORS.iter().any(|error| line.contains(error))
            }),
        }
    }
}

/// Una regla solo cuenta si el formato del arranque es uno de los suyos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Loaders {
    Any,
    /// Solo cuando hay un cargador de mods o no se pudo leer el arranque
    /// (un extracto de log puede empezar después de cargarse los mods).
    Modded,
    /// Solo cuando el arranque no pasa por un cargador de mods.
    Plain,
}

impl Loaders {
    fn allows(self, loader: Loader) -> bool {
        match self {
            Self::Any => true,
            Self::Modded => loader != Loader::Vanilla,
            Self::Plain => !loader.is_modded(),
        }
    }
}

struct Rule {
    code: &'static str,
    severity: CrashSeverity,
    /// `line` en minúsculas, como el resto de needles.
    needles: &'static [Needle],
    action: Option<fn() -> DiagnosticAction>,
    loaders: Loaders,
    /// Se descarta si el contexto cumple algún [`ContextSignal`].
    suppressed_by: &'static [ContextSignal],
}

const RULES: &[Rule] = &[
    Rule {
        code: "java_unsupported_class",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "unsupportedclassversionerror",
            "compiled by a more recent version",
        ])],
        action: Some(|| DiagnosticAction::InstallJava { version: 21 }),
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "java_heap_too_big",
        severity: CrashSeverity::Error,
        // La JVM no pudo reservar la RAM pedida: el arranque muere antes de cargar
        // nada, así que aquí no hay reglas de mods que valgan.
        needles: &[Needle::Any(&[
            "insufficient space for",
            "could not reserve enough space",
            "could not allocate enough space",
            "invalid maximum heap size",
            "invalid initial heap size",
        ])],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "memory_out_of_memory",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "outofmemoryerror",
            "java heap space",
            "insufficient memory",
            "gc overhead limit exceeded",
        ])],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "java_missing_main",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "could not find or load main class",
            "java_home",
            "error: could not open",
        ])],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "mods_mixin_failed",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "mixin apply failed",
            "mixintransformererror",
            "mixin.invalid",
        ])],
        action: Some(|| DiagnosticAction::OpenMods),
        // Un `trying to apply mixin` de un crash de mods es un mod sin
        // actualizar, no una causa de mixin propia.
        loaders: Loaders::Plain,
        suppressed_by: &[],
    },
    Rule {
        code: "mods_duplicate",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&["duplicate mods:", "found duplicate mod"])],
        action: Some(|| DiagnosticAction::OpenMods),
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "mods_missing_dependency",
        severity: CrashSeverity::Error,
        needles: &[
            Needle::Any(&[
                "missing or unsupported mandatory dependencies",
                "mandatory dependency",
            ]),
            Needle::All(&["mod file", "requires", "currently missing"]),
        ],
        action: Some(|| DiagnosticAction::OpenMods),
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "mods_incompatible",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "nosuchmethoderror",
            "noclassdeffounderror",
            "incompatible mod set",
            "no functional mod set",
        ])],
        action: Some(|| DiagnosticAction::OpenMods),
        // Un mod puede romper también el arranque vanilla al que se copió, así
        // que no se descarta por formato. Se retira si hay una causa concreta.
        loaders: Loaders::Any,
        // Los mods de Forge marcan `[ErrorHandler]` los fallos que captura el
        // conjunto de parches del arranque.
        suppressed_by: &[ContextSignal::ErrorHandlerMod],
    },
    Rule {
        code: "mods_outdated",
        severity: CrashSeverity::Error,
        // Fabric nombra el mod exacto; Forge apunta a su parche o a su mixin.
        needles: &[
            Needle::All(&["fabric", "requires version"]),
            Needle::All(&["incompatible", "of mod "]),
            Needle::Any(&[
                "mod resolution failed",
                "trying to apply mixin",
                "mixin apply failed",
            ]),
        ],
        action: Some(|| DiagnosticAction::OpenMods),
        loaders: Loaders::Modded,
        suppressed_by: &[],
    },
    Rule {
        code: "mods_registry",
        severity: CrashSeverity::Error,
        // El registro del juego reporta todas las entradas que fallan a la vez,
        // normalmente por un mod, un datapack o una receta rota.
        needles: &[Needle::Any(&[
            "failure during registry",
            "failed to create registry entry",
            "errors during the loading of a registry",
            "could not parse recipe",
            "failed to parse recipe",
        ])],
        action: Some(|| DiagnosticAction::OpenMods),
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "java_missing_native_lib",
        severity: CrashSeverity::Error,
        // libawt_xawt.so sin libXtst/libXrender/libX11 es un JDK que no puede
        // cargar AWT; LWJGL lo dispara antes de crear la ventana.
        needles: &[
            Needle::All(&["unsatisfiedlinkerror", "libawt"]),
            Needle::All(&["unsatisfiedlinkerror", "libxtst"]),
            Needle::All(&["unsatisfiedlinkerror", "libxrender"]),
        ],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[ContextSignal::GraphicsLibraryLoad],
    },
    Rule {
        code: "graphics_opengl",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "glfw error",
            "pixel format not accelerated",
            "failed to create window",
            "opengl 3.2",
        ])],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "graphics_xrandr_missing",
        severity: CrashSeverity::Error,
        // LWJGL 2 enumera los modos ejecutando `xrandr -q`; si el comando no
        // está en PATH, LinuxDisplay indexa una lista vacía de pantallas.
        needles: &[Needle::Any(&[
            "linuxdisplay.getavailabledisplaymodes",
            "exception in xrandr.populate()",
        ])],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "graphics_shader_pack",
        severity: CrashSeverity::Warning,
        needles: &[Needle::Any(&[
            "shaderpack",
            "shader pack",
            "shaders/",
            "invalid shader",
            "shader compilation failed",
            "failed to compile shader",
        ])],
        action: None,
        loaders: Loaders::Modded,
        suppressed_by: &[],
    },
    Rule {
        code: "network_download",
        severity: CrashSeverity::Warning,
        needles: &[Needle::Any(&[
            "failed to download",
            "unknownhostexception",
            "connection reset",
            "connection refused",
        ])],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "world_corrupt",
        severity: CrashSeverity::Error,
        needles: &[
            Needle::All(&["level.dat", "corrupt"]),
            Needle::All(&["chunk file", "corrupt"]),
        ],
        action: Some(|| DiagnosticAction::OpenWorlds),
        loaders: Loaders::Any,
        // Un pack o shader roto hace saltar lecturas corruptas de archivos que
        // nunca son del guardado.
        suppressed_by: &[ContextSignal::ExternalAssets],
    },
    Rule {
        code: "content_pack_invalid",
        severity: CrashSeverity::Warning,
        // El juego ignora el pack roto y sigue: no es la causa del crash, pero
        // explica fallos de texturas, idioma o modelos que el usuario reporta.
        needles: &[Needle::Any(&[
            "failed to load datapacks",
            "couldn't load datapacks",
            "errors in currently selected datapacks",
            "non-fatal errors in packages",
            "couldn't read",
            "could not read",
            "failed to read",
            "unable to read",
            "not a valid ",
            "malformed ",
            "pack.mcmeta",
        ])],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "auth_session",
        severity: CrashSeverity::Error,
        needles: &[Needle::Any(&[
            "invalid session",
            "failed to verify username",
            "user not authenticated",
        ])],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "auth_skin",
        severity: CrashSeverity::Warning,
        // La sesión sigue viva: solo no se pudo bajar el perfil de skins, que
        // Minecraft reintenta o resuelve con la skin por defecto.
        needles: &[Needle::Any(&[
            "couldn't fetch profile properties",
            "could not fetch profile properties",
            "failed to fetch profile properties",
            "failed to get profile",
            "textures.minecraft.net",
        ])],
        action: None,
        loaders: Loaders::Any,
        suppressed_by: &[],
    },
    Rule {
        code: "mods_sodium_graphics",
        severity: CrashSeverity::Error,
        // Sodium altera el renderizador entero: su fallo se confunde con el
        // driver. Sin Sodium, la causa gráfica es el driver y no el mod.
        needles: &[Needle::Any(&[
            "sodium",
            "embeddium",
            "iris",
            "oculus",
            "optifine",
        ])],
        action: Some(|| DiagnosticAction::OpenMods),
        // Sin Sodium ni OptiFine, el fallo gráfico es del driver.
        loaders: Loaders::Plain,
        suppressed_by: &[],
    },
];

pub(super) const EVIDENCE_MAX_CHARS: usize = 240;
const EVIDENCE_MAX_PER_FINDING: usize = 3;

/// `line` debe venir ya en minúsculas. Las frases de [`RULES`] también lo están,
/// así el análisis no asigna memoria por cada regla y needle.
fn matches(needle: &Needle, line: &str) -> bool {
    match needle {
        Needle::All(phrases) => phrases.iter().all(|phrase| line.contains(phrase)),
        Needle::Any(phrases) => phrases.iter().any(|phrase| line.contains(phrase)),
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

fn sanitized_excerpt(line: &str) -> String {
    excerpt(&crate::services::launcher::sanitize_log_line(line))
}

/// `lines` debe venir ya saneado; aquí no se vuelve a sanear.
pub fn analyze_lines(lines: &[&str]) -> Vec<CrashFinding> {
    analyze_lines_with(lines, excerpt)
}

/// Igual que [`analyze_lines`], pero sanea la evidencia con el filtro del
/// usuario.
pub(super) fn analyze_lines_sanitized(lines: &[&str]) -> Vec<CrashFinding> {
    analyze_lines_with(lines, sanitized_excerpt)
}

fn analyze_lines_with(
    lines: &[&str],
    format_evidence: impl Fn(&str) -> String,
) -> Vec<CrashFinding> {
    let lowered: Vec<String> = lines.iter().map(|line| line.to_lowercase()).collect();
    let loader = Loader::detect(&lowered);
    // Cada señal se evalúa una vez y solo si alguna regla la consulta.
    let mut signals: Vec<ContextSignal> = Vec::new();
    for rule in RULES {
        for signal in rule.suppressed_by {
            if !signals.contains(signal) && signal.detect(&lowered) {
                signals.push(*signal);
            }
        }
    }
    let mut findings: Vec<CrashFinding> = Vec::new();

    for rule in RULES {
        if !rule.loaders.allows(loader)
            || signals
                .iter()
                .any(|signal| rule.suppressed_by.contains(signal))
        {
            continue;
        }
        let mut evidence = Vec::new();
        for (index, line) in lowered.iter().enumerate() {
            if evidence.len() >= EVIDENCE_MAX_PER_FINDING {
                break;
            }
            if rule.needles.iter().any(|needle| matches(needle, line)) {
                evidence.push(CrashEvidence {
                    line: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    text: format_evidence(lines[index]),
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
    // concreta sobre mods (falta de actualización, mixin, duplicados o
    // dependencias faltantes).
    let has_specific_mods_cause = findings.iter().any(|finding| {
        matches!(
            finding.code.as_str(),
            "mods_outdated" | "mods_mixin_failed" | "mods_duplicate" | "mods_missing_dependency"
        )
    });
    if has_specific_mods_cause {
        findings.retain(|finding| finding.code != "mods_incompatible");
    }

    findings.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.code.cmp(&b.code)));
    findings
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

#[cfg(test)]
pub fn needles_are_lowercase() -> bool {
    RULES.iter().all(|rule| {
        rule.needles.iter().all(|needle| {
            let phrases = match needle {
                Needle::All(phrases) | Needle::Any(phrases) => phrases,
            };
            phrases
                .iter()
                .all(|phrase| phrase.chars().all(|c| !c.is_ascii_uppercase()))
        })
    })
}
