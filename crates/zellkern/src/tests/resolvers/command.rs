use super::*;

#[test]
fn custom_minecraft_jar_replaces_original_classpath_and_is_required() {
    let dir = tempfile::tempdir().unwrap();
    let original_dir = dir.path().join("versions").join("1.21.1");
    std::fs::create_dir_all(&original_dir).unwrap();
    let original = original_dir.join("1.21.1.jar");
    std::fs::write(&original, b"original").unwrap();
    let custom = dir.path().join("custom.jar");
    std::fs::write(&custom, b"custom").unwrap();
    let manifest = VersionManifest::from_bytes(
        br#"{"id":"1.21.1","mainClass":"net.minecraft.client.main.Main"}"#,
    )
    .unwrap();
    let mut config = LaunchConfig::builder()
        .java_path(std::env::current_exe().unwrap())
        .build();
    config.minecraft_jar = Some(custom.clone());
    let args = CommandBuilder::new(&manifest, dir.path(), dir.path(), &config)
        .build()
        .unwrap();
    let cp = args.windows(2).find(|a| a[0] == "-cp").unwrap();
    assert_eq!(cp[1], custom.to_string_lossy());
    assert!(!args.iter().any(|a| a.contains(original.to_str().unwrap())));
    // A full replacement must also work without the original client JAR.
    std::fs::remove_file(original).unwrap();
    assert!(
        CommandBuilder::new(&manifest, dir.path(), dir.path(), &config)
            .build()
            .is_ok()
    );
    std::fs::remove_file(custom).unwrap();
    assert!(
        CommandBuilder::new(&manifest, dir.path(), dir.path(), &config)
            .build()
            .is_err()
    );
}

#[test]
fn custom_jar_uses_inherited_base_and_keeps_loader_libraries() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("versions").join("1.21.1");
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(
        base.join("1.21.1.json"),
        br#"{"id":"1.21.1","mainClass":"net.minecraft.client.main.Main"}"#,
    )
    .unwrap();
    let library = dir
        .path()
        .join("libraries")
        .join("example")
        .join("loader")
        .join("1")
        .join("loader-1.jar");
    std::fs::create_dir_all(library.parent().unwrap()).unwrap();
    std::fs::write(&library, b"loader").unwrap();
    let custom = dir.path().join("custom.jar");
    std::fs::write(&custom, b"custom").unwrap();
    let manifest = VersionManifest::from_bytes(br#"{"id":"fabric-loader-0.16.0-1.21.1","inheritsFrom":"1.21.1","mainClass":"example.Loader","libraries":[{"name":"example:loader:1"}]}"#).unwrap();
    let mut config = LaunchConfig::builder()
        .java_path(std::env::current_exe().unwrap())
        .build();
    config.minecraft_jar = Some(custom.clone());
    let builder = CommandBuilder::new(&manifest, dir.path(), dir.path(), &config);
    assert_eq!(
        builder.original_minecraft_jar().unwrap(),
        base.join("1.21.1.jar")
    );
    let args = builder.build().unwrap();
    let cp = args.windows(2).find(|a| a[0] == "-cp").unwrap();
    assert!(cp[1].contains(library.to_str().unwrap()));
    assert!(cp[1].contains(custom.to_str().unwrap()));
    assert!(args.contains(&"example.Loader".to_string()));
}

#[test]
fn modlauncher_rejects_custom_client_instead_of_ignoring_it() {
    let manifest = VersionManifest::from_bytes(
        br#"{"id":"1.20.1-forge-47.0.0","libraries":[{"name":"cpw.mods:modlauncher:10.0.9"}]}"#,
    )
    .unwrap();
    assert!(CommandBuilder::verify_jar_override_support(&manifest).is_err());
    let legacy = VersionManifest::from_bytes(
        br#"{"id":"1.12.2-forge-14.23.5.2860","mainClass":"net.minecraft.launchwrapper.Launch"}"#,
    )
    .unwrap();
    assert!(CommandBuilder::verify_jar_override_support(&legacy).is_ok());
}

fn launch_uuid(config: &LaunchConfig, modern: bool) -> String {
    let dir = tempfile::tempdir().unwrap();
    let version_dir = dir.path().join("versions").join("1.21.1");
    std::fs::create_dir_all(&version_dir).unwrap();
    std::fs::write(version_dir.join("1.21.1.jar"), []).unwrap();
    let mut config = config.clone();
    // build() only checks Java's existence and permissions; it never executes it.
    config.java_path = std::env::current_exe().unwrap();
    let mut value = serde_json::json!({
        "id": "1.21.1",
        "mainClass": "net.minecraft.client.main.Main"
    });
    if modern {
        value["arguments"] = serde_json::json!({"game": [
            "--username", "${auth_player_name}", "--uuid", "${auth_uuid}"
        ]});
    } else {
        value["minecraftArguments"] =
            serde_json::json!("--username ${auth_player_name} --uuid ${auth_uuid}");
    }
    let manifest = VersionManifest::from_bytes(&serde_json::to_vec(&value).unwrap()).unwrap();
    let args = CommandBuilder::new(&manifest, dir.path(), dir.path(), &config)
        .build()
        .unwrap();
    let identities: Vec<_> = args
        .windows(2)
        .filter(|pair| pair[0] == "--uuid")
        .map(|pair| pair[1].clone())
        .collect();
    assert_eq!(identities.len(), 1);
    assert!(!args.iter().any(|arg| arg.contains("${")));
    identities[0].clone()
}

