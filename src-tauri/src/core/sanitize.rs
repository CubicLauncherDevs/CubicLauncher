//! Redacción de credenciales en texto libre.
//!
//! Punto único de filtrado para cualquier texto que pueda acabar en el disco
//! del usuario, en el portapapeles o en un servicio externo (mclo.gs).
//!
//! Los patrones son deliberadamente genéricos: un token que no sepamos
//! reconocer es un token filtrado. Preferimos enmascarar de más (el valor
//! sigue siendo legible en el resto de la línea) antes que dejar pasar un JWT.

use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

/// Texto que sustituye a cualquier valor sensible.
const MASK: &str = "***";

struct Rule {
    re: Regex,
    replacement: &'static str,
}

/// Credenciales y material de sesión. El orden importa: las reglas más
/// específicas (cabecera Authorization completa) van antes que las genéricas.
static SECRET_RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    [
        // Authorization: Bearer <token> | "authorization": "Basic <b64>"
        // Consume la cabecera entera: el valor puede contener espacios.
        (
            r#"(?i)(\bauthorization["']?\s*[:=]\s*["']?)[^\r\n"']+"#,
            "${1}***",
        ),
        // Bearer <token> suelto, sin cabecera.
        (
            r#"(?i)(\bbearer\s+)[A-Za-z0-9\-._~+/]{8,}=*"#,
            "${1}***",
        ),
        // XBL3.0 x=<userhash>;<token>
        (r#"(?i)(\bXBL3\.0\s+x=)[^\s"']+"#, "${1}***"),
        // JWT compacto: header.payload.signature (base64url).
        (
            r#"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]*"#,
            MASK,
        ),
        // clave=valor, clave: valor, "clave":"valor" y --flag valor.
        (
            r#"(?i)((?:access|refresh|client|id|auth|session|xbl)[_-]?token|userhash|ticket)(["']?\s*[:=]\s*["']?|\s+)([^\s"',;&<>]+)"#,
            "${1}${2}***",
        ),
        // "session id is <id>"
        (
            r#"(?i)(session\s+id\s+is\s*[:=]?\s*)[A-Za-z0-9\-._=+/]+"#,
            "${1}***",
        ),
    ]
    .into_iter()
    .map(|(pattern, replacement)| Rule {
        re: Regex::new(pattern).expect("patrón de redacción inválido"),
        replacement,
    })
    .collect()
});

/// Datos personales que tampoco deben salir del equipo al subir un log.
static PRIVACY_RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    [
        (r#"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}"#, "<email>"),
        (r#"\b(?:\d{1,3}\.){3}\d{1,3}\b"#, "<ip>"),
    ]
    .into_iter()
    .map(|(pattern, replacement)| Rule {
        re: Regex::new(pattern).expect("patrón de privacidad inválido"),
        replacement,
    })
    .collect()
});

/// Aplica todas las reglas sin asignar memoria cuando el texto está limpio.
fn redact_with<'a>(rules: &[Rule], text: &'a str) -> Cow<'a, str> {
    let mut redacted: Option<String> = None;
    for rule in rules {
        let haystack = redacted.as_deref().unwrap_or(text);
        if rule.re.is_match(haystack) {
            redacted = Some(rule.re.replace_all(haystack, rule.replacement).into_owned());
        }
    }
    match redacted {
        Some(value) => Cow::Owned(value),
        None => Cow::Borrowed(text),
    }
}

/// Enmascara tokens de sesión (JWT, Bearer, access/refresh/client tokens)
/// conservando el resto de la línea intacto.
pub fn redact_secrets(text: &str) -> Cow<'_, str> {
    redact_with(&SECRET_RULES, text)
}

/// Enmascara credenciales y además datos personales (correo e IP).
/// Es el filtro que deben usar los logs antes de persistirse o publicarse.
pub fn redact_log_line(text: &str) -> Cow<'_, str> {
    let mut out = redact_secrets(text);
    for rule in PRIVACY_RULES.iter() {
        if rule.re.is_match(out.as_ref()) {
            out = Cow::Owned(
                rule.re
                    .replace_all(out.as_ref(), rule.replacement)
                    .into_owned(),
            );
        }
    }
    out
}

#[cfg(test)]
#[path = "../tests/core/sanitize.rs"]
mod tests;
