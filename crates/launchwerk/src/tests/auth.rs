use crate::auth::storage::SecureStorage;
use crate::auth::{AccountType, MinecraftUser};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn secret_path(root: &Path, uuid: &str, key_name: &str) -> PathBuf {
    root.join("secrets")
        .join(format!("{}_{}.enc", uuid, key_name))
}

fn key_path(root: &Path) -> PathBuf {
    root.join("vault.key")
}

fn premium_user() -> MinecraftUser {
    MinecraftUser::premium(
        "test_user".to_string(),
        "test_uuid".to_string(),
        "secret_token".to_string(),
        Some("refresh_token".to_string()),
    )
}

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
    for account_type in [AccountType::Microsoft, AccountType::Yggdrasil] {
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
    let user = premium_user();

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
fn debug_does_not_expose_tokens() {
    let user = MinecraftUser::premium(
        "debug_user".into(),
        "debug_uuid".into(),
        "secret-access-value".into(),
        Some("secret-refresh-value".into()),
    );
    let debug = format!("{user:?}");

    assert!(!debug.contains("secret-access-value"));
    assert!(!debug.contains("secret-refresh-value"));
    assert!(debug.contains("<redacted>"));
    assert!(debug.contains("debug_user"));
}

#[test]
fn test_secure_storage_save_load() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        let mut user = MinecraftUser::premium(
            "test_user_storage".to_string(),
            "uuid_storage".to_string(),
            "token123".to_string(),
            Some("refresh123".to_string()),
        );

        user.save_tokens().expect("Failed to save tokens");
        assert!(secret_path(dir.path(), &user.uuid, "access").exists());
        assert!(secret_path(dir.path(), &user.uuid, "refresh").exists());
        assert!(key_path(dir.path()).exists());

        user.access_token.clear();
        user.refresh_token = None;

        user.load_tokens().expect("Failed to load tokens");

        assert_eq!(user.access_token, "token123");
        assert_eq!(user.refresh_token, Some("refresh123".to_string()));

        user.delete_tokens().unwrap();
        assert!(!secret_path(dir.path(), &user.uuid, "access").exists());
        assert!(!secret_path(dir.path(), &user.uuid, "refresh").exists());
    });
}

#[test]
fn v2_storage_roundtrip_does_not_leak_plaintext() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        SecureStorage::save("uuid-v2", "access", "super-secret-token").unwrap();

        let raw = fs::read(secret_path(dir.path(), "uuid-v2", "access")).unwrap();
        assert!(raw.starts_with(b"CLSECRET"));
        assert_eq!(raw[8], 2);
        assert!(!String::from_utf8_lossy(&raw).contains("super-secret-token"));

        let loaded = SecureStorage::load("uuid-v2", "access").unwrap();
        assert_eq!(loaded, "super-secret-token");
    });
}

#[test]
fn legacy_tokens_are_migrated_to_v2() {
    let dir = tempdir().unwrap();
    let storage = SecureStorage::with_root(dir.path());
    storage
        .save_legacy("uuid-legacy", "access", "legacy-token")
        .unwrap();

    let legacy_raw = fs::read(secret_path(dir.path(), "uuid-legacy", "access")).unwrap();
    assert!(!legacy_raw.starts_with(b"CLSECRET"));

    SecureStorage::with_test_root(dir.path(), || {
        let loaded = SecureStorage::load("uuid-legacy", "access").unwrap();
        assert_eq!(loaded, "legacy-token");
    });

    let migrated_raw = fs::read(secret_path(dir.path(), "uuid-legacy", "access")).unwrap();
    assert!(migrated_raw.starts_with(b"CLSECRET"));
    assert_eq!(migrated_raw[8], 2);
}

#[test]
fn copying_a_secret_to_another_account_fails() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        SecureStorage::save("uuid-a", "access", "token-a").unwrap();
        SecureStorage::save("uuid-b", "access", "token-b").unwrap();

        fs::copy(
            secret_path(dir.path(), "uuid-a", "access"),
            secret_path(dir.path(), "uuid-b", "access"),
        )
        .unwrap();

        assert!(SecureStorage::load("uuid-b", "access").is_err());
    });
}

