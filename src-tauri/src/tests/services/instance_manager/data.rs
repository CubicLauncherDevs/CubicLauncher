use super::*;

#[test]
fn legacy_instances_default_to_original_minecraft_jar() {
    let data: InstanceData =
        serde_json::from_str(r#"{"name":"Old","version":"1.12.2","last_played":0,"uuid":"test"}"#)
            .unwrap();
    assert!(!data.minecraft_jar.is_active());
    assert!(data.minecraft_jar.mods.is_empty());
}

#[test]
fn legacy_instances_default_playtime_to_zero() {
    let data: InstanceData =
        serde_json::from_str(r#"{"name":"Old","version":"1.12.2","last_played":0,"uuid":"test"}"#)
            .unwrap();
    assert_eq!(data.playtime_seconds, 0);
}

#[test]
fn test_validate_name_empty() {
    assert!(validate_instance_name("").is_err());
}

#[test]
fn test_validate_name_non_ascii() {
    assert!(validate_instance_name("ñoña").is_err());
}

#[test]
fn test_validate_name_too_long() {
    assert!(validate_instance_name("a".repeat(usize::from(MAX_LEN) + 1).as_str()).is_err());
}

#[test]
fn test_validate_name_with_slash() {
    assert!(validate_instance_name("a/b").is_err());
}

#[test]
fn test_validate_name_with_backslash() {
    assert!(validate_instance_name("a\\b").is_err());
}

#[test]
fn test_validate_name_with_null() {
    assert!(validate_instance_name("a\0b").is_err());
}

#[test]
fn test_validate_name_with_dotdot() {
    assert!(validate_instance_name("..").is_err());
}

#[test]
fn test_validate_name_valid() {
    assert!(validate_instance_name("MyInstance").is_ok());
}

#[test]
fn test_validate_name_max_length() {
    assert!(validate_instance_name("a".repeat(usize::from(MAX_LEN)).as_str()).is_ok());
}

#[test]
fn test_get_loader_fabric() {
    let data = InstanceData::new("test".into(), "1.21-fabric".into(), None);
    assert_eq!(data.get_loader(), "Fabric");
}

#[test]
fn test_get_loader_forge() {
    let data = InstanceData::new("test".into(), "1.20.1-forge".into(), None);
    assert_eq!(data.get_loader(), "Forge");
}

#[test]
fn test_get_loader_quilt() {
    let data = InstanceData::new("test".into(), "1.19-quilt".into(), None);
    assert_eq!(data.get_loader(), "Quilt");
}

#[test]
fn test_get_loader_vanilla() {
    let data = InstanceData::new("test".into(), "1.21".into(), None);
    assert_eq!(data.get_loader(), "Vanilla");
}
