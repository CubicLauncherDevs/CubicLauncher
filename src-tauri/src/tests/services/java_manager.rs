use super::*;

#[test]
fn test_parse_java_version_from_quoted_line() {
    assert_eq!(
        parse_java_version("openjdk version \"21.0.11\" 2025-04-15\nOpenJDK Runtime Environment"),
        Some("21.0.11".to_string())
    );
    assert_eq!(
        parse_java_version("java version \"1.8.0_412\" 2024-05-16"),
        Some("1.8.0_412".to_string())
    );
}

#[test]
fn test_parse_java_version_without_quotes() {
    assert_eq!(
        parse_java_version("openjdk 21.0.11 2025-04-15"),
        Some("21.0.11".to_string())
    );
    assert_eq!(parse_java_version(""), None);
}

#[test]
fn test_parse_java_version_from_full_output() {
    // Salida completa de un runtime de Temurin.
    let output = "openjdk version \"21.0.2\" 2024-01-16\nOpenJDK Runtime Environment Temurin-21.0.2+13 (build 21.0.2+13)\nOpenJDK 64-Bit Server VM Temurin-21.0.2+13 (build 21.0.2+13, mixed mode, sharing)\n";

    assert_eq!(parse_java_version(output), Some("21.0.2".to_string()));
}