#[test]
fn offline_launches_without_a_valid_uuid_keep_the_same_identity() {
    for modern in [true, false] {
        for uuid in [
            None,
            Some(""),
            Some("invalid"),
            Some("00000000-0000-0000-0000-000000000000"),
        ] {
            let mut config = LaunchConfig::builder()
                .username("Steve")
                .cracked(true)
                .build();
            config.auth_uuid = uuid.map(str::to_owned);
            let first = launch_uuid(&config, modern);
            assert_eq!(first, "5627dd98-e6be-3c21-b8a8-e92344183641");
            assert_eq!(launch_uuid(&config, modern), first);
            config.username = "Alex".into();
            assert_ne!(launch_uuid(&config, modern), first);
        }
    }
}

#[test]
fn launches_preserve_supplied_offline_microsoft_and_yggdrasil_identities() {
    for modern in [true, false] {
        for (cracked, user_type) in [(true, "legacy"), (false, "msa"), (false, "mojang")] {
            for uuid in [
                "fd317fac-4605-4fde-bca7-c9847b745491",
                "fd317fac46054fdebca7c9847b745491",
            ] {
                let mut config = LaunchConfig::builder()
                    .username("Steve")
                    .auth_uuid(uuid)
                    .cracked(cracked)
                    .user_type(user_type)
                    .build();
                assert_eq!(launch_uuid(&config, modern), uuid);
                config.username = "RenamedPlayer".into();
                assert_eq!(launch_uuid(&config, modern), uuid);
            }
        }
    }
}

