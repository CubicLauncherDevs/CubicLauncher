use super::*;
use std::io::Read;

fn archive(path: &Path, entries: &[(&str, &[u8])]) {
    let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
    for (name, contents) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap();
}

fn contents(path: &Path, name: &str) -> Vec<u8> {
    let mut zip = open_archive(path).unwrap();
    let mut result = Vec::new();
    zip.by_name(name).unwrap().read_to_end(&mut result).unwrap();
    result
}

#[test]
fn jar_mods_merge_in_order_preserving_resources_and_shared_original() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("original.jar");
    archive(
        &original,
        &[
            ("Minecraft.class", b"original"),
            ("asset.png", b"image"),
            ("META-INF/MANIFEST.MF", b"signed"),
            ("META-INF/MOJANG.RSA", b"signature"),
            ("META-INF/services/service", b"provider"),
        ],
    );
    let before = std::fs::read(&original).unwrap();
    let mod_path = root.path().join("mod.zip");
    archive(
        &mod_path,
        &[("Minecraft.class", b"first"), ("new.class", b"new")],
    );
    let first = import_file(root.path(), &mod_path, false).unwrap();
    archive(
        &mod_path,
        &[
            ("Minecraft.class", b"second"),
            ("META-INF/TEST.SF", b"signature"),
        ],
    );
    let second = import_file(root.path(), &mod_path, false).unwrap();
    let mut config = MinecraftJarConfig {
        replacement: None,
        mods: vec![
            JarMod {
                archive: first,
                enabled: true,
            },
            JarMod {
                archive: second,
                enabled: true,
            },
        ],
    };
    let result = prepare(root.path(), &original, &config).unwrap().unwrap();
    assert_eq!(contents(&result, "Minecraft.class"), b"second");
    assert_eq!(contents(&result, "asset.png"), b"image");
    assert_eq!(contents(&result, "new.class"), b"new");
    assert_eq!(contents(&result, "META-INF/services/service"), b"provider");
    assert!(
        !open_archive(&result)
            .unwrap()
            .file_names()
            .any(is_signature)
    );
    config.mods.swap(0, 1);
    prepare(root.path(), &original, &config).unwrap();
    assert_eq!(contents(&result, "Minecraft.class"), b"first");
    config.mods[1].enabled = false;
    prepare(root.path(), &original, &config).unwrap();
    assert_eq!(contents(&result, "Minecraft.class"), b"second");
    assert_eq!(std::fs::read(&original).unwrap(), before);
    let other = root.path().join("other-instance");
    assert_eq!(
        prepare(&other, &original, &MinecraftJarConfig::default()).unwrap(),
        None
    );
    assert!(!other.exists());
}

#[test]
fn replacement_is_copied_and_can_be_patched_then_restored() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("original.jar");
    let source = root.path().join("custom.jar");
    archive(&original, &[("Minecraft.class", b"vanilla")]);
    archive(&source, &[("Minecraft.class", b"custom")]);
    let replacement = import_file(root.path(), &source, true).unwrap();
    std::fs::remove_file(source).unwrap();
    let mut config = MinecraftJarConfig {
        replacement: Some(replacement),
        mods: vec![],
    };
    let result = prepare(root.path(), &original, &config).unwrap().unwrap();
    assert_eq!(contents(&result, "Minecraft.class"), b"custom");
    let mod_path = root.path().join("mod.zip");
    archive(&mod_path, &[("asset", b"added")]);
    config.mods.push(JarMod {
        archive: import_file(root.path(), &mod_path, false).unwrap(),
        enabled: true,
    });
    let result = prepare(root.path(), &original, &config).unwrap().unwrap();
    assert_eq!(contents(&result, "Minecraft.class"), b"custom");
    config.replacement = None;
    prepare(root.path(), &original, &config).unwrap();
    assert_eq!(contents(&result, "Minecraft.class"), b"vanilla");
    assert_eq!(contents(&result, "asset"), b"added");
    config.mods[0].enabled = false;
    assert_eq!(prepare(root.path(), &original, &config).unwrap(), None);
}

#[test]
fn failed_rebuild_keeps_previous_output_and_does_not_fall_back() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().join("base.jar");
    archive(&base, &[("Minecraft.class", b"base")]);
    let item = import_file(root.path(), &base, false).unwrap();
    let config = MinecraftJarConfig {
        replacement: None,
        mods: vec![JarMod {
            archive: item.clone(),
            enabled: true,
        }],
    };
    let output = prepare(root.path(), &base, &config).unwrap().unwrap();
    let before = std::fs::read(&output).unwrap();
    std::fs::write(root.path().join(INPUT_DIR).join(item.file), b"corrupt").unwrap();
    assert!(prepare(root.path(), &base, &config).is_err());
    assert_eq!(std::fs::read(output).unwrap(), before);
}

#[test]
fn invalid_archives_and_paths_are_rejected_without_committing_inputs() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("bad.jar");
    for entries in [
        vec![],
        vec![("../outside.class", b"bad".as_slice())],
        vec![("asset", b"no classes".as_slice())],
    ] {
        archive(&source, &entries);
        assert!(import_file(root.path(), &source, true).is_err());
    }
    assert_eq!(
        std::fs::read_dir(root.path().join(INPUT_DIR))
            .unwrap()
            .count(),
        0
    );
    let config = MinecraftJarConfig {
        replacement: Some(JarFile {
            file: "../../outside.jar".into(),
            name: "unsafe".into(),
        }),
        mods: vec![],
    };
    assert!(prepare(root.path(), &source, &config).is_err());
    assert!(copy_inputs(root.path(), root.path(), &config).is_err());
}

#[test]
fn export_import_inputs_preserve_config_and_skip_cache() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let jar = source.path().join("source.jar");
    archive(&jar, &[("Minecraft.class", b"custom")]);
    let replacement = import_file(source.path(), &jar, true).unwrap();
    let item = import_file(source.path(), &jar, false).unwrap();
    let config = MinecraftJarConfig {
        replacement: Some(replacement),
        mods: vec![JarMod {
            archive: item,
            enabled: false,
        }],
    };
    let serialized = serde_json::to_string(&config).unwrap();
    let restored: MinecraftJarConfig = serde_json::from_str(&serialized).unwrap();
    copy_inputs(source.path(), target.path(), &restored).unwrap();
    assert_eq!(config, restored);
    for file in restored.files() {
        assert_eq!(
            contents(
                &target.path().join(INPUT_DIR).join(&file.file),
                "Minecraft.class"
            ),
            b"custom"
        );
    }
    assert!(!target.path().join("cubic-jar-cache").exists());
}
