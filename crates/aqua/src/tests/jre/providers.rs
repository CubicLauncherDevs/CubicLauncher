use super::*;

#[test]
fn test_vendor_ids_round_trip() {
    for vendor in JreVendor::ALL {
        assert_eq!(JreVendor::from_id(vendor.id()), Some(*vendor));
    }

    assert_eq!(JreVendor::from_id(" Temurin "), Some(JreVendor::Temurin));
    assert_eq!(JreVendor::from_id("GRAALVM"), Some(JreVendor::GraalVm));
    assert_eq!(JreVendor::from_id("adoptium"), None);
    assert_eq!(JreVendor::from_id(""), None);
}

#[test]
fn test_vendor_ids_and_names_are_unique() {
    let mut ids: Vec<&str> = JreVendor::ALL.iter().map(JreVendor::id).collect();
    let mut names: Vec<&str> = JreVendor::ALL.iter().map(JreVendor::name).collect();

    ids.sort_unstable();
    ids.dedup();
    names.sort_unstable();
    names.dedup();

    assert_eq!(ids.len(), JreVendor::ALL.len());
    assert_eq!(names.len(), JreVendor::ALL.len());
}

#[test]
fn test_default_chain_is_listed() {
    for vendor in JreProviderChain::FALLBACKS {
        assert!(JreVendor::ALL.contains(vendor));
    }
}

#[test]
fn test_foojay_slug_covers_extra_vendors() {
    for vendor in JreVendor::ALL {
        let uses_own_api = matches!(vendor, JreVendor::Zulu | JreVendor::Temurin);
        assert_eq!(vendor.foojay_slug().is_none(), uses_own_api);
    }

    assert_eq!(JreVendor::Liberica.foojay_slug(), Some("liberica"));
    assert_eq!(JreVendor::Semeru.foojay_slug(), Some("semeru"));
}

#[test]
fn test_foojay_package_type_avoids_missing_jre() {
    // Estos proveedores solo publican JDK.
    assert_eq!(JreVendor::GraalVm.foojay_package_type(), "jdk");
    assert_eq!(JreVendor::Corretto.foojay_package_type(), "jdk");
    assert_eq!(JreVendor::Microsoft.foojay_package_type(), "jdk");
    assert_eq!(JreVendor::Liberica.foojay_package_type(), "jre");
}

#[test]
fn test_foojay_packages_url_includes_filters() {
    let url = foojay_packages_url(JreVendor::GraalVm, 21);

    for expected in [
        "version=21",
        "distribution=graalvm",
        "package_type=jdk",
        "architecture=",
        "latest=available",
        "directly_downloadable=true",
        "release_status=ga",
    ] {
        assert!(url.contains(expected), "missing {expected} in {url}");
    }

    let url = foojay_packages_url(JreVendor::Semeru, 8);
    assert!(url.contains("distribution=semeru"));
    assert!(url.contains("package_type=jre"));
}

#[test]
fn test_select_foojay_package_prefers_glibc() {
    let package = |filename: &str, lib_c_type: Option<&str>| FoojayPackage {
        java_version: "21.0.12.1+1".into(),
        archive_type: "tar.gz".into(),
        lib_c_type: lib_c_type.map(str::to_string),
        filename: filename.into(),
        size: None,
        links: FoojayLinks {
            pkg_info_uri: "https://api.foojay.io/disco/v3.0/ids/example".into(),
            pkg_download_redirect: "https://api.foojay.io/disco/v3.0/ids/example/redirect".into(),
        },
    };

    let selected = select_foojay_package(vec![
        package("musl.tar.gz", Some("musl")),
        package("glibc.tar.gz", Some("glibc")),
    ])
    .unwrap();
    assert_eq!(selected.filename, "glibc.tar.gz");

    // Sin alternativas glibc se conserva el build musl.
    let selected = select_foojay_package(vec![package("musl.tar.gz", Some("musl"))]).unwrap();
    assert_eq!(selected.filename, "musl.tar.gz");

    assert!(select_foojay_package(Vec::new()).is_none());
}

#[test]
fn test_archive_format_from_foojay() {
    assert_eq!(archive_format("zip"), ArchiveFormat::Zip);
    assert_eq!(archive_format("tar.gz"), ArchiveFormat::TarGz);
}

#[test]
fn test_detect_vendor_from_java_version_output() {
    let cases = [
        (
            "OpenJDK Runtime Environment Temurin-21.0.2+13 (build 21.0.2+13)",
            JreVendor::Temurin,
        ),
        (
            "OpenJDK Runtime Environment Zulu21.32+17-CA (build 21.0.2+13)",
            JreVendor::Zulu,
        ),
        (
            "OpenJDK Runtime Environment GraalVM CE 21.0.2+13.1 (build 21.0.2+13.1)",
            JreVendor::GraalVm,
        ),
        (
            "OpenJDK Runtime Environment Corretto-21.0.2.13.1 (build 21.0.2+13)",
            JreVendor::Corretto,
        ),
        (
            "OpenJDK Runtime Environment Microsoft-10345742 (build 21.0.2+13)",
            JreVendor::Microsoft,
        ),
        (
            "IBM Semeru Runtime Open Edition (build 21.0.12.0)",
            JreVendor::Semeru,
        ),
        ("Eclipse OpenJ9 VM (build 21.0.12)", JreVendor::Semeru),
        (
            "BellSoft Liberica Runtime Environment 21.0.12 (build 21.0.12+12)",
            JreVendor::Liberica,
        ),
    ];

    for (output, expected) in cases {
        assert_eq!(
            JreVendor::detect(output),
            Some(expected),
            "output: {output}"
        );
    }

    assert_eq!(
        JreVendor::detect("openjdk version \"21.0.12\" 2026-07-21"),
        None
    );
}
