use crate::auth::MinecraftUser;

#[test]
fn test_token_serialization_skip() {
    let user = MinecraftUser::premium(
        "test_user".to_string(),
        "test_uuid".to_string(),
        "secret_token".to_string(),
        Some("refresh_token".to_string()),
    );

    let json = serde_json::to_string(&user).unwrap();

    // El JSON no debería contener los tokens
    assert!(!json.contains("secret_token"));
    assert!(!json.contains("refresh_token"));
    assert!(!json.contains("access_token"));

    // Al deserializar, los tokens deberían estar vacíos (o por defecto)
    let deserialized: MinecraftUser = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.access_token, "");
    assert_eq!(deserialized.refresh_token, None);
    assert_eq!(deserialized.username, "test_user");
    assert_eq!(deserialized.uuid, "test_uuid");
}

#[test]
fn test_secure_storage_save_load() {
    let mut user = MinecraftUser::premium(
        "test_user_storage".to_string(),
        "uuid_storage".to_string(),
        "token123".to_string(),
        Some("refresh123".to_string()),
    );

    // Guardar tokens
    user.save_tokens().expect("Failed to save tokens");

    // Limpiar tokens en memoria
    user.access_token = String::new();
    user.refresh_token = None;

    // Cargar tokens
    user.load_tokens().expect("Failed to load tokens");

    assert_eq!(user.access_token, "token123");
    assert_eq!(user.refresh_token, Some("refresh123".to_string()));

    // Borrar tokens
    user.delete_tokens().unwrap();
}