#[test]
fn copying_a_secret_to_another_token_type_fails() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        SecureStorage::save("uuid-a", "access", "token-a").unwrap();
        fs::copy(
            secret_path(dir.path(), "uuid-a", "access"),
            secret_path(dir.path(), "uuid-a", "refresh"),
        )
        .unwrap();

        assert!(SecureStorage::load("uuid-a", "refresh").is_err());
    });
}

#[test]
fn missing_vault_key_does_not_destroy_v2_tokens() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        SecureStorage::save("uuid-key", "access", "token-key").unwrap();
        fs::remove_file(key_path(dir.path())).unwrap();

        assert!(SecureStorage::load("uuid-key", "access").is_err());
        assert!(secret_path(dir.path(), "uuid-key", "access").exists());
    });
}

#[test]
fn corrupted_v2_files_are_rejected() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        SecureStorage::save("uuid-corrupt", "access", "token-corrupt").unwrap();
        let path = secret_path(dir.path(), "uuid-corrupt", "access");

        let mut raw = fs::read(&path).unwrap();
        raw[8] = 99;
        fs::write(&path, &raw).unwrap();
        assert!(SecureStorage::load("uuid-corrupt", "access").is_err());

        fs::write(&path, b"CLSECRET\x02short").unwrap();
        assert!(SecureStorage::load("uuid-corrupt", "access").is_err());
    });
}

#[test]
fn unsafe_account_identifiers_are_rejected() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        assert!(SecureStorage::save("../escape", "access", "token").is_err());
        assert!(SecureStorage::save("..", "access", "token").is_err());
        assert!(SecureStorage::save("a/b", "access", "token").is_err());
        assert!(SecureStorage::save("a\\b", "access", "token").is_err());
        assert!(SecureStorage::save("uuid", "other", "token").is_err());
        assert!(!dir.path().join("escape_access.enc").exists());
    });
}

#[test]
fn saving_removes_stale_credentials() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        let mut user = MinecraftUser::premium(
            "stale_user".into(),
            "uuid-stale".into(),
            "access".into(),
            Some("refresh".into()),
        );
        user.save_tokens().unwrap();
        assert!(secret_path(dir.path(), &user.uuid, "refresh").exists());

        user.refresh_token = None;
        user.save_tokens().unwrap();
        assert!(!secret_path(dir.path(), &user.uuid, "refresh").exists());

        let mut yggdrasil = MinecraftUser::yggdrasil(
            "ygg_user".into(),
            "uuid-ygg".into(),
            "access".into(),
            "client".into(),
            "https://example.invalid".into(),
        );
        yggdrasil.save_tokens().unwrap();
        assert!(secret_path(dir.path(), &yggdrasil.uuid, "client").exists());

        yggdrasil.client_token = None;
        yggdrasil.save_tokens().unwrap();
        assert!(!secret_path(dir.path(), &yggdrasil.uuid, "client").exists());
    });
}

#[cfg(unix)]
#[test]
fn storage_permissions_are_restrictive() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        SecureStorage::save("uuid-perms", "access", "token").unwrap();

        let file_mode = fs::metadata(secret_path(dir.path(), "uuid-perms", "access"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, 0o600);

        let dir_mode = fs::metadata(dir.path().join("secrets"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);

        let key_mode = fs::metadata(key_path(dir.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(key_mode, 0o600);
    });
}

#[test]
fn path_components_cannot_escape_secret_storage() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        for uuid in [
            "../escape",
            "..",
            "a/b",
            "a\\b",
            "/absolute",
            "C:\\drive",
            "x/../../y",
            "a\0b",
            "",
        ] {
            assert!(
                SecureStorage::save(uuid, "access", "token").is_err(),
                "save accepted {uuid:?}"
            );
            assert!(
                SecureStorage::load(uuid, "access").is_err(),
                "load accepted {uuid:?}"
            );
            assert!(
                SecureStorage::delete(uuid, "access").is_err(),
                "delete accepted {uuid:?}"
            );
        }
        for key_name in ["../access", "access/../refresh", "", "other", "access\0"] {
            assert!(SecureStorage::save("uuid-key-name", key_name, "token").is_err());
            assert!(SecureStorage::load("uuid-key-name", key_name).is_err());
            assert!(SecureStorage::delete("uuid-key-name", key_name).is_err());
        }
    });
    assert!(!dir.path().join("escape_access.enc").exists());
}

