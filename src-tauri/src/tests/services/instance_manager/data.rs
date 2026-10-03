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

#[tokio::test]
async fn loaded_metadata_cannot_redirect_instance_paths() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("Pack");
    std::fs::create_dir(&folder).unwrap();
    let original = serde_json::json!({"name": "Pack", "version": "1.21", "last_played": 0,
        "uuid": "11111111-1111-4111-8111-111111111111"});
    for (key, value) in [
        ("name", "../outside"),
        ("name", "/outside"),
        ("name", "Other"),
        ("uuid", "../../outside"),
        ("version", "../../outside"),
        ("version", "C:\\outside"),
    ] {
        let mut poisoned = original.clone();
        poisoned[key] = value.into();
        std::fs::write(
            folder.join("instance.cub"),
            serde_json::to_vec(&poisoned).unwrap(),
        )
        .unwrap();
        assert!(
            InstanceData::load_from(root.path(), "Pack").await.is_none(),
            "{key}={value}"
        );
    }
    std::fs::write(
        folder.join("instance.cub"),
        serde_json::to_vec(&original).unwrap(),
    )
    .unwrap();
    let valid = InstanceData::load_from(root.path(), "Pack").await.unwrap();
    assert_eq!(valid.get_instance_dir(), folder);
    assert!(
        InstanceData::load_from(root.path(), "../Pack")
            .await
            .is_none()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn loading_rejects_symlinked_instance_metadata() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("Pack");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(root.path().join("outside"), br#"{"name":"Pack","version":"1.21","last_played":0,"uuid":"11111111-1111-4111-8111-111111111111"}"#).unwrap();
    std::os::unix::fs::symlink(root.path().join("outside"), folder.join("instance.cub")).unwrap();
    assert!(InstanceData::load_from(root.path(), "Pack").await.is_none());
}
