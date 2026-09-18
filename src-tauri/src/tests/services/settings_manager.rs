use super::*;

/// `SettingsManager::default()` debe inicializar todos los campos con
/// los valores por defecto definidos en las funciones `default_*`.
/// Verifica: username, min_memory, max_memory, language, auto_updates,
/// close_launcher_on_play, show_snapshots, show_alpha, show_unstable_loaders,
/// theme, dirty, env_vars, jvm_args y console_history_limit.
#[test]
fn test_default_values() {
    let s = SettingsManager::default();
    assert_eq!(s.get_user().username, "Steve");
    assert_eq!(s.active_user_idx, 0);
    assert_eq!(s.min_memory, 1024);
    assert_eq!(s.max_memory, 2048);
    assert_eq!(s.language, "es");
    assert!(s.auto_updates);
    assert!(s.close_launcher_on_play);
    assert!(!s.show_snapshots);
    assert!(!s.show_alpha);
    assert!(!s.show_unstable_loaders);
    assert!(!s.open_console_on_launch);
    assert_eq!(s.console_history_limit, 3000);
    assert!(s.console_show_level_tags);
    assert_eq!(s.theme, "dark");
    assert!(s.dirty);
    assert!(s.env_vars.is_empty());
    assert!(s.jvm_args.is_empty());
    assert!(!s.prominent_notifications);
    assert!(!s.reduce_animations);
    assert!(!s.disable_blur_effects);
    assert!(!s.disable_infinite_animations);
    assert!(!s.disable_skin3d_animations);
    assert!(!s.reduce_log_animations);
}

/// Valores en GB (min=2, max=4) deben convertirse a MB (2048, 4096)
/// y marcar `dirty` como `true`.
#[test]
fn test_migrate_converts_gb_to_mb() {
    let mut s = SettingsManager {
        min_memory: 2,
        max_memory: 4,
        dirty: false,
        ..Default::default()
    };
    s.migrate();
    assert_eq!(s.min_memory, 2048);
    assert_eq!(s.max_memory, 4096);
    assert!(s.dirty);
}

/// Valores legacy en MB (min=2048, max=4096) pasan por v1→v2 (÷1024)
/// y luego v2→v3 (×1024), dando el mismo resultado (idempotente).
#[test]
fn test_migrate_legacy_mb_roundtrip() {
    let mut s = SettingsManager {
        min_memory: 2048,
        max_memory: 4096,
        dirty: false,
        ..Default::default()
    };
    s.migrate();
    assert_eq!(s.min_memory, 2048);
    assert_eq!(s.max_memory, 4096);
    assert!(s.dirty);
}

/// Un valor impar legacy en MB como 1500 debe pasar por v1→v2 (→1)
/// y luego v2→v3 (→1024).
#[test]
fn test_migrate_odd_legacy_mb() {
    let mut s = SettingsManager {
        min_memory: 1500,
        dirty: false,
        ..Default::default()
    };
    s.migrate();
    assert_eq!(s.min_memory, 1024);
}

/// Un valor de max_memory = 128 es ambiguo (128 MB o 128 GB).
/// Como está fuera de ambos rangos (no >128 para v1, no <=64 para v2),
/// se deja intacto.
#[test]
fn test_migrate_ambiguous_128() {
    let mut s = SettingsManager {
        min_memory: 2,
        max_memory: 128,
        dirty: false,
        ..Default::default()
    };
    s.migrate();
    assert_eq!(s.min_memory, 2048);
    assert_eq!(s.max_memory, 128);
}

/// Serializar y deserializar un `SettingsManager` con serde_json debe
/// preservar todos los campos públicos. El campo `dirty` tiene
/// `#[serde(skip)]` por lo que siempre se deserializa como `false`.
#[test]
fn test_serde_roundtrip() {
    let s = SettingsManager {
        prominent_notifications: true,
        ..Default::default()
    };
    let json = serde_json::to_string(&s).unwrap();
    let deserialized: SettingsManager = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.active_user_idx, s.active_user_idx);
    assert_eq!(deserialized.min_memory, s.min_memory);
    assert_eq!(deserialized.max_memory, s.max_memory);
    assert_eq!(deserialized.language, s.language);
    assert_eq!(deserialized.console_history_limit, s.console_history_limit);
    assert_eq!(
        deserialized.console_show_level_tags,
        s.console_show_level_tags
    );
    assert_eq!(deserialized.theme, s.theme);
    assert!(deserialized.prominent_notifications);
    assert_eq!(deserialized.reduce_animations, s.reduce_animations);
    assert_eq!(deserialized.disable_blur_effects, s.disable_blur_effects);
    assert_eq!(
        deserialized.disable_infinite_animations,
        s.disable_infinite_animations
    );
    assert_eq!(
        deserialized.disable_skin3d_animations,
        s.disable_skin3d_animations
    );
    assert_eq!(deserialized.reduce_log_animations, s.reduce_log_animations);
    assert!(!deserialized.dirty);
}

#[test]
fn test_legacy_settings_default_to_corner_notifications() {
    let mut json = serde_json::to_value(SettingsManager::default()).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("prominent_notifications");
    let settings: SettingsManager = serde_json::from_value(json).unwrap();
    assert!(!settings.prominent_notifications);
}

