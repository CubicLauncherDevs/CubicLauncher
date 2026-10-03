use super::*;

fn profile(shared: &Path, version: &str, extra: serde_json::Value) {
    let dir = version_dir(shared, version);
    std::fs::create_dir_all(&dir).unwrap();
    let mut json = serde_json::json!({"id": version, "mainClass": "Main"});
    json.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    std::fs::write(
        dir.join(format!("{version}.json")),
        serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn metadata_and_failed_retries_are_not_completed_installations() {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path();
    profile(shared, "1.21", serde_json::json!({}));
    assert!(!is_complete(shared, "1.21"));
    std::fs::write(version_dir(shared, "1.21").join("1.21.jar"), b"client").unwrap();
    assert!(is_complete(shared, "1.21"), "legacy offline installation");
    begin(shared, "1.21").await.unwrap();
    assert!(!is_complete(shared, "1.21"), "interrupted installation");
    finish(shared, "1.21").await.unwrap();
    assert!(is_complete(shared, "1.21"));
    begin(shared, "1.21").await.unwrap();
    assert!(
        !is_complete(shared, "1.21"),
        "old receipt must not hide a failed repair"
    );
    finish(shared, "1.21").await.unwrap();
    assert!(is_complete(shared, "1.21"));
}

#[tokio::test]
async fn a_completed_loader_still_requires_its_base_version() {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path();
    let loader = "fabric-loader-0.16.0-1.21";
    profile(shared, loader, serde_json::json!({"inheritsFrom": "1.21"}));
    begin(shared, loader).await.unwrap();
    finish(shared, loader).await.unwrap();
    assert!(is_complete(shared, loader));
    assert_eq!(missing_dependencies(shared, loader), vec!["1.21"]);
    profile(shared, "1.21", serde_json::json!({}));
    std::fs::write(version_dir(shared, "1.21").join("1.21.jar"), b"client").unwrap();
    assert!(missing_dependencies(shared, loader).is_empty());
}

#[test]
fn legacy_profiles_require_runtime_libraries_and_assets() {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path();
    profile(
        shared,
        "1.21",
        serde_json::json!({
            "libraries": [{"name": "test:lib:1", "downloads": {"artifact": {"path": "test.jar", "size": 3}}}],
            "assetIndex": {"id": "test"}
        }),
    );
    std::fs::write(version_dir(shared, "1.21").join("1.21.jar"), b"client").unwrap();
    assert!(!is_complete(shared, "1.21"));
    std::fs::create_dir_all(shared.join("libraries")).unwrap();
    std::fs::write(shared.join("libraries/test.jar"), b"jar").unwrap();
    assert!(!is_complete(shared, "1.21"));
    std::fs::create_dir_all(shared.join("assets/indexes")).unwrap();
    std::fs::write(
        shared.join("assets/indexes/test.json"),
        br#"{"objects":{}}"#,
    )
    .unwrap();
    assert!(is_complete(shared, "1.21"));
    std::fs::write(shared.join("libraries/test.jar"), b"x").unwrap();
    assert!(!is_complete(shared, "1.21"));
}