#[cfg(unix)]
#[test]
fn symlinked_secret_file_is_rejected_and_external_file_is_untouched() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let secrets = dir.path().join("secrets");
    fs::create_dir_all(&secrets).unwrap();
    let outside = dir.path().join("outside.txt");
    fs::write(&outside, b"outside-secret").unwrap();
    symlink(&outside, secret_path(dir.path(), "uuid-link", "access")).unwrap();

    SecureStorage::with_test_root(dir.path(), || {
        assert!(SecureStorage::load("uuid-link", "access").is_err());
        assert!(SecureStorage::save("uuid-link", "access", "new-secret").is_err());
        assert!(SecureStorage::delete("uuid-link", "access").is_err());
    });

    assert_eq!(fs::read(&outside).unwrap(), b"outside-secret");
    assert!(
        secret_path(dir.path(), "uuid-link", "access")
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[cfg(unix)]
#[test]
fn symlinked_directory_at_secret_name_is_rejected() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let secrets = dir.path().join("secrets");
    let outside = dir.path().join("outside");
    fs::create_dir_all(&secrets).unwrap();
    fs::create_dir(&outside).unwrap();
    symlink(&outside, secret_path(dir.path(), "uuid-dir", "access")).unwrap();

    SecureStorage::with_test_root(dir.path(), || {
        assert!(SecureStorage::load("uuid-dir", "access").is_err());
        assert!(SecureStorage::save("uuid-dir", "access", "token").is_err());
        assert!(SecureStorage::delete("uuid-dir", "access").is_err());
    });

    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn symlinked_secrets_directory_is_rejected() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let outside = dir.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, dir.path().join("secrets")).unwrap();

    SecureStorage::with_test_root(dir.path(), || {
        assert!(SecureStorage::save("uuid-root", "access", "token").is_err());
        assert!(SecureStorage::load("uuid-root", "access").is_err());
        assert!(SecureStorage::delete("uuid-root", "access").is_err());
    });

    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[test]
fn replaced_secrets_directory_does_not_leak_or_redirect_credentials() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        SecureStorage::save("uuid-replace", "access", "original-token").unwrap();
    });

    let secrets = dir.path().join("secrets");
    let moved = dir.path().join("secrets-old");
    fs::rename(&secrets, &moved).unwrap();
    fs::create_dir(&secrets).unwrap();

    SecureStorage::with_test_root(dir.path(), || {
        assert!(SecureStorage::load("uuid-replace", "access").is_err());
    });

    #[cfg(unix)]
    {
        fs::remove_dir(&secrets).unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, &secrets).unwrap();

        SecureStorage::with_test_root(dir.path(), || {
            assert!(SecureStorage::save("uuid-replace", "access", "token").is_err());
            assert!(SecureStorage::load("uuid-replace", "access").is_err());
            assert!(SecureStorage::delete("uuid-replace", "access").is_err());
        });
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    }
}

