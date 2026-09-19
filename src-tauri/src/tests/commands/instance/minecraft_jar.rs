use super::*;
use std::io::Write;

fn write_jar(path: &std::path::Path) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    zip.start_file("Minecraft.class", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"test").unwrap();
    zip.finish().unwrap();
}

#[test]
fn failed_multi_add_is_atomic_and_removes_copied_files() {
    let dir = tempfile::tempdir().unwrap();
    let valid = dir.path().join("valid.jar");
    let invalid = dir.path().join("invalid.jar");
    write_jar(&valid);
    std::fs::write(&invalid, b"invalid").unwrap();
    let mut config = MinecraftJarConfig::default();
    assert!(
        apply_action(
            dir.path(),
            &mut config,
            JarAction::Add {
                paths: vec![valid, invalid]
            }
        )
        .is_err()
    );
    assert_eq!(config, MinecraftJarConfig::default());
    assert_eq!(
        std::fs::read_dir(dir.path().join(minecraft_jar::INPUT_DIR))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn actions_manage_mods_and_restore_only_the_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let jar = dir.path().join("test.jar");
    write_jar(&jar);
    let mut config = MinecraftJarConfig::default();
    apply_action(
        dir.path(),
        &mut config,
        JarAction::Add {
            paths: vec![jar.clone(), jar.clone()],
        },
    )
    .unwrap();
    let first = config.mods[0].archive.file.clone();
    apply_action(
        dir.path(),
        &mut config,
        JarAction::Toggle {
            file: first.clone(),
            enabled: false,
        },
    )
    .unwrap();
    apply_action(
        dir.path(),
        &mut config,
        JarAction::Move {
            file: first.clone(),
            offset: 1,
        },
    )
    .unwrap();
    assert_eq!(config.mods[1].archive.file, first);
    assert!(!config.mods[1].enabled);
    assert!(
        apply_action(
            dir.path(),
            &mut config,
            JarAction::Move {
                file: first.clone(),
                offset: 1
            }
        )
        .is_err()
    );
    apply_action(dir.path(), &mut config, JarAction::Replace { path: jar }).unwrap();
    let before = config.clone();
    apply_action(dir.path(), &mut config, JarAction::Restore).unwrap();
    assert_eq!(config.mods, before.mods);
    cleanup_removed(dir.path(), &before, &config);
    assert!(
        !dir.path()
            .join(minecraft_jar::INPUT_DIR)
            .join(before.replacement.unwrap().file)
            .exists()
    );
    apply_action(dir.path(), &mut config, JarAction::Remove { file: first }).unwrap();
    assert_eq!(config.mods.len(), 1);
}
