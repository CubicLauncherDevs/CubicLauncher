use super::*;
use std::collections::{HashMap, HashSet};

fn test_client() -> Client {
    Client::builder()
        .user_agent("Cubic Proton/2.0 (tests)")
        .build()
        .expect("Failed to build test reqwest client")
}

async fn fetch_manifest_for_test() -> ManifestV2 {
    fetch_manifest_v2(&test_client()).await.unwrap()
}

async fn fetch_and_parse_version(client: &Client, url: &str) -> VersionManifest {
    let bytes = client.get(url).send().await.unwrap().bytes().await.unwrap();
    VersionManifest::from_bytes(&bytes).unwrap()
}

#[tokio::test]
async fn download_and_parse_manifests_from_all_release_types() {
    let manifest = fetch_manifest_for_test().await;
    let client = test_client();

    let mut by_type: HashMap<String, Vec<&ManifestEntry>> = HashMap::new();
    for entry in &manifest.versions {
        by_type
            .entry(entry.version_type.clone())
            .or_default()
            .push(entry);
    }

    let interesting_types = ["release", "snapshot", "old_alpha", "old_beta"];
    let mut tested: HashSet<&str> = HashSet::new();

    for entry in &manifest.versions {
        if !interesting_types.contains(&entry.version_type.as_str()) {
            continue;
        }
        if tested.contains(entry.version_type.as_str()) {
            continue;
        }

        let version = fetch_and_parse_version(&client, &entry.url).await;

        assert!(
            !version.id_raw.is_empty(),
            "id_raw empty for type {}",
            entry.version_type
        );
        assert!(
            version.main_class.is_some(),
            "main_class missing for {} ({})",
            entry.id,
            entry.version_type
        );
        assert!(
            version.downloads.is_some(),
            "downloads missing for {} ({})",
            entry.id,
            entry.version_type
        );
        if let Some(dl) = &version.downloads {
            assert!(
                !dl.client.url.is_empty(),
                "client url empty for {}",
                entry.id
            );
        }
        assert!(
            version.asset_index.is_some(),
            "asset_index missing for {} ({})",
            entry.id,
            entry.version_type
        );

        tested.insert(entry.version_type.as_str());
    }

    assert_eq!(
        tested.len(),
        4,
        "Not all release types were tested: {:?}",
        tested
    );
}

#[tokio::test]
async fn resolve_normalized_from_each_release_type() {
    let manifest = fetch_manifest_for_test().await;
    let client = test_client();

    let interesting_types = ["release", "snapshot", "old_alpha", "old_beta"];
    let mut tested: HashSet<&str> = HashSet::new();

    for entry in &manifest.versions {
        if !interesting_types.contains(&entry.version_type.as_str()) {
            continue;
        }
        if tested.contains(entry.version_type.as_str()) {
            continue;
        }

        let version = fetch_and_parse_version(&client, &entry.url).await;
        let normalized = resolve_normalized(version).unwrap();

        assert!(!normalized.id.is_empty(), "normalized id empty");
        assert!(!normalized.main_class.is_empty(), "main_class empty");
        assert!(
            normalized.java_version >= 6,
            "java_version too low: {}",
            normalized.java_version
        );

        tested.insert(entry.version_type.as_str());
    }

    assert_eq!(
        tested.len(),
        4,
        "Not all release types were tested: {:?}",
        tested
    );
}

#[test]
fn parses_virtual_and_mapped_asset_indexes() {
    let r#virtual: VersionAssets = serde_json::from_slice(
        br#"{"objects":{"minecraft/sounds/step.ogg":{"hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size":4}},"virtual":true}"#,
    )
    .unwrap();
    assert_eq!(r#virtual.r#virtual, Some(true));
    assert_eq!(r#virtual.map_to_resources, None);
    assert_eq!(r#virtual.len(), 1);

    let mapped: VersionAssets = serde_json::from_slice(
        br#"{"objects":{"sound/step.ogg":{"hash":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","size":4}},"map_to_resources":true}"#,
    )
    .unwrap();
    assert_eq!(mapped.map_to_resources, Some(true));
    assert_eq!(mapped.r#virtual, None);

    let modern: VersionAssets = serde_json::from_slice(br#"{"objects":{}}"#).unwrap();
    assert_eq!(modern.r#virtual, None);
    assert_eq!(modern.map_to_resources, None);
}
