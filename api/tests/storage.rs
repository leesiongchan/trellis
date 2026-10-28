#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::storage::{KnowledgeStore, StorageConfig};
use std::{collections::HashMap, path::Path};

fn config(root: &Path) -> StorageConfig {
    StorageConfig::from_values(
        &HashMap::from([(
            "APP_DATA_DIR".into(),
            root.join("data").display().to_string(),
        )]),
        &root.join("release/branch-backend"),
    )
    .unwrap()
}

#[tokio::test]
async fn storage_exclusive_open_releases_after_close() {
    let directory = tempfile::tempdir().unwrap();
    let settings = config(directory.path());
    let first = KnowledgeStore::open(&settings).await.unwrap();
    assert!(KnowledgeStore::open(&settings).await.is_err());
    first.close().await.unwrap();
    KnowledgeStore::open(&settings)
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
}

#[test]
fn storage_rejects_relative_or_missing_data_root() {
    assert!(
        StorageConfig::from_values(&HashMap::new(), Path::new("/tmp/release/branch-backend"))
            .is_err()
    );
    assert!(
        StorageConfig::from_values(
            &HashMap::from([("APP_DATA_DIR".into(), "relative/data".into())]),
            Path::new("/tmp/release/branch-backend")
        )
        .is_err()
    );
}

#[test]
fn storage_rejects_overlapping_roots_and_symlink_escape() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let executable = root.join("release/branch-backend");
    for data in [
        root.to_path_buf(),
        root.join("release/public/private"),
        root.join("release/models/private"),
    ] {
        assert!(
            StorageConfig::from_values(
                &HashMap::from([("APP_DATA_DIR".into(), data.display().to_string())]),
                &executable
            )
            .is_err()
        );
    }
    assert!(
        StorageConfig::from_values(
            &HashMap::from([
                (
                    "APP_DATA_DIR".into(),
                    root.join("data").display().to_string()
                ),
                (
                    "APP_BACKUP_DIR".into(),
                    root.join("data/backups").display().to_string()
                ),
            ]),
            &executable
        )
        .is_err()
    );
    std::fs::create_dir_all(root.join("release/public")).unwrap();
    std::os::unix::fs::symlink(root.join("release/public"), root.join("alias")).unwrap();
    assert!(
        StorageConfig::from_values(
            &HashMap::from([(
                "APP_DATA_DIR".into(),
                root.join("alias/private").display().to_string()
            ),]),
            &executable
        )
        .is_err()
    );
    assert!(
        StorageConfig::from_values(
            &HashMap::from([(
                "APP_DATA_DIR".into(),
                root.join("unknown/../release/public/private")
                    .display()
                    .to_string()
            ),]),
            &executable
        )
        .is_err()
    );
}

#[tokio::test]
async fn storage_does_not_adopt_an_unowned_nonempty_directory() {
    let directory = tempfile::tempdir().unwrap();
    let settings = config(directory.path());
    std::fs::create_dir_all(&settings.data_dir).unwrap();
    std::fs::write(settings.data_dir.join("unrelated.txt"), "keep me").unwrap();
    assert!(KnowledgeStore::open(&settings).await.is_err());
    assert_eq!(
        std::fs::read_to_string(settings.data_dir.join("unrelated.txt")).unwrap(),
        "keep me"
    );
}
