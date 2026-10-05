use super::*;

#[tokio::test]
async fn exported_minecraft_jar_and_modpack_round_trip_without_external_paths_or_cache() {
    use crate::services::minecraft_jar::{self, JarMod, MinecraftJarConfig};
    use crate::services::modpack::PackFile;
    let source = tempfile::tempdir().unwrap();
    let restored = tempfile::tempdir().unwrap();
    let jar = source.path().join("custom.jar");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
    writer
        .start_file("Minecraft.class", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"custom client").unwrap();
    writer.finish().unwrap();
    let replacement = minecraft_jar::import_file(source.path(), &jar, true).unwrap();
    let added = minecraft_jar::import_file(source.path(), &jar, false).unwrap();
    let config = MinecraftJarConfig {
        replacement: Some(replacement),
        mods: vec![JarMod {
            archive: added,
            enabled: false,
        }],
    };
    std::fs::create_dir(source.path().join("cubic-jar-cache")).unwrap();
    std::fs::write(
        source.path().join("cubic-jar-cache/minecraft.jar"),
        b"stale",
    )
    .unwrap();
    let pack = PackState {
        schema_version: 1,
        source: "modrinth".into(),
        project_id: Some("pack-project".into()),
        version_id: Some("pack-release".into()),
        name: "Original pack".into(),
        author: Some("Pack author".into()),
        version: "v1".into(),
        game_version: "1.12.2".into(),
        locked: false,
        needs_inventory: false,
        files: [
            "mods/owned.jar",
            "config/pack.json",
            "pack.txt",
            "extras/nested/pack.txt",
            "deleted.txt",
        ]
        .into_iter()
        .map(|path| {
            (
                path.into(),
                PackFile {
                    sha1: "0123456789abcdef0123456789abcdef01234567".into(),
                    size: 123,
                    downloads: vec!["https://example.org/original".into()],
                },
            )
        })
        .collect(),
        identities: [("mods/owned.jar".into(), ["original_mod".into()].into())].into(),
    };
    for relative in pack.files.keys().filter(|path| *path != "deleted.txt") {
        let path = source.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"locally customized").unwrap();
    }
    // Metadata internos incluso dentro de carpetas de juego no pertenecen al backup.
    for relative in [
        "config/instance.cub",
        "config/modpack.cub.json",
        "config/cache.crep",
        "unowned.txt",
    ] {
        std::fs::write(source.path().join(relative), b"internal").unwrap();
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&jar, source.path().join("mods/linked.jar")).unwrap();
        std::os::unix::fs::symlink(source.path(), source.path().join("config/loop")).unwrap();
    }
    modpack::save(source.path(), &pack).unwrap();
    let input = ExportInput {
        uuid: "uuid".into(),
        name: "Custom".into(),
        version_id: "1.12.2".into(),
        mc_version: "1.12.2".into(),
        loader_name: "Vanilla".into(),
        loader_version: None,
        loader_mmc_uid: None,
        instance_dir: source.path().to_owned(),
        min_memory: 512,
        max_memory: 2048,
        overrides: None,
        icon_src: None,
        minecraft_jar: config.clone(),
        modpack: modpack::read_state(source.path()).await.unwrap(),
    };
    let output = source.path().join("export.zip");
    export_to_zip(&input, &output).unwrap();
    let mut archive = zip::ZipArchive::new(std::fs::File::open(output).unwrap()).unwrap();
    assert!(!archive.file_names().any(|n| n.contains("cubic-jar-cache")));
    let names: Vec<_> = archive.file_names().map(str::to_owned).collect();
    assert_eq!(
        names.len(),
        names.iter().collect::<std::collections::HashSet<_>>().len()
    );
    for relative in [
        "modpack.cub.json",
        "config/instance.cub",
        "config/modpack.cub.json",
        "config/cache.crep",
        "unowned.txt",
        "mods/linked.jar",
        "config/loop",
        "deleted.txt",
    ] {
        assert!(
            !names.contains(&format!(".minecraft/{relative}")),
            "{relative}"
        );
    }
    let metadata: serde_json::Value =
        serde_json::from_reader(archive.by_name("cubic-manifest.json").unwrap()).unwrap();
    let imported: MinecraftJarConfig =
        serde_json::from_value(metadata["minecraft_jar"].clone()).unwrap();
    assert_eq!(imported, config);
    let imported_pack: PackState = serde_json::from_value(metadata["modpack"].clone()).unwrap();
    assert_eq!(imported_pack, pack);
    assert!(
        !metadata
            .to_string()
            .contains(source.path().to_str().unwrap())
    );
    archive.extract(restored.path()).unwrap();
    for relative in pack.files.keys().filter(|path| *path != "deleted.txt") {
        assert_eq!(
            std::fs::read(restored.path().join(".minecraft").join(relative)).unwrap(),
            b"locally customized"
        );
    }
    let target = restored.path().join("target");
    minecraft_jar::copy_inputs(&restored.path().join(".minecraft"), &target, &imported).unwrap();
    let client = minecraft_jar::prepare(&target, Path::new("missing-original.jar"), &imported)
        .unwrap()
        .unwrap();
    assert_eq!(std::fs::read(client).unwrap(), std::fs::read(jar).unwrap());
}

