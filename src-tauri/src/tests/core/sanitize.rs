use super::*;

const JWT: &str = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

#[test]
fn masks_a_bare_jwt() {
    let line = format!("respuesta del servidor: {JWT} (fin)");
    let out = redact_secrets(&line);
    assert!(!out.contains("eyJ"));
    assert!(!out.contains("SflKxwRJ"));
    assert!(out.starts_with("respuesta del servidor: "));
    assert!(out.ends_with(" (fin)"));
}

#[test]
fn masks_authorization_header_with_bearer() {
    let line = format!("> Authorization: Bearer {JWT}");
    let out = redact_secrets(&line);
    assert_eq!(out, "> Authorization: ***");
}

#[test]
fn masks_quoted_authorization_header() {
    let line = r#"{"authorization":"Bearer abcdef123456"}"#;
    let out = redact_secrets(line);
    assert_eq!(out, r#"{"authorization":"***"}"#);
}

#[test]
fn masks_bearer_without_header() {
    let out = redact_secrets("reenviando bearer abcdef1234567890 al proxy");
    assert_eq!(out, "reenviando bearer *** al proxy");
}

#[test]
fn masks_key_value_pairs_and_keeps_separators() {
    let out = redact_secrets("access_token=abc123&refresh_token=def456&x=1");
    assert_eq!(out, "access_token=***&refresh_token=***&x=1");
}

#[test]
fn masks_json_credentials() {
    let out = redact_secrets(r#"{"access_token":"abc123","client_token":"def456"}"#);
    assert_eq!(out, r#"{"access_token":"***","client_token":"***"}"#);
}

#[test]
fn masks_launch_flag_value() {
    let line = format!("--accessToken {JWT} --username Steve");
    let out = redact_secrets(&line);
    assert!(!out.contains("eyJ"));
    assert!(out.contains("--accessToken ***"));
    assert!(out.ends_with("--username Steve"));
}

#[test]
fn masks_session_id_and_xbl_ticket() {
    let out = redact_secrets("session id is abc123def456 XBL3.0 x=userhash;token");
    assert_eq!(out, "session id is *** XBL3.0 x=***");
}

#[test]
fn leaves_clean_lines_borrowed() {
    let line = "Setting user: Steve";
    let out = redact_secrets(line);
    assert!(matches!(out, Cow::Borrowed(_)));
    assert_eq!(out, line);
}

#[test]
fn log_lines_also_hide_email_and_ip() {
    let out = redact_log_line("login steve@example.com from 192.168.1.20 ok");
    assert_eq!(out, "login <email> from <ip> ok");
}

#[test]
fn redaction_is_idempotent() {
    let line = format!("Authorization: Bearer {JWT} user=steve@example.com");
    let once = redact_log_line(&line);
    let twice = redact_log_line(&once);
    assert_eq!(once, twice);
}
