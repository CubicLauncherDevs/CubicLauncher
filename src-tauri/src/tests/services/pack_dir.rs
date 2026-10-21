use super::*;

#[test]
fn versions_before_1_6_use_texturepacks() {
    for version in [
        "a1.2.6",
        "b1.7.3",
        "b1.8.1",
        "c0.30_01c",
        "rd-132211",
        "inf-20100618",
        "1.0",
        "1.2.5",
        "1.5.2",
        "1.5.2-forge-7.8.1.738",
        "b1.7.3-OptiFine_HD_G",
        "11w49a",
        "12w50a",
        "13w22a",
    ] {
        assert!(uses_texture_packs(version), "{version} debería ser legacy");
    }
}

#[test]
fn versions_from_1_6_use_resourcepacks() {
    for version in [
        "1.6",
        "1.6.1",
        "1.6.4",
        "1.7.10",
        "1.7.10-forge-10.13.4.1614",
        "1.12.2",
        "1.21.4",
        "26.2-snapshot-5",
        "13w23a",
        "13w23b",
        "14w02a",
        "fabric-loader-0.15.11-1.20.1",
        "custom-profile",
        "",
    ] {
        assert!(
            !uses_texture_packs(version),
            "{version} no debería ser legacy"
        );
    }
}

#[test]
fn physical_pack_dir_only_remaps_resourcepacks() {
    assert_eq!(physical_pack_dir("b1.7.3", "resourcepacks"), "texturepacks");
    assert_eq!(
        physical_pack_dir("1.20.1", "resourcepacks"),
        "resourcepacks"
    );
    assert_eq!(physical_pack_dir("b1.7.3", "shaderpacks"), "shaderpacks");
    assert_eq!(physical_pack_dir("b1.7.3", "mods"), "mods");
}

#[tokio::test]
async fn migrates_the_whole_directory_when_target_is_missing() {
    let temp = tempfile::tempdir().unwrap();
    let instance = temp.path().join("instance");
    std::fs::create_dir_all(instance.join("resourcepacks")).unwrap();
    std::fs::write(instance.join("resourcepacks/a.zip"), b"pack").unwrap();

    migrate_legacy_packs(&instance, "b1.7.3").await.unwrap();

    assert!(!instance.join("resourcepacks").exists());
    assert_eq!(
        std::fs::read(instance.join("texturepacks/a.zip")).unwrap(),
        b"pack"
    );
}

#[tokio::test]
async fn merges_without_overwriting_existing_packs() {
    let temp = tempfile::tempdir().unwrap();
    let instance = temp.path().join("instance");
    std::fs::create_dir_all(instance.join("resourcepacks")).unwrap();
    std::fs::create_dir_all(instance.join("texturepacks")).unwrap();
    std::fs::write(instance.join("resourcepacks/shared.zip"), b"src").unwrap();
    std::fs::write(instance.join("resourcepacks/extra.zip"), b"extra").unwrap();
    std::fs::write(instance.join("texturepacks/shared.zip"), b"dst").unwrap();

    migrate_legacy_packs(&instance, "b1.7.3").await.unwrap();

    assert_eq!(
        std::fs::read(instance.join("texturepacks/shared.zip")).unwrap(),
        b"dst"
    );
    assert_eq!(
        std::fs::read(instance.join("texturepacks/extra.zip")).unwrap(),
        b"extra"
    );
    assert_eq!(
        std::fs::read(instance.join("resourcepacks/shared.zip")).unwrap(),
        b"src"
    );
}

#[tokio::test]
async fn modern_versions_are_left_untouched() {
    let temp = tempfile::tempdir().unwrap();
    let instance = temp.path().join("instance");
    std::fs::create_dir_all(instance.join("resourcepacks")).unwrap();
    std::fs::write(instance.join("resourcepacks/a.zip"), b"pack").unwrap();

    migrate_legacy_packs(&instance, "1.20.1").await.unwrap();

    assert!(instance.join("resourcepacks/a.zip").exists());
    assert!(!instance.join("texturepacks").exists());
}

#[tokio::test]
async fn missing_source_is_not_an_error() {
    let temp = tempfile::tempdir().unwrap();
    let instance = temp.path().join("instance");
    std::fs::create_dir_all(&instance).unwrap();

    migrate_legacy_packs(&instance, "b1.7.3").await.unwrap();
}
