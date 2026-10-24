use super::*;

#[tokio::test]
async fn minecraft_jar_settings_survive_save_reload_and_instance_rename() {
    use crate::services::minecraft_jar::{JarFile, JarMod, MinecraftJarConfig};
    let temp = tempfile::tempdir().unwrap();
    let mut data = InstanceData::new("Pack".into(), "1.21".into(), None);
    data.instance_root = temp.path().to_owned();
    let dir = data.get_instance_dir();
    std::fs::create_dir(&dir).unwrap();
    let handle = InstanceHandle::new(data);
    let _guard = handle.try_lock_files().unwrap();
    let config = MinecraftJarConfig {
        replacement: Some(JarFile {
            file: format!("{}.jar", uuid::Uuid::new_v4()),
            name: "custom.jar".into(),
        }),
        mods: vec![JarMod {
            archive: JarFile {
                file: format!("{}.jar", uuid::Uuid::new_v4()),
                name: "mod.zip".into(),
            },
            enabled: false,
        }],
    };
    handle.save_minecraft_jar(config.clone()).await.unwrap();
    let mut loaded: InstanceData =
        serde_json::from_slice(&std::fs::read(dir.join("instance.cub")).unwrap()).unwrap();
    assert_eq!(loaded.minecraft_jar, config);
    loaded.name = "Renamed".into();
    loaded.instance_root = temp.path().to_owned();
    assert_eq!(loaded.minecraft_jar, config);
    assert_eq!(loaded.get_instance_dir(), temp.path().join("Renamed"));
    // Failed persistence must not make the UI observe unsaved JAR settings.
    std::fs::remove_file(dir.join("instance.cub")).unwrap();
    std::fs::remove_dir(&dir).unwrap();
    assert!(
        handle
            .save_minecraft_jar(MinecraftJarConfig::default())
            .await
            .is_err()
    );
    assert_eq!(handle.get_minecraft_jar().await, config);
}

#[tokio::test]
async fn play_session_accumulates_elapsed_seconds_and_persists() {
    let temp = tempfile::tempdir().unwrap();
    let mut data = InstanceData::new("Pack".into(), "1.21".into(), None);
    data.instance_root = temp.path().to_path_buf();
    let dir = data.get_instance_dir();
    tokio::fs::create_dir(&dir).await.unwrap();
    let handle = InstanceHandle::new(data);
    assert_eq!(handle.get_playtime_seconds().await, 0);

    // Una sesión demasiado corta no altera el total.
    handle.begin_play_session();
    handle.end_play_session().await;
    assert_eq!(handle.get_playtime_seconds().await, 0);

    handle.begin_play_session();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    handle.end_play_session().await;
    let total = handle.get_playtime_seconds().await;
    assert!(total >= 1, "se esperaba al menos un segundo, hubo {total}");

    let saved: InstanceData =
        serde_json::from_slice(&std::fs::read(dir.join("instance.cub")).unwrap()).unwrap();
    assert_eq!(saved.playtime_seconds, total);
}

#[tokio::test]
async fn a_save_queued_before_deletion_is_rejected_after_admission() {
    let temp = tempfile::tempdir().unwrap();
    let mut data = InstanceData::new("Pack".into(), "1.21".into(), None);
    data.instance_root = temp.path().to_path_buf();
    let dir = data.get_instance_dir();
    tokio::fs::create_dir(&dir).await.unwrap();
    let handle = InstanceHandle::new(data);
    let guard = handle.try_lock_files().unwrap();
    let save = handle.save_if_dirty();
    tokio::pin!(save);
    assert!(futures::poll!(&mut save).is_pending());
    tokio::fs::remove_dir_all(&dir).await.unwrap();
    guard.mark_deleted();
    drop(guard);
    assert_eq!(save.await.unwrap_err().kind(), io::ErrorKind::NotFound);
    assert!(!dir.exists());
}

#[tokio::test]
async fn world_operation_lock_survives_cancelled_async_caller() {
    let handle = InstanceHandle::new(InstanceData::new("test".into(), "1.21".into(), None));
    let cloned = handle.clone();
    let guard = handle.try_lock_files().unwrap();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            drop(guard);
            done_tx.send(()).unwrap();
        })
        .await
        .unwrap();
    });
    started_rx.await.unwrap();
    assert!(cloned.try_lock_files().is_err());
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(handle.try_lock_files().is_err());
    release_tx.send(()).unwrap();
    done_rx.await.unwrap();
    assert!(handle.try_lock_files().is_ok());
}

fn write_png(path: &std::path::Path, width: u32, height: u32) {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]));
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[tokio::test]
async fn legacy_icon_is_migrated_to_a_bounded_png() {
    let temp = tempfile::tempdir().unwrap();
    let mut data = InstanceData::new(
        "Pack".into(),
        "1.21".into(),
        Some("icons/custom_1.png".into()),
    );
    data.instance_root = temp.path().to_path_buf();
    let dir = data.get_instance_dir();
    std::fs::create_dir_all(dir.join("icons")).unwrap();
    let legacy = dir.join("icons/custom_1.png");
    write_png(&legacy, 512, 512);
    data.save().await.unwrap();
    let handle = InstanceHandle::new(data);

    assert!(handle.migrate_icon_if_needed().await);

    let icon = handle.get_icon().await.unwrap();
    assert!(icon.starts_with("icons/icon_"));
    assert!(icon.ends_with(".png"));
    assert!(!legacy.exists());

    let migrated =
        image::load_from_memory(&std::fs::read(dir.join(icon.as_ref())).unwrap()).unwrap();
    assert_eq!((migrated.width(), migrated.height()), (256, 256));

    let saved: InstanceData =
        serde_json::from_slice(&std::fs::read(dir.join("instance.cub")).unwrap()).unwrap();
    assert_eq!(saved.icon.as_deref(), Some(icon.as_ref()));

    // Idempotente: una segunda pasada no vuelve a tocar nada.
    assert!(!handle.migrate_icon_if_needed().await);
}

#[tokio::test]
async fn a_corrupt_legacy_icon_does_not_break_the_instance() {
    let temp = tempfile::tempdir().unwrap();
    let mut data = InstanceData::new(
        "Pack".into(),
        "1.21".into(),
        Some("icons/custom_1.png".into()),
    );
    data.instance_root = temp.path().to_path_buf();
    let dir = data.get_instance_dir();
    std::fs::create_dir_all(dir.join("icons")).unwrap();
    let legacy = dir.join("icons/custom_1.png");
    std::fs::write(&legacy, b"not an image").unwrap();
    data.save().await.unwrap();
    let handle = InstanceHandle::new(data);

    assert!(!handle.migrate_icon_if_needed().await);
    assert_eq!(
        handle.get_icon().await.as_deref(),
        Some("icons/custom_1.png")
    );
    assert!(legacy.exists());
}
