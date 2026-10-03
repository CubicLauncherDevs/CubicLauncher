use super::*;
use crate::types::{AssetMeta, Downloadable, NormalizedArguments};

#[tokio::test]
async fn metadata_is_published_after_success_and_replaces_corrupt_profiles() {
    let root = std::env::temp_dir().join(format!("aqua-publication-{}", Uuid::new_v4()));
    let version = NormalizedVersion {
        id: "1.21".into(),
        parsed_version: zellkern::parse_version("1.21").unwrap(),
        release_time: String::new(),
        java_version: 21,
        main_class: "Main".into(),
        client_jar: Downloadable {
            url: String::new(),
            sha1: String::new(),
            size: 0,
        },
        server_jar: None,
        asset_index: AssetMeta {
            id: "test".into(),
            url: String::new(),
            sha1: String::new(),
            size: 0,
        },
        libraries: vec![],
        natives: vec![],
        arguments: NormalizedArguments {
            game: vec![],
            jvm: vec![],
        },
    };
    let batch = MinecraftBatch {
        dirs: compute_dirs(&root, &version.id, &version.parsed_version),
        version,
        temp_dir: root.join("temp"),
        items: vec![],
        version_json_bytes: br#"{"id":"1.21","mainClass":"Main"}"#.to_vec(),
        asset_index_bytes: br#"{"objects":{}}"#.to_vec(),
    };
    let profile = root.join("versions/1.21/1.21.json");
    batch.prepare().await.unwrap();
    assert!(
        !profile.exists(),
        "preparation must not advertise an installed version"
    );
    tokio::fs::write(&profile, b"corrupt previous attempt")
        .await
        .unwrap();
    batch.finalize(None).await.unwrap();
    assert_eq!(
        tokio::fs::read(&profile).await.unwrap(),
        batch.version_json_bytes
    );
    assert_eq!(
        tokio::fs::read(root.join("assets/indexes/test.json"))
            .await
            .unwrap(),
        batch.asset_index_bytes
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}