#[test]
fn atomic_writes_leave_no_temporary_files() {
    let dir = tempdir().unwrap();
    SecureStorage::with_test_root(dir.path(), || {
        SecureStorage::save("uuid-temp", "access", "token").unwrap();
    });

    let names: Vec<_> = fs::read_dir(dir.path().join("secrets"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["uuid-temp_access.enc"]);
    assert!(names.iter().all(|name| !name.starts_with(".cubic-write-")));
}

#[cfg(unix)]
#[test]
fn concurrent_secret_symlink_swaps_do_not_escape() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let dir = tempdir().unwrap();
    let secrets = dir.path().join("secrets");
    let outside = dir.path().join("outside.txt");
    fs::create_dir_all(&secrets).unwrap();
    fs::write(&outside, b"outside-secret").unwrap();

    SecureStorage::with_test_root(dir.path(), || {
        SecureStorage::save("uuid-race", "access", "initial").unwrap();
    });

    let target = secret_path(dir.path(), "uuid-race", "access");
    let stop = Arc::new(AtomicBool::new(false));
    let attacker_target = target.clone();
    let attacker_outside = outside.clone();
    let attacker_stop = Arc::clone(&stop);
    let attacker = std::thread::spawn(move || {
        while !attacker_stop.load(Ordering::Relaxed) {
            let _ = fs::remove_file(&attacker_target);
            let _ = std::os::unix::fs::symlink(&attacker_outside, &attacker_target);
            let _ = fs::remove_file(&attacker_target);
            let _ = fs::write(&attacker_target, b"inside");
        }
    });

    SecureStorage::with_test_root(dir.path(), || {
        for _ in 0..500 {
            let _ = SecureStorage::save("uuid-race", "access", "new-token");
            let _ = SecureStorage::load("uuid-race", "access");
            let _ = SecureStorage::delete("uuid-race", "access");
        }
    });

    stop.store(true, Ordering::Relaxed);
    attacker.join().unwrap();
    assert_eq!(fs::read(&outside).unwrap(), b"outside-secret");
}

#[cfg(windows)]
#[test]
fn windows_reparse_secret_file_is_rejected() {
    let dir = tempdir().unwrap();
    let secrets = dir.path().join("secrets");
    fs::create_dir_all(&secrets).unwrap();
    let outside = dir.path().join("outside.txt");
    fs::write(&outside, b"outside-secret").unwrap();
    let target = secret_path(dir.path(), "uuid-reparse", "access");
    if std::os::windows::fs::symlink_file(&outside, &target).is_err() {
        eprintln!("skipping Windows reparse test: symlink creation is unavailable");
        return;
    }

    SecureStorage::with_test_root(dir.path(), || {
        assert!(SecureStorage::load("uuid-reparse", "access").is_err());
        assert!(SecureStorage::save("uuid-reparse", "access", "new").is_err());
        assert!(SecureStorage::delete("uuid-reparse", "access").is_err());
    });
    assert_eq!(fs::read(&outside).unwrap(), b"outside-secret");
}

#[cfg(windows)]
#[test]
fn windows_reparse_secrets_directory_is_rejected() {
    let dir = tempdir().unwrap();
    let outside = dir.path().join("outside");
    fs::create_dir(&outside).unwrap();
    if std::os::windows::fs::symlink_dir(&outside, dir.path().join("secrets")).is_err() {
        eprintln!("skipping Windows reparse test: symlink creation is unavailable");
        return;
    }

    SecureStorage::with_test_root(dir.path(), || {
        assert!(SecureStorage::save("uuid-root", "access", "token").is_err());
        assert!(SecureStorage::load("uuid-root", "access").is_err());
        assert!(SecureStorage::delete("uuid-root", "access").is_err());
    });
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

/// Token de Minecraft con la forma real: header.payload.signature en base64url.
fn jwt_with_exp(exp: i64) -> String {
    use base64::{Engine as _, engine::general_purpose};
    let header = general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256"}"#);
    let payload = general_purpose::URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp}}}"#));
    format!("{header}.{payload}.signature")
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn access_token_freshness_reads_the_jwt_expiry() {
    let now = now_unix();
    let mut user = MinecraftUser::premium(
        "Steve".into(),
        "id".into(),
        jwt_with_exp(now + 3600),
        Some("refresh".into()),
    );

    assert_eq!(user.access_token_expiry(), Some(now + 3600));
    assert!(user.access_token_is_fresh(600));

    // Dentro del margen de seguridad: hay que refrescar.
    user.access_token = jwt_with_exp(now + 60);
    assert!(!user.access_token_is_fresh(600));

    // Ya expirado.
    user.access_token = jwt_with_exp(now - 10);
    assert!(!user.access_token_is_fresh(0));

    // Tokens opacos (Yggdrasil) no son JWT: se refresca como siempre.
    user.access_token = "opaque-token".into();
    assert_eq!(user.access_token_expiry(), None);
    assert!(!user.access_token_is_fresh(0));
}
