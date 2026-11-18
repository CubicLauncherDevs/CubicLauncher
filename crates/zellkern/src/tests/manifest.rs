use super::*;

#[test]
fn resolved_manifest_preserves_overrides_and_argument_order() {
    let parent = VersionManifest::from_bytes(
        br#"{
        "id":"1.21", "mainClass":"parent.Main",
        "minecraftArguments":"parent arguments",
        "arguments":{"jvm":["-Dparent=true"],"game":["--parent"]},
        "libraries":[{"name":"example:parent:1"}],
        "javaVersion":{"component":"java-runtime-delta","majorVersion":21}
    }"#,
    )
    .unwrap();
    let child = VersionManifest::from_bytes(
        br#"{
        "id":"1.21", "mainClass":"child.Main", "inheritsFrom":"1.21",
        "arguments":{"jvm":["-Dchild=true"],"game":["--child"]},
        "libraries":[{"name":"example:child:1"}]
    }"#,
    )
    .unwrap();

    let resolved = child.resolve(&parent);
    assert_eq!(resolved.main_class.as_deref(), Some("child.Main"));
    assert_eq!(
        resolved.minecraft_arguments.as_deref(),
        Some("parent arguments")
    );
    assert_eq!(resolved.java_major_version(), 21);
    assert_eq!(
        serde_json::to_value(resolved.arguments.unwrap()).unwrap(),
        serde_json::json!({
            "jvm":["-Dparent=true", "-Dchild=true"],
            "game":["--parent", "--child"],
            "defaultUserJvm": null
        })
    );
    let libraries = resolved.libraries.unwrap();
    assert_eq!(libraries[0].name, "example:parent:1");
    assert_eq!(libraries[1].name, "example:child:1");
    assert_eq!(parent.libraries.unwrap().len(), 1);
    assert_eq!(child.libraries.unwrap().len(), 1);
}

#[test]
fn rule_without_os_allows() {
    let r: Rule = serde_json::from_str(r#"{"action":"allow"}"#).unwrap();
    assert!(r.action_if_matches().is_some());
}

#[test]
fn native_detection_by_path() {
    let lib: Library = serde_json::from_str(
        r#"{
            "name": "org.lwjgl:lwjgl:3.4.1:natives-linux",
            "downloads": {
                "artifact": {
                    "path": "org/lwjgl/lwjgl/3.4.1/lwjgl-3.4.1-natives-linux.jar",
                    "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl/3.4.1/lwjgl-3.4.1-natives-linux.jar",
                    "sha1": "abc123",
                    "size": 12345
                }
            },
            "rules": [{"action":"allow","os":{"name":"linux"}}]
        }"#,
    )
    .unwrap();
    assert!(lib.is_native());
}

#[test]
#[cfg(target_os = "linux")]
fn allow_linux() {
    let r: Rule = serde_json::from_str(r#"{"action":"allow","os":{"name":"linux"}}"#).unwrap();
    assert!(r.evaluate());
}

#[test]
#[cfg(not(target_os = "linux"))]
fn disallow_linux_on_non_linux() {
    let r: Rule = serde_json::from_str(r#"{"action":"allow","os":{"name":"linux"}}"#).unwrap();
    assert!(!r.evaluate());
}
