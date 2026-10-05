use super::*;

fn config() -> serde_json::Value {
    serde_json::json!({
        "pubkey": "test-public-key", "windows": { "installMode": "passive" },
        "endpoints": ["https://updates.example.test/", "https://github.com/example/launcher/releases/latest/download/latest.json"]
    })
}

#[test]
fn channel_uses_the_supplied_settings_path_and_preserves_fallback_and_signature_key() {
    let temp = tempfile::tempdir().unwrap();
    let settings = temp.path().join("settings.cub");
    std::fs::write(&settings, r#"{"update_channel":"prerelease"}"#).unwrap();
    let mut value = config();
    configure(&mut value, &settings, None).unwrap();
    assert_eq!(
        value["endpoints"][0],
        "https://updates.example.test/?channel=prerelease"
    );
    assert_eq!(value["endpoints"][1], config()["endpoints"][1]);
    assert_eq!(value["pubkey"], config()["pubkey"]);
    assert_eq!(value["windows"], config()["windows"]);
}

#[test]
fn stable_missing_corrupt_and_unknown_settings_keep_stable_fallbacks() {
    let temp = tempfile::tempdir().unwrap();
    let settings = temp.path().join("settings.cub");
    let mut value = config();
    configure(&mut value, &settings, None).unwrap();
    assert_eq!(value, config());
    for content in [
        "bad json",
        "{}",
        r#"{"update_channel":"stable"}"#,
        r#"{"update_channel":"unknown"}"#,
    ] {
        std::fs::write(&settings, content).unwrap();
        let mut value = config();
        configure(&mut value, &settings, None).unwrap();
        assert_eq!(value, config());
    }
}

#[test]
fn test_override_preserves_query_parameters_without_production_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let settings = temp.path().join("settings.cub");
    std::fs::write(&settings, r#"{"update_channel":"prerelease"}"#).unwrap();
    let mut value = config();
    configure(
        &mut value,
        &settings,
        Some("http://localhost:8080/update?fixture=1&channel=stable"),
    )
    .unwrap();
    assert_eq!(
        value["endpoints"],
        serde_json::json!(["http://localhost:8080/update?fixture=1&channel=prerelease"])
    );
    let previous = value.clone();
    assert!(configure(&mut value, &settings, Some("not a URL")).is_err());
    assert_eq!(value, previous);
}
