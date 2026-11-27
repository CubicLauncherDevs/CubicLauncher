use super::*;

#[test]
fn parses_standard() {
    assert_eq!(parse_version("1.21").unwrap(), MCVersion::new(1, 21, None));
    assert_eq!(
        parse_version("1.20.6").unwrap(),
        MCVersion::new(1, 20, Some(6))
    );
}

#[test]
fn parses_snapshot() {
    let v = parse_version("26.2-snapshot-5").unwrap();
    assert_eq!(v, MCVersion::new(26, 2, None));
}

#[test]
fn extract_fabric_version() {
    let gv = GameVersion::from_version_id("fabric-loader-0.15.11-1.20.1");
    assert_eq!(gv.mc_version, "1.20.1");
    assert_eq!(gv.loader, Loader::Fabric("0.15.11".into()));
}

#[test]
fn extract_forge_version() {
    let gv = GameVersion::from_version_id("1.20.1-forge-47.2.0");
    assert_eq!(gv.mc_version, "1.20.1");
}

#[test]
fn extract_neoforge_version() {
    let gv = GameVersion::from_version_id("1.21-neoforge-21.0.0");
    assert_eq!(gv.mc_version, "1.21");
}

#[test]
fn extract_quilt_version() {
    let gv = GameVersion::from_version_id("quilt-loader-0.25.0-1.20.1");
    assert_eq!(gv.mc_version, "1.20.1");
    assert_eq!(gv.loader, Loader::Quilt("0.25.0".into()));
}

#[test]
fn roundtrip_fabric() {
    let original = "fabric-loader-0.15.11-1.20.1";
    let gv = GameVersion::from_version_id(original);
    assert_eq!(gv.to_version_id(), original);
}

#[test]
fn roundtrip_forge() {
    let original = "1.20.1-forge-47.2.0";
    let gv = GameVersion::from_version_id(original);
    assert_eq!(gv.to_version_id(), original);
}

#[test]
fn dependencies_vanilla() {
    let deps = resolve_dependencies("1.20.1");
    assert_eq!(deps, vec!["1.20.1"]);
}

#[test]
fn dependencies_forge() {
    let deps = resolve_dependencies("1.20.1-forge-47.2.0");
    assert_eq!(deps, vec!["1.20.1", "1.20.1-forge-47.2.0"]);
}

#[test]
fn dependencies_fabric() {
    let deps = resolve_dependencies("fabric-loader-0.15.11-1.20.1");
    assert_eq!(deps, vec!["1.20.1", "fabric-loader-0.15.11-1.20.1"]);
}

#[test]
fn extract_fabric_snapshot_version() {
    let gv = GameVersion::from_version_id("fabric-loader-0.19.1-26.3-snapshot-2");
    assert_eq!(gv.mc_version, "26.3-snapshot-2");
    assert_eq!(gv.loader, Loader::Fabric("0.19.1".into()));
}

#[test]
fn extract_quilt_snapshot_version() {
    let gv = GameVersion::from_version_id("quilt-loader-0.25.0-26.3-snapshot-2");
    assert_eq!(gv.mc_version, "26.3-snapshot-2");
    assert_eq!(gv.loader, Loader::Quilt("0.25.0".into()));
}

#[test]
fn dependencies_fabric_snapshot() {
    let deps = resolve_dependencies("fabric-loader-0.19.1-26.3-snapshot-2");
    assert_eq!(
        deps,
        vec!["26.3-snapshot-2", "fabric-loader-0.19.1-26.3-snapshot-2"]
    );
}

#[test]
fn dependencies_neoforge() {
    let deps = resolve_dependencies("1.21-neoforge-21.0.0");
    assert_eq!(deps, vec!["1.21", "1.21-neoforge-21.0.0"]);
}

#[test]
fn dependencies_quilt() {
    let deps = resolve_dependencies("quilt-loader-0.25.0-1.20.1");
    assert_eq!(deps, vec!["1.20.1", "quilt-loader-0.25.0-1.20.1"]);
}

#[test]
fn optifine_version_roundtrip_and_dependencies() {
    for id in ["1.12.2-OptiFine_HD_U_G5", "1.21-OptiFine_HD_U_J1_pre9"] {
        let version = crate::GameVersion::from_version_id(id);
        assert!(matches!(version.loader, crate::Loader::OptiFine(_)));
        assert_eq!(version.to_version_id(), id);
        assert_eq!(
            crate::resolve_dependencies(id),
            vec![version.mc_version, id.into()]
        );
    }
}

#[test]
fn pre_1_6_versions() {
    for version in [
        "a1.2.6",
        "b1.7.3",
        "b1.8.1",
        "c0.30_01c",
        "rd-132211",
        "inf-20100618",
        "1.0",
        "1.5.2",
        "1.5.2-forge-7.8.1.738",
        "b1.7.3-OptiFine_HD_G",
        "fabric-loader-0.15.11-a1.2.6",
        "11w49a",
        "12w50a",
        "13w22a",
    ] {
        assert!(crate::is_pre_1_6(version), "{version}");
    }
}

#[test]
fn versions_since_1_6_are_not_pre_1_6() {
    for version in [
        "1.6",
        "1.6.4",
        "1.7.10",
        "1.7.10-forge-10.13.4.1614",
        "1.21.4",
        "26.2-snapshot-5",
        "13w23a",
        "13w41b",
        "14w02a",
        "fabric-loader-0.15.11-1.20.1",
        "custom-profile",
        "beta1.7.3",
        "1.7.2",
        "",
    ] {
        assert!(!crate::is_pre_1_6(version), "{version}");
    }
}
