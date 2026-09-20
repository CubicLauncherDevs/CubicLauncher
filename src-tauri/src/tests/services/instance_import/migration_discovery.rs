use super::*;
use serde_json::json;

fn write_json(path: &Path, value: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

fn vanilla(root: &Path) {
    write_json(
        &root.join("versions/1.20.1/1.20.1.json"),
        json!({ "id": "1.20.1", "mainClass": "net.minecraft.client.main.Main" }),
    );
}

#[test]
fn official_profiles_resolve_loaders_custom_directories_and_shared_data() {
    let root = tempfile::tempdir().unwrap();
    vanilla(root.path());
    write_json(
        &root.path().join("versions/custom/custom.json"),
        json!({
            "inheritsFrom": "1.20.1", "libraries": [{ "name": "net.fabricmc:fabric-loader:0.16.10" }]
        }),
    );
    fs::create_dir(root.path().join("custom-game")).unwrap();
    write_json(
        &root.path().join("launcher_profiles.json"),
        json!({ "profiles": {
            "a": { "name": "Vanilla", "lastVersionId": "1.20.1" },
            "b": { "name": "Latest", "lastVersionId": "latest-release" },
            "c": { "name": "Fabric", "lastVersionId": "custom", "gameDir": "custom-game" }
        }}),
    );
    let profiles = official(root.path(), &[("1.20.1".into(), "release".into())]).unwrap();
    assert_eq!(profiles.len(), 3);
    assert!(profiles[0].shared_directory);
    assert!(profiles[1].shared_directory);
    assert!(!profiles[2].shared_directory);
    assert_eq!(profiles[2].source, root.path().join("custom-game"));
    assert_eq!(
        profiles[2].version.as_ref().unwrap().to_version_id(),
        "fabric-loader-0.16.10-1.20.1"
    );
}

#[test]
fn official_bad_profiles_do_not_hide_valid_profiles() {
    let root = tempfile::tempdir().unwrap();
    vanilla(root.path());
    write_json(
        &root.path().join("versions/cycle/cycle.json"),
        json!({ "inheritsFrom": "cycle" }),
    );
    write_json(
        &root.path().join("launcher_profiles.json"),
        json!({ "profiles": {
            "valid": { "lastVersionId": "1.20.1" },
            "cycle": { "lastVersionId": "cycle" },
            "traversal": { "lastVersionId": "../../outside" },
            "missing": { "lastVersionId": "1.20.1", "gameDir": "missing-game" }
        }}),
    );
    let profiles = official(root.path(), &[]).unwrap();
    assert_eq!(profiles.iter().filter(|p| p.error.is_none()).count(), 1);
    assert_eq!(
        profiles.iter().find(|p| p.error.is_none()).unwrap().name,
        "valid"
    );
}

#[test]
fn official_forge_library_is_normalized_and_unknown_custom_versions_are_blocked() {
    let root = tempfile::tempdir().unwrap();
    vanilla(root.path());
    write_json(
        &root.path().join("versions/forge/forge.json"),
        json!({
            "inheritsFrom": "1.20.1", "libraries": [{ "name": "net.minecraftforge:forge:1.20.1-47.3.0" }]
        }),
    );
    let version = resolve_official(root.path(), "forge", &[], &mut HashSet::new()).unwrap();
    assert_eq!(version.to_version_id(), "1.20.1-forge-47.3.0");
    write_json(
        &root.path().join("versions/custom/custom.json"),
        json!({ "inheritsFrom": "1.20.1", "mainClass": "example.Custom" }),
    );
    assert!(resolve_official(root.path(), "custom", &[], &mut HashSet::new()).is_err());
}

#[test]
fn multimc_forks_accept_both_game_folders_and_only_explicit_memory_overrides() {
    let root = tempfile::tempdir().unwrap();
    for (name, folder, override_memory) in [
        ("Prism", ".minecraft", "true"),
        ("PolyMC", "minecraft", "false"),
    ] {
        let dir = root.path().join("instances").join(name);
        fs::create_dir_all(dir.join(folder)).unwrap();
        fs::write(dir.join("instance.cfg"), format!("[General]\nname={name}\nOverrideMemory={override_memory}\nMinMemAlloc=1024\nMaxMemAlloc=4096\n")).unwrap();
        write_json(
            &dir.join("mmc-pack.json"),
            json!({ "components": [
                { "uid": "net.minecraft", "version": "1.21.1" },
                { "uid": "net.neoforged", "cachedVersion": "21.1.0" }
            ]}),
        );
    }
    let candidates = multimc(root.path()).unwrap();
    assert_eq!(candidates.len(), 2);
    let prism = candidates.iter().find(|c| c.name == "Prism").unwrap();
    assert_eq!(prism.memory, Some((1024, 4096)));
    assert_eq!(
        prism.version.as_ref().unwrap().to_version_id(),
        "1.21.1-neoforge-21.1.0"
    );
    assert_eq!(
        candidates
            .iter()
            .find(|c| c.name == "PolyMC")
            .unwrap()
            .memory,
        None
    );
    assert_eq!(
        multimc(&root.path().join("instances/Prism")).unwrap().len(),
        1
    );
    assert_eq!(multimc(&root.path().join("instances")).unwrap().len(), 2);
}

#[test]
fn malformed_multimc_instance_does_not_abort_other_instances() {
    let root = tempfile::tempdir().unwrap();
    let bad = root.path().join("bad");
    fs::create_dir(&bad).unwrap();
    fs::write(bad.join("instance.cfg"), [0xff, 0xfe]).unwrap();
    let entries = multimc(root.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].error.is_some());
}

