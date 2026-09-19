use super::*;

async fn fixture() -> (tempfile::TempDir, Arc<InstanceManager>) {
    let temp = tempfile::tempdir().unwrap();
    let instance_dir = temp.path().join("instances");
    tokio_fs::create_dir(&instance_dir).await.unwrap();
    let manager = Arc::new(InstanceManager {
        instances: Arc::new(RwLock::new(HashMap::new())),
        instance_dir,
        _sync_handle: tokio::spawn(async {}),
    });
    (temp, manager)
}

async fn create(manager: &InstanceManager) -> InstanceHandle {
    manager
        .create_instance("Pack".into(), "1.21".into(), None)
        .await
        .unwrap()
}

#[tokio::test]
async fn deleted_handles_cannot_save_or_touch_a_replacement_with_the_same_name() {
    let (_temp, manager) = fixture().await;
    let old = create(&manager).await;
    let stale = old.clone();
    stale.set_pinned(true).await;
    let dir = old.get_instance_dir().await;
    manager.delete_instance(&old.uuid).await.unwrap();
    assert!(!dir.exists());
    assert!(manager.get_handle(&old.uuid).await.is_none());
    assert!(stale.try_lock_files().is_err());
    assert_eq!(
        stale.save_if_dirty().await.unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    assert!(!dir.exists());

    let replacement = create(&manager).await;
    let manifest = tokio_fs::read(dir.join("instance.cub")).await.unwrap();
    stale.set_icon(Some("old.png".into())).await;
    assert!(stale.save_if_dirty().await.is_err());
    assert!(manager.delete_instance(&old.uuid).await.is_err());
    assert_eq!(
        tokio_fs::read(dir.join("instance.cub")).await.unwrap(),
        manifest
    );
    assert_ne!(replacement.uuid, old.uuid);
    assert_eq!(manager.count().await, 1);
}

#[tokio::test]
async fn deletion_rejects_an_active_file_operation_without_changing_disk_or_memory() {
    let (_temp, manager) = fixture().await;
    let handle = create(&manager).await;
    let guard = handle.try_lock_files().unwrap();
    handle.set_pinned(true).await;
    guard.save_if_dirty().await.unwrap();
    assert!(manager.delete_instance(&handle.uuid).await.is_err());
    assert!(
        handle
            .get_instance_dir()
            .await
            .join("instance.cub")
            .is_file()
    );
    assert_eq!(manager.count().await, 1);
    drop(guard);
    manager.delete_instance(&handle.uuid).await.unwrap();
}

#[tokio::test]
async fn failed_quarantine_keeps_the_complete_instance_and_allows_retry() {
    let (_temp, manager) = fixture().await;
    let handle = create(&manager).await;
    let dir = handle.get_instance_dir().await;
    tokio_fs::write(dir.join("world.dat"), "world data")
        .await
        .unwrap();
    let trash = trash_dir(&manager.instance_dir);
    tokio_fs::write(&trash, "blocked destination")
        .await
        .unwrap();
    assert!(manager.delete_instance(&handle.uuid).await.is_err());
    assert_eq!(
        tokio_fs::read_to_string(dir.join("world.dat"))
            .await
            .unwrap(),
        "world data"
    );
    assert!(dir.join("instance.cub").is_file());
    assert_eq!(manager.count().await, 1);
    assert!(handle.try_lock_files().is_ok());
    handle.set_pinned(true).await;
    handle.save_if_dirty().await.unwrap();
    tokio_fs::remove_file(&trash).await.unwrap();
    manager.delete_instance(&handle.uuid).await.unwrap();
    assert!(!dir.exists());
}

#[tokio::test]
async fn missing_directory_can_be_deleted_and_autosave_never_recreates_it() {
    let (_temp, manager) = fixture().await;
    let handle = create(&manager).await;
    let dir = handle.get_instance_dir().await;
    tokio_fs::remove_dir_all(&dir).await.unwrap();
    handle.set_pinned(true).await;
    assert_eq!(
        handle.save_if_dirty().await.unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    assert!(!dir.exists());
    // A missing folder must not allow two live UUIDs to share the same path.
    assert!(
        manager
            .create_instance("Pack".into(), "1.21".into(), None)
            .await
            .is_err()
    );
    manager.delete_instance(&handle.uuid).await.unwrap();
    assert_eq!(manager.count().await, 0);
}

#[tokio::test]
async fn quarantine_is_recoverable_and_cleanup_does_not_touch_live_instances() {
    let (_temp, manager) = fixture().await;
    let handle = create(&manager).await;
    let dir = handle.get_instance_dir().await;
    let trash = trash_dir(&manager.instance_dir);
    quarantine_instance(&dir, &trash, &handle.uuid)
        .await
        .unwrap();
    assert!(!dir.exists());
    assert!(
        trash
            .join(handle.uuid.as_ref())
            .join("instance.cub")
            .is_file()
    );
    tokio_fs::create_dir(&dir).await.unwrap();
    tokio_fs::write(dir.join("new-world"), "keep")
        .await
        .unwrap();
    tokio_fs::create_dir(trash.join("unrelated")).await.unwrap();
    cleanup_deleted_instances(&trash).await;
    assert!(!trash.join(handle.uuid.as_ref()).exists());
    assert!(trash.join("unrelated").is_dir());
    assert_eq!(
        tokio_fs::read_to_string(dir.join("new-world"))
            .await
            .unwrap(),
        "keep"
    );
    cleanup_deleted_instances(&trash).await;
}

#[tokio::test]
async fn concurrent_creates_reserve_a_name_once() {
    let (_temp, manager) = fixture().await;
    let (a, b) = tokio::join!(
        manager.create_instance("Pack".into(), "1.21".into(), None),
        manager.create_instance("Pack".into(), "1.21".into(), None),
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert_eq!(manager.count().await, 1);
}

#[tokio::test]
async fn admitted_deletion_finishes_when_the_caller_is_cancelled() {
    let (_temp, manager) = fixture().await;
    let handle = create(&manager).await;
    let dir = handle.get_instance_dir().await;
    let mut deletion = Box::pin(manager.delete_instance(&handle.uuid));
    assert!(futures::poll!(deletion.as_mut()).is_pending());
    drop(deletion);
    time::timeout(Duration::from_secs(2), async {
        while manager.get_handle(&handle.uuid).await.is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!dir.exists());
    assert!(handle.save_if_dirty().await.is_err());
}

#[tokio::test]
async fn failed_cleanup_retains_the_pending_entry_for_a_later_retry() {
    let (_temp, manager) = fixture().await;
    let trash = trash_dir(&manager.instance_dir);
    tokio_fs::create_dir(&trash).await.unwrap();
    let pending = trash.join(uuid::Uuid::new_v4().to_string());
    // An unexpected file cannot be removed with remove_dir_all on any platform.
    tokio_fs::write(&pending, "blocked cleanup").await.unwrap();
    cleanup_deleted_instances(&trash).await;
    assert!(pending.exists());
    tokio_fs::remove_file(&pending).await.unwrap();
    tokio_fs::create_dir(&pending).await.unwrap();
    tokio_fs::write(pending.join("instance.cub"), "pending")
        .await
        .unwrap();
    cleanup_deleted_instances(&trash).await;
    assert!(!pending.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn deletion_staging_stays_inside_a_symlinked_instance_root() {
    let temp = tempfile::tempdir().unwrap();
    let physical = temp.path().join("physical");
    tokio_fs::create_dir(&physical).await.unwrap();
    let logical = temp.path().join("instances");
    std::os::unix::fs::symlink(&physical, &logical).unwrap();
    let manager = InstanceManager {
        instances: Arc::new(RwLock::new(HashMap::new())),
        instance_dir: logical.clone(),
        _sync_handle: tokio::spawn(async {}),
    };
    let handle = create(&manager).await;
    let trash = trash_dir(&logical);
    assert!(validate_instance_name(DELETION_DIR).is_err());
    quarantine_instance(&handle.get_instance_dir().await, &trash, &handle.uuid)
        .await
        .unwrap();
    assert!(
        physical
            .join(DELETION_DIR)
            .join(handle.uuid.as_ref())
            .is_dir()
    );
    cleanup_deleted_instances(&trash).await;
    assert!(logical.is_symlink());
    assert!(!physical.join("Pack").exists());
}

#[test]
fn kill_request_survives_cleanup_before_the_signal_is_received() {
    let id = uuid::Uuid::new_v4().to_string();
    let (tx, mut rx) = oneshot::channel();
    let requested = register_kill_sender(&id, tx);

    assert!(!requested.load(Ordering::Acquire));
    assert!(signal_kill(&id));
    unregister_kill_sender(&id);
    assert!(requested.load(Ordering::Acquire));
    assert_eq!(rx.try_recv(), Ok(()));
    assert!(!signal_kill(&id));
}

#[test]
fn unregistering_does_not_count_as_a_kill_request() {
    let id = uuid::Uuid::new_v4().to_string();
    let (tx, mut rx) = oneshot::channel();
    let requested = register_kill_sender(&id, tx);

    unregister_kill_sender(&id);
    assert!(!requested.load(Ordering::Acquire));
    assert_eq!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed));
    assert!(!signal_kill(&id));
}

#[test]
fn a_new_execution_does_not_inherit_a_previous_kill_request() {
    let id = uuid::Uuid::new_v4().to_string();
    let (old_tx, _old_rx) = oneshot::channel();
    let old_requested = register_kill_sender(&id, old_tx);
    assert!(signal_kill(&id));

    let (new_tx, _new_rx) = oneshot::channel();
    let new_requested = register_kill_sender(&id, new_tx);
    unregister_kill_sender(&id);
    assert!(old_requested.load(Ordering::Acquire));
    assert!(!new_requested.load(Ordering::Acquire));
}

#[test]
fn signaling_a_closed_receiver_returns_false() {
    let id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel();
    register_kill_sender(&id, tx);
    drop(rx);
    assert!(!signal_kill(&id));
}
