use super::*;

fn launch_uuid(config: &LaunchConfig, modern: bool) -> String {
    let dir = tempfile::tempdir().unwrap();
    let version_dir = dir.path().join("versions/1.21.1");
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