#[test]
fn multimc_unsupported_or_incomplete_loaders_are_not_silently_imported_as_vanilla() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("instance.cfg"), "name=Modded").unwrap();
    for component in [
        json!({ "uid": "net.fabricmc.fabric-loader" }),
        json!({ "uid": "custom.loader", "version": "1" }),
    ] {
        write_json(
            &root.path().join("mmc-pack.json"),
            json!({ "components": [
                { "uid": "net.minecraft", "version": "1.20.1" }, component
            ]}),
        );
        assert!(multimc(root.path()).unwrap()[0].error.is_some());
    }
}

#[test]
fn prism_and_polymc_fabric_include_intermediary_and_keep_loader_version() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("instance.cfg"), "name=Fabric").unwrap();
    // Fabric's Prism metadata requires net.fabricmc.intermediary as a separate component.
    write_json(
        &root.path().join("mmc-pack.json"),
        json!({ "components": [
            { "uid": "net.minecraft", "version": "1.21.1" },
            { "uid": "org.lwjgl3", "version": "3.3.3" },
            { "uid": "net.fabricmc.intermediary", "version": "1.21.1" },
            { "uid": "net.fabricmc.fabric-loader", "cachedVersion": "0.16.10" }
        ]}),
    );
    let candidates = multimc(root.path()).unwrap();
    assert!(candidates[0].error.is_none(), "{:?}", candidates[0].error);
    let version = candidates[0].version.as_ref().unwrap();
    assert_eq!(version.to_version_id(), "fabric-loader-0.16.10-1.21.1");
    assert_eq!(
        GameVersion::from_version_id(&version.to_version_id()),
        *version
    );
}

#[test]
fn known_fabric_and_neoforge_patches_resolve_versions_with_reinstall_notice() {
    for (uid, loader_version, expected) in [
        (
            "net.fabricmc.fabric-loader",
            "0.16.10",
            "fabric-loader-0.16.10-1.21.1",
        ),
        ("net.neoforged", "21.1.172", "1.21.1-neoforge-21.1.172"),
    ] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("instance.cfg"), "name=Modded").unwrap();
        write_json(
            &root.path().join("mmc-pack.json"),
            json!({ "components": [
                { "uid": "net.minecraft", "version": "1.21.1" }, { "uid": uid }
            ]}),
        );
        write_json(
            &root.path().join(format!("patches/{uid}.json")),
            json!({
                "uid": uid, "version": loader_version, "formatVersion": 1,
                "libraries": [], "requires": [{ "uid": "net.minecraft", "equals": "1.21.1" }]
            }),
        );
        let candidates = multimc(root.path()).unwrap();
        assert!(candidates[0].error.is_none(), "{:?}", candidates[0].error);
        assert!(candidates[0].reinstalls_components);
        let version = candidates[0].version.as_ref().unwrap();
        assert_eq!(version.to_version_id(), expected);
        assert_eq!(GameVersion::from_version_id(expected), *version);
        write_json(
            &root.path().join(format!("patches/{uid}.json")),
            json!({
                "uid": uid, "version": loader_version,
                "requires": [{ "uid": "net.minecraft", "equals": "1.20.1" }]
            }),
        );
        assert!(multimc(root.path()).unwrap()[0].error.is_some());
    }
}

