use super::*;

#[tokio::test]
async fn import_copies_new_file_without_overwriting_existing_content() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("source.jar");
    let dest = dir.path().join("installed.jar");
    tokio::fs::write(&src, b"new content").await.unwrap();
    copy_instance_file(&src, &dest, false).await.unwrap();
    assert_eq!(tokio::fs::read(&dest).await.unwrap(), b"new content");

    tokio::fs::write(&src, b"replacement").await.unwrap();
    let error = copy_instance_file(&src, &dest, false).await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(tokio::fs::read(&dest).await.unwrap(), b"new content");
}

#[tokio::test]
async fn importing_a_file_from_its_own_folder_does_not_truncate_it() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("mod.jar");
    tokio::fs::write(&src, b"original").await.unwrap();
    assert!(copy_instance_file(&src, &src, false).await.is_err());
    assert_eq!(tokio::fs::read(&src).await.unwrap(), b"original");
}

#[tokio::test]
async fn concurrent_imports_to_one_filename_have_exactly_one_winner() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.zip");
    let second = dir.path().join("second.zip");
    let dest = dir.path().join("installed.zip");
    tokio::fs::write(&first, vec![1; 100_000]).await.unwrap();
    tokio::fs::write(&second, vec![2; 200_000]).await.unwrap();
    let (a, b) = tokio::join!(
        copy_instance_file(&first, &dest, false),
        copy_instance_file(&second, &dest, false)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let expected = if a.is_ok() {
        vec![1; 100_000]
    } else {
        vec![2; 200_000]
    };
    assert_eq!(tokio::fs::read(&dest).await.unwrap(), expected);
}

#[tokio::test]
async fn missing_source_does_not_leave_an_empty_destination() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("installed.jar");
    assert!(
        copy_instance_file(&dir.path().join("missing.jar"), &dest, false)
            .await
            .is_err()
    );
    assert!(!dest.exists());
}