/// El límite del historial de consola debe quedar dentro del rango
/// permitido (100-5000) durante la migración.
#[test]
fn test_migrate_clamps_console_history_limit() {
    let mut s = SettingsManager {
        console_history_limit: 99999,
        dirty: false,
        ..Default::default()
    };
    s.migrate();
    assert_eq!(s.console_history_limit, 5000);
    assert!(s.dirty);

    let mut s = SettingsManager {
        console_history_limit: 50,
        dirty: false,
        ..Default::default()
    };
    s.migrate();
    assert_eq!(s.console_history_limit, 100);
    assert!(s.dirty);
}

#[test]
fn notification_preferences_migrate_legacy_modes_only_once() {
    use crate::services::notification_preferences::{NotificationPosition, NotificationSize};
    for prominent in [false, true] {
        let mut settings: SettingsManager = serde_json::from_value(serde_json::json!({
            "prominent_notifications": prominent,
        }))
        .unwrap();
        assert!(settings.notification_preferences.is_none());
        settings.normalize_notification_preferences();
        let prefs = settings.notification_preferences.as_mut().unwrap();
        assert!(!prefs.enabled);
        assert_eq!(
            prefs.position,
            if prominent {
                NotificationPosition::TopCenter
            } else {
                NotificationPosition::TopRight
            }
        );
        assert_eq!(
            prefs.size,
            if prominent {
                NotificationSize::Wide
            } else {
                NotificationSize::Compact
            }
        );
        assert_eq!(prefs.uppercase_title, prominent);
        assert_eq!(prefs.duration_seconds, if prominent { 8 } else { 5 });
        prefs.position = NotificationPosition::BottomLeft;
        prefs.uppercase_title = false;
        let json = serde_json::to_vec(&settings).unwrap();
        let mut reloaded: SettingsManager = serde_json::from_slice(&json).unwrap();
        reloaded.normalize_notification_preferences();
        assert_eq!(
            reloaded.notification_preferences,
            settings.notification_preferences
        );
        assert!(!reloaded.dirty);
    }
}

#[test]
fn notification_preferences_preserve_independent_controls_and_all_positions() {
    for position in [
        "top-left",
        "top-center",
        "top-right",
        "bottom-left",
        "bottom-center",
        "bottom-right",
    ] {
        for size in ["compact", "normal", "wide"] {
            let json = serde_json::json!({
                "prominent_notifications": true,
                "notification_preferences": {
                    "enabled": true,
                    "position": position, "size": size,
                    "title_size": 24, "message_size": 11,
                    "uppercase_title": false, "bold_title": true,
                    "duration_seconds": 30,
                },
            });
            let mut settings: SettingsManager = serde_json::from_value(json.clone()).unwrap();
            settings.normalize_notification_preferences();
            let saved = serde_json::to_value(&settings).unwrap();
            assert_eq!(
                saved["notification_preferences"],
                json["notification_preferences"]
            );
            let mut reloaded: SettingsManager = serde_json::from_value(saved).unwrap();
            reloaded.migrate();
            assert_eq!(
                reloaded.notification_preferences,
                settings.notification_preferences
            );
        }
    }
}

#[test]
fn notification_preferences_bound_numeric_values_and_default_missing_fields() {
    let mut settings: SettingsManager = serde_json::from_value(serde_json::json!({
        "notification_preferences": {
            "position": "unknown", "size": "unknown",
            "title_size": 0, "message_size": 999, "duration_seconds": 0,
        },
    }))
    .unwrap();
    settings.normalize_notification_preferences();
    let prefs = settings.notification_preferences.as_ref().unwrap();
    assert_eq!(prefs.title_size, 12);
    assert_eq!(prefs.message_size, 22);
    assert_eq!(prefs.duration_seconds, 3);
    assert!(!prefs.uppercase_title);
    assert!(!prefs.bold_title);
    assert!(!prefs.enabled);
    let prefs_json = serde_json::to_value(prefs).unwrap();
    assert_eq!(prefs_json["position"], "top-right");
    assert_eq!(prefs_json["size"], "compact");
    assert!(settings.dirty);
}

#[test]
fn notification_customization_requires_opt_in_and_preserves_disabled_preferences() {
    let mut settings: SettingsManager = serde_json::from_value(serde_json::json!({
        "notification_preferences": {
            "position": "bottom-left", "title_size": 24, "duration_seconds": 30,
        }
    }))
    .unwrap();
    settings.normalize_notification_preferences();
    let prefs = settings.notification_preferences.as_mut().unwrap();
    assert!(!prefs.enabled);
    prefs.enabled = true;
    let saved = serde_json::to_vec(&settings).unwrap();
    let mut loaded: SettingsManager = serde_json::from_slice(&saved).unwrap();
    loaded.normalize_notification_preferences();
    assert!(loaded.notification_preferences.as_ref().unwrap().enabled);
    loaded.notification_preferences.as_mut().unwrap().enabled = false;
    let saved = serde_json::to_vec(&loaded).unwrap();
    let mut loaded: SettingsManager = serde_json::from_slice(&saved).unwrap();
    loaded.normalize_notification_preferences();
    let prefs = loaded.notification_preferences.unwrap();
    assert!(!prefs.enabled);
    assert_eq!(prefs.title_size, 24);
    assert_eq!(prefs.duration_seconds, 30);
    assert_eq!(
        serde_json::to_value(prefs).unwrap()["position"],
        "bottom-left"
    );
}