#[test]
fn test_loader_to_mmc_uid() {
    assert_eq!(loader_to_mmc_uid(&Loader::Vanilla), None);
    assert_eq!(
        loader_to_mmc_uid(&Loader::Fabric("0.15.0".into())),
        Some("net.fabricmc.fabric-loader")
    );
    assert_eq!(
        loader_to_mmc_uid(&Loader::NeoForge("21.0.0".into())),
        Some("net.neoforged")
    );
}

#[test]
fn test_build_mmc_pack_json() {
    let input = ExportInput {
        uuid: "uuid".into(),
        name: "Test".into(),
        version_id: "1.21-fabric-0.15.0".into(),
        mc_version: "1.21".into(),
        loader_name: "Fabric".into(),
        loader_version: Some("0.15.0".into()),
        loader_mmc_uid: loader_to_mmc_uid(&Loader::Fabric("0.15.0".into())),
        instance_dir: PathBuf::new(),
        min_memory: 512,
        max_memory: 2048,
        overrides: None,
        minecraft_jar: Default::default(),
        modpack: None,
        icon_src: None,
    };
    let json = build_mmc_pack(&input);
    assert!(json.contains("net.minecraft"));
    assert!(json.contains("net.fabricmc.fabric-loader"));
    assert!(json.contains("0.15.0"));
}

#[test]
fn test_build_instance_cfg() {
    let input = ExportInput {
        uuid: "uuid".into(),
        name: "MiInstancia".into(),
        version_id: "1.20.1".into(),
        mc_version: "1.20.1".into(),
        loader_name: "Vanilla".into(),
        loader_version: None,
        loader_mmc_uid: None,
        instance_dir: PathBuf::new(),
        min_memory: 1024,
        max_memory: 4096,
        overrides: None,
        minecraft_jar: Default::default(),
        modpack: None,
        icon_src: None,
    };
    let cfg = build_instance_cfg(&input);
    assert!(cfg.contains("name=MiInstancia"));
    assert!(cfg.contains("MinMemAlloc=1024"));
    assert!(cfg.contains("MaxMemAlloc=4096"));
    assert!(!cfg.contains("iconKey"));
    assert!(!cfg.contains("JvmArgs"));
}

#[test]
fn test_build_instance_cfg_exports_per_instance_jvm_args() {
    use crate::services::instance_manager::data::InstOverrides;
    let input = ExportInput {
        uuid: "uuid".into(),
        name: "ConArgs".into(),
        version_id: "1.20.1".into(),
        mc_version: "1.20.1".into(),
        loader_name: "Vanilla".into(),
        loader_version: None,
        loader_mmc_uid: None,
        instance_dir: PathBuf::new(),
        min_memory: 1024,
        max_memory: 4096,
        overrides: Some(InstOverrides {
            java_version: None,
            memory: None,
            jvm_args: Some(vec!["-XX:+UseG1GC".into(), "-Dfoo=bar".into()]),
        }),
        minecraft_jar: Default::default(),
        modpack: None,
        icon_src: None,
    };
    let cfg = build_instance_cfg(&input);
    assert!(cfg.contains("OverrideJavaArgs=true"));
    assert!(cfg.contains("JvmArgs=-XX:+UseG1GC -Dfoo=bar"));
}
