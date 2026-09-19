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
