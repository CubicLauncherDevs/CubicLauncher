use crate::services::SettingsSnapshot;
use crate::services::java_resolution::*;
use std::path::Path;

/// Snapshot con JRE administrados sin instalar: las rutas resuelven a los
/// valores por defecto y el test no depende de lo que haya en el disco real.
fn settings() -> SettingsSnapshot {
    SettingsSnapshot {
        min_memory: 1024,
        max_memory: 2048,
        env_vars: Default::default(),
        jvm_args: Default::default(),
        jre8_managed: false,
        jre17_managed: false,
        jre21_managed: false,
        jre25_managed: false,
        jre8_path: Path::new("/jre8").into(),
        jre17_path: Path::new("/jre17").into(),
        jre21_path: Path::new("/jre21").into(),
        jre25_path: Path::new("/jre25").into(),
        open_console_on_launch: false,
    }
}

fn write_manifest(shared: &Path, version: &str, json: serde_json::Value) {
    let dir = shared.join("versions").join(version);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{version}.json")),
        serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
}

fn manifest(version: &str, java: Option<u8>, inherits: Option<&str>) -> serde_json::Value {
    let mut json = serde_json::json!({"id": version, "mainClass": "Main"});
    if let Some(java) = java {
        json["javaVersion"] =
            serde_json::json!({ "component": "java-runtime", "majorVersion": java });
    }
    if let Some(parent) = inherits {
        json["inheritsFrom"] = serde_json::json!(parent);
    }
    json
}

#[test]
fn vanilla_uses_the_manifest_java_version() {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path();
    write_manifest(shared, "1.21", manifest("1.21", Some(21), None));
    let resolution = resolve(&settings(), shared, "1.21", None);
    assert_eq!(resolution.requested, 21);
    assert_eq!(resolution.major, 21);
}

#[test]
fn inherited_java_version_is_found_in_the_parent_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path();
    write_manifest(shared, "1.20.1", manifest("1.20.1", Some(17), None));
    write_manifest(
        shared,
        "fabric-loader-0.15.11-1.20.1",
        manifest("fabric-loader-0.15.11-1.20.1", None, Some("1.20.1")),
    );
    let resolution = resolve(&settings(), shared, "fabric-loader-0.15.11-1.20.1", None);
    assert_eq!(resolution.requested, 17);
    assert_eq!(resolution.major, 17);
}

#[test]
fn missing_manifest_falls_back_to_version_inference() {
    let dir = tempfile::tempdir().unwrap();
    let resolution = resolve(&settings(), dir.path(), "1.16.5", None);
    assert!(resolution.requested >= 8);
    assert_eq!(resolution.major, resolution.requested);
}

#[test]
fn old_forge_keeps_java_8_and_modern_forge_bumps_to_17() {
    assert!(!is_forge_version_safe("36.2.25"));
    assert!(is_forge_version_safe("36.2.26"));
    assert!(is_forge_version_safe("47.2.0"));
    assert!(!is_forge_version_safe("invalid"));

    assert_eq!(bump_forge_java("1.16.5-forge-36.2.20", 8), 8);
    assert_eq!(bump_forge_java("1.20.1-forge-47.2.0", 8), 17);
    assert_eq!(bump_forge_java("1.21-neoforge-21.0.0", 8), 17);
    // Vanilla nunca se sube.
    assert_eq!(bump_forge_java("1.16.5", 8), 8);
    // Java >= 16 no se toca.
    assert_eq!(bump_forge_java("1.20.1-forge-47.2.0", 21), 21);
}

#[test]
fn instance_override_wins_over_the_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path();
    write_manifest(shared, "1.21", manifest("1.21", Some(21), None));
    let resolution = resolve(&settings(), shared, "1.21", Some(17));
    assert_eq!(resolution.requested, 21);
    assert_eq!(resolution.major, 17);
}

#[test]
fn java4shell_versions_below_the_fix_are_flagged() {
    // Advisory de OpenJDK del 2026-11-17: afecta a 8u512, 11.0.33,
    // 17.0.21.0.3, 21.0.13.0.3, 25.0.5.0.3 y 27.0.1.0.3 y anteriores. Las
    // series 22-24 no recibieron actualización, así que siguen afectadas.
    for version in [
        "1.8.0_412",
        "1.8.0_512",
        "8u512",
        "11.0.33",
        "17.0.21",
        "17.0.21+9",
        "21.0.13",
        "22.0.2",
        "23.0.2",
        "24.0.2",
        "25.0.5",
        "27.0.1",
    ] {
        assert!(is_java_known_vulnerable(version), "{version}");
    }
    for version in [
        "1.8.0_522",
        "11.0.34",
        "17.0.22",
        "21.0.14",
        "25.0.6",
        "27.0.2",
    ] {
        assert!(!is_java_known_vulnerable(version), "{version}");
    }
    // Texto no reconocible no se marca como vulnerable.
    assert!(!is_java_known_vulnerable(""));
    assert!(!is_java_known_vulnerable("error: failed"));
}

#[test]
fn java_major_parses_oracle_and_openjdk_schemes() {
    assert_eq!(java_major_and_update("1.8.0_522"), Some((8, 522)));
    assert_eq!(java_major_and_update("8u522"), Some((8, 522)));
    assert_eq!(java_major_and_update("21.0.14+12-LTS"), Some((21, 14)));
    assert_eq!(java_major_and_update("openjdk"), None);
}