#[test]
fn unknown_component_patches_still_block_migration() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("instance.cfg"), "name=Custom").unwrap();
    write_json(
        &root.path().join("mmc-pack.json"),
        json!({ "components": [
            { "uid": "net.minecraft", "version": "1.21.1" }
        ]}),
    );
    write_json(
        &root.path().join("patches/custom.json"),
        json!({ "uid": "custom.loader", "version": "1" }),
    );
    assert!(multimc(root.path()).unwrap()[0].error.is_some());
}

#[test]
fn official_neoforge_installer_arguments_work_without_neoforge_library() {
    let root = tempfile::tempdir().unwrap();
    // Reduced from version.json in NeoForge's official 21.1.172 installer.
    // The FML library version (4.0.39) is NOT the NeoForge version.
    write_json(
        &root
            .path()
            .join("versions/neoforge-21.1.172/neoforge-21.1.172.json"),
        json!({
            "id": "neoforge-21.1.172", "inheritsFrom": "1.21.1",
            "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments": { "game": ["--fml.neoForgeVersion", "21.1.172", "--fml.fmlVersion", "4.0.39", "--fml.mcVersion", "1.21.1", "--launchTarget", "forgeclient"] },
            "libraries": [{ "name": "net.neoforged.fancymodloader:loader:4.0.39" }, { "name": "net.fabricmc:sponge-mixin:0.15.2+mixin.0.8.7" }]
        }),
    );
    write_json(
        &root.path().join("launcher_profiles.json"),
        json!({ "profiles": {
            "neo": { "name": "NeoForge", "lastVersionId": "neoforge-21.1.172" }
        }}),
    );
    // No installed vanilla parent and no network catalog, as after running only the installer.
    let candidates = official(root.path(), &[]).unwrap();
    assert!(candidates[0].error.is_none(), "{:?}", candidates[0].error);
    assert_eq!(
        candidates[0].version.as_ref().unwrap().to_version_id(),
        "1.21.1-neoforge-21.1.172"
    );
}

#[test]
fn official_fabric_profiles_from_both_files_work_before_first_vanilla_launch() {
    let root = tempfile::tempdir().unwrap();
    // Reduced from meta.fabricmc.net/v2/versions/loader/1.21.1/0.16.10/profile/json.
    write_json(
        &root
            .path()
            .join("versions/fabric-loader-0.16.10-1.21.1/fabric-loader-0.16.10-1.21.1.json"),
        json!({
            "inheritsFrom": "1.21.1", "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [{ "name": "net.fabricmc:intermediary:1.21.1" }, { "name": "net.fabricmc:fabric-loader:0.16.10" }]
        }),
    );
    let profile = json!({ "name": "Fabric", "lastVersionId": "fabric-loader-0.16.10-1.21.1" });
    write_json(
        &root.path().join("launcher_profiles.json"),
        json!({ "profiles": { "fabric": profile } }),
    );
    write_json(
        &root.path().join("launcher_profiles_microsoft_store.json"),
        json!({ "profiles": {
            "fabric": profile, "second": { "name": "Fabric Store", "lastVersionId": "fabric-loader-0.16.10-1.21.1" }
        }}),
    );
    let candidates = official(root.path(), &[]).unwrap();
    assert_eq!(candidates.len(), 2);
    for candidate in candidates {
        assert!(candidate.error.is_none(), "{:?}", candidate.error);
        assert_eq!(
            candidate.version.unwrap().to_version_id(),
            "fabric-loader-0.16.10-1.21.1"
        );
    }
}

#[test]
fn official_loader_version_conflicts_and_unknown_parent_are_not_guessed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("versions/neo/neo.json");
    write_json(
        &path,
        json!({ "inheritsFrom": "1.21.1",
            "arguments": { "game": ["--fml.neoForgeVersion", "21.1.172", "--fml.mcVersion", "1.21.1"] },
            "libraries": [{ "name": "net.neoforged:neoforge:21.1.1" }]
        }),
    );
    assert!(resolve_official(root.path(), "neo", &[], &mut HashSet::new()).is_err());
    write_json(
        &path,
        json!({ "inheritsFrom": "missing-parent",
            "arguments": { "game": ["--fml.neoForgeVersion", "21.1.172"] }
        }),
    );
    assert!(resolve_official(root.path(), "neo", &[], &mut HashSet::new()).is_err());
}
