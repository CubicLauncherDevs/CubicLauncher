use super::*;
use crate::services::instance_import::sanitize_instance_name;

#[test]
fn test_sanitize_instance_name() {
    assert_eq!(sanitize_instance_name("Mi Instancia"), "Mi Instancia");
    assert_eq!(sanitize_instance_name("ñoña"), "oa");
    assert_eq!(sanitize_instance_name("a/b"), "ab");
    assert_eq!(sanitize_instance_name(".."), "Imported");
    assert_eq!(sanitize_instance_name("<invalid>"), "invalid");
}

#[test]
fn test_parse_jvm_args_requires_override_flag() {
    let mut cfg = std::collections::HashMap::new();
    cfg.insert("JvmArgs".to_string(), "-Xmx4G -XX:+UseG1GC".to_string());
    // Sin OverrideJavaArgs=true se ignoran los argumentos del fork.
    assert!(parse_jvm_args(&cfg).is_empty());

    cfg.insert("OverrideJavaArgs".to_string(), "true".to_string());
    assert_eq!(
        parse_jvm_args(&cfg),
        vec!["-Xmx4G".to_string(), "-XX:+UseG1GC".to_string()]
    );
}

#[test]
fn test_resolve_game_version_fabric() {
    let pack = MmcPack {
        components: vec![
            MmcComponent {
                uid: UID_MINECRAFT.into(),
                version: "1.20.1".into(),
                cached_version: None,
            },
            MmcComponent {
                uid: UID_FABRIC.into(),
                version: "0.15.11".into(),
                cached_version: None,
            },
        ],
    };
    let (gv, unsupported) = resolve_game_version(&pack);
    assert!(unsupported.is_empty());
    let gv = gv.unwrap();
    assert_eq!(gv.to_version_id(), "fabric-loader-0.15.11-1.20.1");
}

#[test]
fn test_resolve_game_version_forge() {
    let pack = MmcPack {
        components: vec![
            MmcComponent {
                uid: UID_MINECRAFT.into(),
                version: "1.20.1".into(),
                cached_version: None,
            },
            MmcComponent {
                uid: UID_FORGE.into(),
                version: "47.2.0".into(),
                cached_version: None,
            },
        ],
    };
    let (gv, unsupported) = resolve_game_version(&pack);
    assert!(unsupported.is_empty());
    let gv = gv.unwrap();
    assert_eq!(gv.to_version_id(), "1.20.1-forge-47.2.0");
}

#[test]
fn test_resolve_game_version_fabric_from_cached_version() {
    let pack = MmcPack {
        components: vec![
            MmcComponent {
                uid: UID_MINECRAFT.into(),
                version: String::new(),
                cached_version: Some("1.20.1".into()),
            },
            MmcComponent {
                uid: UID_FABRIC.into(),
                version: String::new(),
                cached_version: Some("0.15.11".into()),
            },
        ],
    };
    let (gv, unsupported) = resolve_game_version(&pack);
    assert!(unsupported.is_empty());
    let gv = gv.unwrap();
    assert_eq!(gv.to_version_id(), "fabric-loader-0.15.11-1.20.1");
}

#[test]
fn test_resolve_game_version_with_extra_components() {
    let pack = MmcPack {
        components: vec![
            MmcComponent {
                uid: UID_MINECRAFT.into(),
                version: "1.21".into(),
                cached_version: None,
            },
            MmcComponent {
                uid: "org.lwjgl3".into(),
                version: "3.3.2".into(),
                cached_version: None,
            },
            MmcComponent {
                uid: UID_NEOFORGE.into(),
                version: "21.0.0".into(),
                cached_version: None,
            },
        ],
    };
    let (gv, unsupported) = resolve_game_version(&pack);
    assert!(unsupported.is_empty());
    let gv = gv.unwrap();
    assert_eq!(gv.to_version_id(), "1.21-neoforge-21.0.0");
}

#[test]
fn test_resolve_game_version_empty_version_is_skipped() {
    let pack = MmcPack {
        components: vec![
            MmcComponent {
                uid: UID_MINECRAFT.into(),
                version: String::new(),
                cached_version: None,
            },
            MmcComponent {
                uid: UID_FABRIC.into(),
                version: "0.15.11".into(),
                cached_version: None,
            },
        ],
    };
    let (gv, unsupported) = resolve_game_version(&pack);
    assert!(unsupported.is_empty());
    // Sin versión de Minecraft no se debe fabricar un GameVersion.
    assert!(gv.is_none());
}

#[test]
fn test_resolve_game_version_quilt() {
    let pack = MmcPack {
        components: vec![
            MmcComponent {
                uid: UID_MINECRAFT.into(),
                version: "1.20.1".into(),
                cached_version: None,
            },
            MmcComponent {
                uid: UID_QUILT.into(),
                version: "0.25.0".into(),
                cached_version: None,
            },
        ],
    };
    let (gv, unsupported) = resolve_game_version(&pack);
    assert!(unsupported.is_empty());
    let gv = gv.unwrap();
    assert_eq!(gv.to_version_id(), "quilt-loader-0.25.0-1.20.1");
}
