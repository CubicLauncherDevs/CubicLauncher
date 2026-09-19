use crate::auth::MinecraftUser;

#[test]
fn offline_accounts_keep_their_identity_after_recreation_and_serialization() {
    let first = MinecraftUser::cracked("Steve");
    assert_eq!(first.uuid, "5627dd98-e6be-3c21-b8a8-e92344183641");
    assert_eq!(MinecraftUser::cracked("Steve").uuid, first.uuid);
    let mut reloaded: MinecraftUser =
        serde_json::from_str(&serde_json::to_string(&first).unwrap()).unwrap();
    assert!(!reloaded.ensure_offline_uuid());
    assert_eq!(reloaded.uuid, first.uuid);
    reloaded.username = "RenamedPlayer".into();
    assert!(!reloaded.ensure_offline_uuid());
    assert_eq!(reloaded.uuid, first.uuid);
}

#[test]
fn offline_uuid_repair_preserves_valid_ids_and_leaves_online_accounts_alone() {
    for value in ["", "invalid", "00000000-0000-0000-0000-000000000000"] {
        let mut offline = MinecraftUser::cracked("Steve");
        offline.uuid = value.into();
        assert!(offline.ensure_offline_uuid());
        assert_eq!(offline.uuid, "5627dd98-e6be-3c21-b8a8-e92344183641");
        assert!(!offline.ensure_offline_uuid());
    }
    for value in [
        "fd317fac-4605-4fde-bca7-c9847b745491",
        "fd317fac46054fdebca7c9847b745491",
    ] {
        let mut offline = MinecraftUser::cracked("Steve");
        offline.uuid = value.into();
        assert!(!offline.ensure_offline_uuid());
        assert_eq!(offline.uuid, value);
    }
    for account_type in [
        crate::auth::AccountType::Microsoft,
        crate::auth::AccountType::Yggdrasil,
    ] {
        let mut online =
            MinecraftUser::premium("Steve".into(), "online-id".into(), "token".into(), None);
        online.user_type = account_type;
        assert!(!online.ensure_offline_uuid());
        assert_eq!(online.uuid, "online-id");
        assert_eq!(online.access_token, "token");
    }
}

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
