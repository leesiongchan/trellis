#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::{
    contracts::library::OperationId,
    error::ErrorCode,
    storage::{
        KnowledgeStore, StorageConfig,
        operations::{ClaimOutcome, OperationIntent},
    },
};
use std::sync::Arc;
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn operation_claim_is_atomic() {
    let root = tempfile::tempdir().unwrap();
    let store = Arc::new(
        KnowledgeStore::open(&StorageConfig {
            data_dir: root.path().join("data"),
            backup_dir: root.path().join("backups"),
            model_dir: root.path().join("models"),
        })
        .await
        .unwrap(),
    );
    store.install_schema().await.unwrap();
    store.install_schema().await.unwrap();
    let intent = OperationIntent::new(
        OperationId::new("op-one").unwrap(),
        1,
        serde_json::json!({"question":"Memory?"}),
        "create",
    )
    .unwrap();
    let mut handles = vec![];
    for _ in 0..12 {
        let store = store.clone();
        let intent = intent.clone();
        handles.push(tokio::spawn(
            async move { store.claim_operation(&intent).await },
        ));
    }
    let (mut claimed, mut replayed) = (0, 0);
    for handle in handles {
        match handle.await.unwrap().unwrap() {
            ClaimOutcome::Claimed(_) => claimed += 1,
            ClaimOutcome::Replay(_) => replayed += 1,
        }
    }
    assert_eq!(claimed, 1);
    assert_eq!(replayed, 11);
    assert_eq!(
        store.operation(&intent.id).await.unwrap().state,
        branch_backend::contracts::library::OperationState::Pending
    );
    let mut changed = intent.clone();
    changed.request_hash = "different".into();
    assert_eq!(
        store.claim_operation(&changed).await.unwrap_err().kind,
        ErrorCode::IntentConflict
    );
    changed = intent.clone();
    changed.library_epoch = 0;
    assert_eq!(
        store.claim_operation(&changed).await.unwrap_err().kind,
        ErrorCode::RevisionConflict
    );
    store.close().await.unwrap();
    assert_eq!(
        store.operation(&intent.id).await.unwrap_err().kind,
        ErrorCode::StorageUnavailable
    );
}
