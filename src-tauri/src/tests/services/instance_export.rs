use super::*;

#[test]
fn exported_minecraft_jar_inputs_round_trip_without_external_paths_or_cache() {
    use crate::services::minecraft_jar::{self, JarMod, MinecraftJarConfig};
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
    };
    let output = source.path().join("export.zip");
    export_to_zip(&input, &output).unwrap();
    let mut archive = zip::ZipArchive::new(std::fs::File::open(output).unwrap()).unwrap();
    assert!(!archive.file_names().any(|n| n.contains("cubic-jar-cache")));
    let metadata: serde_json::Value =
        serde_json::from_reader(archive.by_name("cubic-manifest.json").unwrap()).unwrap();
    let imported: MinecraftJarConfig =
        serde_json::from_value(metadata["minecraft_jar"].clone()).unwrap();
    assert_eq!(imported, config);
    assert!(
        !metadata
            .to_string()
            .contains(source.path().to_str().unwrap())
    );
    archive.extract(restored.path()).unwrap();
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
        icon_src: None,
    };
    let cfg = build_instance_cfg(&input);
    assert!(cfg.contains("name=MiInstancia"));
    assert!(cfg.contains("MinMemAlloc=1024"));
    assert!(cfg.contains("MaxMemAlloc=4096"));
    assert!(!cfg.contains("iconKey"));
}