fn manifest(modern: bool) -> VersionManifest {
    let mut value = serde_json::json!({"id":"1.21.1"});
    if modern {
        value["arguments"] = serde_json::json!({"game":[
            "--username", "${auth_player_name}",
            {"rules":[{"action":"allow","features":{"is_quick_play_multiplayer":true}}],"value":["--quickPlayMultiplayer","${quickPlayMultiplayer}"]},
            {"rules":[{"action":"allow","features":{"is_quick_play_singleplayer":true}}],"value":["--quickPlaySingleplayer","${quickPlaySingleplayer}"]}
        ]});
    } else {
        value["minecraftArguments"] = serde_json::json!("--username ${auth_player_name}");
    }
    VersionManifest::from_bytes(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn game_args(manifest: &VersionManifest, config: &LaunchConfig) -> Vec<String> {
    let builder = CommandBuilder::new(manifest, Path::new("."), Path::new("."), config);
    let mut args = Vec::new();
    builder.add_game_args(
        &mut args,
        &HashMap::from([("auth_player_name".into(), "Player".into())]),
        manifest,
    );
    builder.cleanup_unresolved(&mut args);
    builder.add_optional_args(&mut args, manifest);
    args
}

#[test]
fn modern_server_launch_has_one_resolved_target_and_no_other_quick_play_flags() {
    let config = LaunchConfig::builder()
        .quick_play(crate::QuickPlay::Multiplayer("play.example:25565".into()))
        .legacy_server("srv.example", 25566)
        .build();
    assert_eq!(
        game_args(&manifest(true), &config),
        [
            "--username",
            "Player",
            "--quickPlayMultiplayer",
            "play.example:25565"
        ]
    );
}

#[test]
fn legacy_server_launch_uses_resolved_host_and_port() {
    let config = LaunchConfig::builder()
        .quick_play(crate::QuickPlay::Multiplayer("play.example:25565".into()))
        .legacy_server("srv.example", 25566)
        .build();
    assert_eq!(
        game_args(&manifest(false), &config),
        [
            "--username",
            "Player",
            "--server",
            "srv.example",
            "--port",
            "25566"
        ]
    );
}

#[test]
fn normal_launch_does_not_join_any_server() {
    assert_eq!(
        game_args(&manifest(true), &LaunchConfig::default()),
        ["--username", "Player"]
    );
    assert_eq!(
        game_args(&manifest(false), &LaunchConfig::default()),
        ["--username", "Player"]
    );
}

#[test]
fn loader_inherits_quick_play_support_from_parent() {
    let child = VersionManifest::from_bytes(br#"{"id":"fabric-loader-0.16.0-1.21.1","inheritsFrom":"1.21.1","arguments":{"game":["--loader-arg"]}}"#).unwrap();
    let resolved = child.resolve(&manifest(true));
    assert!(supports_multiplayer_quick_play(&resolved));
    let config = LaunchConfig::builder()
        .quick_play(crate::QuickPlay::Multiplayer("[::1]:25565".into()))
        .legacy_server("::1", 25565)
        .build();
    let args = game_args(&resolved, &config);
    assert_eq!(
        args.iter()
            .filter(|arg| *arg == "--quickPlayMultiplayer")
            .count(),
        1
    );
    assert!(args.contains(&"[::1]:25565".to_string()));
    assert!(!args.iter().any(|arg| arg.contains("${")));
}

fn legacy_launch(id: &str, value: serde_json::Value, config: &LaunchConfig) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let version_dir = dir.path().join("versions").join(id);
    std::fs::create_dir_all(&version_dir).unwrap();
    std::fs::write(version_dir.join(format!("{id}.jar")), []).unwrap();
    std::fs::write(
        dir.path().join("expected-assets.txt"),
        dir.path().join("assets").display().to_string(),
    )
    .unwrap();
    let manifest = VersionManifest::from_bytes(&serde_json::to_vec(&value).unwrap()).unwrap();
    let config = LaunchConfig::builder()
        .java_path(std::env::current_exe().unwrap())
        .cracked(config.cracked)
        .access_token(config.access_token.clone().unwrap_or_else(|| "0".into()))
        .build();
    let mut args = CommandBuilder::new(&manifest, dir.path(), dir.path(), &config)
        .build()
        .unwrap();
    let expected = std::fs::read_to_string(dir.path().join("expected-assets.txt")).unwrap();
    for arg in &mut args {
        if *arg == expected {
            *arg = "<assets>".into();
        }
    }
    args
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}

#[test]
fn legacy_manifests_keep_their_own_arguments_and_modern_defaults_stay_away() {
    for id in ["1.5.2", "1.6.4", "1.7.2", "13w16a", "rd-132211"] {
        let value = serde_json::json!({
            "id": id,
            "mainClass": "net.minecraft.client.main.Main",
            "minecraftArguments": "--username ${auth_player_name} --gameDir ${game_directory} --assetsDir ${game_assets} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userProperties ${user_properties} --userType ${user_type} --session ${auth_session}"
        });
        let args = legacy_launch(id, value, &LaunchConfig::default());
        assert!(!args.iter().any(|arg| arg.contains("${")), "{id}: {args:?}");
        assert_eq!(flag_value(&args, "--username"), Some("Player"), "{id}");
        assert_eq!(flag_value(&args, "--assetsDir"), Some("<assets>"), "{id}");
        let session = flag_value(&args, "--session").unwrap();
        assert!(session.starts_with("token:0:"), "{id}: {session}");
        assert_eq!(flag_value(&args, "--width"), None, "{id}");
        assert_eq!(flag_value(&args, "--height"), None, "{id}");
        assert_eq!(flag_value(&args, "--version"), None, "{id}");
    }
}

#[test]
fn modern_manifests_still_receive_default_arguments() {
    let value = serde_json::json!({
        "id": "1.21.4",
        "mainClass": "net.minecraft.client.main.Main",
        "arguments": {"game": ["--username", "${auth_player_name}"]}
    });
    let args = legacy_launch("1.21.4", value, &LaunchConfig::default());
    assert_eq!(flag_value(&args, "--width"), Some("854"));
    assert_eq!(flag_value(&args, "--assetsDir"), Some("<assets>"));
}

#[test]
fn game_assets_override_is_used_for_assets_dir_and_game_assets() {
    let dir = tempfile::tempdir().unwrap();
    let version_dir = dir.path().join("versions").join("1.5.2");
    std::fs::create_dir_all(&version_dir).unwrap();
    std::fs::write(version_dir.join("1.5.2.jar"), []).unwrap();
    let value = serde_json::json!({
        "id": "1.5.2",
        "mainClass": "net.minecraft.client.main.Main",
        "assetIndex": {"id": "pre-1.6"},
        "minecraftArguments": "--assetsDir ${game_assets} --assetIndex ${assets_index_name}"
    });
    let manifest = VersionManifest::from_bytes(&serde_json::to_vec(&value).unwrap()).unwrap();
    let resources = dir.path().join("instance").join("resources");
    let config = LaunchConfig::builder()
        .java_path(std::env::current_exe().unwrap())
        .game_assets(&resources)
        .build();
    let args = CommandBuilder::new(&manifest, dir.path(), dir.path(), &config)
        .build()
        .unwrap();
    assert_eq!(
        flag_value(&args, "--assetsDir"),
        Some(resources.to_str().unwrap())
    );
    assert_eq!(flag_value(&args, "--assetIndex"), Some("pre-1.6"));
}
