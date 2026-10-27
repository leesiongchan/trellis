#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/saved_provider.rs"]
mod provider_fixture;
use branch_backend::{
    config::Config,
    contracts::{legacy::LegacyExploration, library::*},
    jobs::Coordinator,
    knowledge::{KnowledgeRuntime, KnowledgeService},
    storage::{KnowledgeStore, StorageConfig},
};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use tokio_util::sync::CancellationToken;
fn command<T>(payload: T) -> Command<T> {
    Command {
        operation_id: OperationId::generate(),
        library_epoch: 1,
        expected_revision: None,
        payload,
    }
}
async fn open(root: &std::path::Path) -> Arc<KnowledgeStore> {
    let s = Arc::new(
        KnowledgeStore::open(&StorageConfig {
            data_dir: root.join("data"),
            backup_dir: root.join("backups"),
            model_dir: root.join("models"),
            asset_dir: root.join("public"),
        })
        .await
        .unwrap(),
    );
    s.install_schema().await.unwrap();
    s
}
#[tokio::test]
async fn runtime_is_ready_with_missing_model_and_never_restarts_paid_work() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("branch_backend=debug")
        .try_init();
    let root = tempfile::tempdir().unwrap();
    let config = Arc::new(
        Config::from_values(
            &HashMap::from([
                ("APP_ORIGIN".into(), "http://127.0.0.1:3000".into()),
                (
                    "APP_DATA_DIR".into(),
                    root.path().join("data").display().to_string(),
                ),
            ]),
            &root.path().join("release/app"),
        )
        .unwrap(),
    );
    let store = open(root.path()).await;
    let p = provider_fixture::Provider::new();
    let service = Arc::new(KnowledgeService::new(
        store.clone(),
        Coordinator::new(p.clone(), config.limits.clone()),
        p.clone(),
    ));
    let runtime = KnowledgeRuntime::start(
        config,
        store.clone(),
        service,
        None,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(runtime.status().await.storage_ready);
    assert_eq!(
        store.library_info().await.unwrap().discovery,
        DiscoveryStatus::Unavailable
    );
    assert_eq!(p.searches.load(std::sync::atomic::Ordering::SeqCst), 0);
    runtime.shutdown().await.unwrap();
}
struct OwnedChild(std::process::Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[tokio::test]
async fn acknowledged_nested_save_survives_sigkill_and_recovery_is_provider_free() {
    let root = tempfile::tempdir().unwrap();
    let mut child = OwnedChild(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "durability_child", "--nocapture"])
            .env("BRANCH_DURABILITY_ROOT", root.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        while !root.path().join("ack.json").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let ack: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("ack.json")).unwrap()).unwrap();
    let store = open(root.path()).await;
    let recovery = store.recover_interrupted().await.unwrap();
    assert_eq!(recovery.interrupted_attempts, 1);
    assert_eq!(recovery.interrupted_verifications, 1);
    let saved = store
        .exploration(&ExplorationId::new(ack["explorationId"].as_str().unwrap()).unwrap())
        .await
        .unwrap();
    assert_eq!(saved.answers.len(), 3);
    let root_topic = &saved
        .appearances
        .iter()
        .find(|a| a.id == saved.exploration.root_id)
        .unwrap()
        .topic_id;
    assert_eq!(
        store.topic(root_topic, None).await.unwrap().versions.len(),
        2
    );
    assert_eq!(
        store
            .operation(&OperationId::new(ack["completedOperationId"].as_str().unwrap()).unwrap())
            .await
            .unwrap()
            .state,
        OperationState::Completed
    );
    assert_eq!(
        serde_json::to_value(&saved.appearances).unwrap(),
        ack["appearances"]
    );
    assert_eq!(
        saved.answers[0].content.sources[0].url,
        "https://example.org/memory"
    );
    let operation = store
        .operation(&OperationId::new(ack["operationId"].as_str().unwrap()).unwrap())
        .await
        .unwrap();
    assert_eq!(operation.state, OperationState::Interrupted);
    let p = provider_fixture::Provider::new();
    let automatic_provider_calls_after_restart =
        p.searches.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(automatic_provider_calls_after_restart, 0);
    assert_eq!(
        store
            .recover_interrupted()
            .await
            .unwrap()
            .interrupted_attempts,
        0
    );
    store.close().await.unwrap();
}
#[tokio::test]
#[ignore = "SIGKILL durability child, only spawned by the parent test"]
async fn durability_child() {
    let root = std::path::PathBuf::from(std::env::var("BRANCH_DURABILITY_ROOT").unwrap());
    let store = open(&root).await;
    let raw: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/library/migration.json")).unwrap();
    let legacy = LegacyExploration::validate(raw.clone()).unwrap();
    let encoded = raw.to_string();
    let v = store
        .begin_import(&command(BeginImport {
            legacy_exploration_id: legacy.id.clone(),
            content_digest: legacy.digest(),
            expected_bytes: encoded.len() as u64,
            expected_nodes: 3,
            explicit_restore: false,
        }))
        .await
        .unwrap();
    store
        .stage_import(&command(ImportBatch {
            import_id: v.id.clone(),
            batch_index: 0,
            records: vec![json!(encoded)],
        }))
        .await
        .unwrap();
    let done = store
        .finalize_import(&command(FinalizeImport { import_id: v.id }))
        .await
        .unwrap();

    use branch_backend::{
        embeddings::{EmbeddingWorker, ModelManifest},
        knowledge::matching::MatchingService,
    };
    use sha2::{Digest, Sha256};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let bytes = include_bytes!("../../tests/fixtures/library/test-tokenizer.json");
    std::fs::write(root.join("tokenizer.json"), bytes).unwrap();
    let mut manifest = ModelManifest::pinned().unwrap();
    manifest.artifact_hashes = std::collections::BTreeMap::from([(
        "tokenizer.json".into(),
        format!("{:x}", Sha256::digest(bytes)),
    )]);
    let embedder = Arc::new(matching_fixture::FakeEmbedder {
        manifest: manifest.with_tokenizer(&root).unwrap(),
        calls: AtomicUsize::new(0),
    });
    let _worker = EmbeddingWorker::start(store.clone(), embedder.clone(), CancellationToken::new());
    while store.library_info().await.unwrap().discovery != DiscoveryStatus::Ready {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let p = provider_fixture::Provider::new();
    let verifier = Arc::new(BlockingVerifier(AtomicUsize::new(0)));
    let service = KnowledgeService::new(
        store.clone(),
        Coordinator::new(p.clone(), Default::default()),
        p.clone(),
    )
    .with_matching(
        Arc::new(MatchingService::new(store.clone(), embedder)),
        verifier.clone(),
    );
    let mut completed = command(ResearchInput {
        appearance_id: AppearanceId::new(done.mapping["root"].clone()).unwrap(),
        refresh: true,
        clarification: None,
    });
    completed.expected_revision = Some(0);
    service.start_research(completed.clone()).await.unwrap();
    p.release.add_permits(1);
    while verifier.0.load(Ordering::SeqCst) == 0 {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let exploration = done.exploration_id.unwrap();
    let snapshot = store.exploration(&exploration).await.unwrap();
    let mut pending = command(ResearchInput {
        appearance_id: AppearanceId::new(done.mapping["child-a"].clone()).unwrap(),
        refresh: true,
        clarification: None,
    });
    pending.expected_revision = Some(snapshot.exploration.revision);
    service.start_research(pending.clone()).await.unwrap();
    while p.searches.load(Ordering::SeqCst) < 2 {
        tokio::task::yield_now().await;
    }
    std::fs::write(root.join("ack.json"),json!({"explorationId":exploration,"operationId":pending.operation_id,"completedOperationId":completed.operation_id,"appearances":snapshot.appearances}).to_string()).unwrap();
    std::future::pending::<()>().await;
}
#[path = "support/matching.rs"]
mod matching_fixture;
struct BlockingVerifier(std::sync::atomic::AtomicUsize);
impl branch_backend::knowledge::verification::ConnectionVerifier for BlockingVerifier {
    fn verify<'a>(
        &'a self,
        _: &'a [branch_backend::knowledge::matching::MatchCandidate],
        _: &'a branch_backend::research::context::JobContext,
    ) -> branch_backend::providers::ProviderFuture<
        'a,
        branch_backend::knowledge::verification::ConnectionVerdict,
    > {
        Box::pin(async move {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::future::pending().await
        })
    }
}

#[tokio::test]
async fn backup_failure_is_visible_without_disabling_saved_library_reads() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("backups"), "not a directory").unwrap();
    let config = Arc::new(
        Config::from_values(
            &HashMap::from([
                ("APP_ORIGIN".into(), "http://127.0.0.1:3000".into()),
                (
                    "APP_DATA_DIR".into(),
                    root.path().join("data").display().to_string(),
                ),
            ]),
            &root.path().join("release/app"),
        )
        .unwrap(),
    );
    let store = open(root.path()).await;
    let provider = provider_fixture::Provider::new();
    let service = Arc::new(KnowledgeService::new(
        store.clone(),
        Coordinator::new(provider.clone(), config.limits.clone()),
        provider.clone(),
    ));
    let runtime = KnowledgeRuntime::start(config, store, service, None, CancellationToken::new())
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !runtime.status().await.backup_failed {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let status = runtime.status().await;
    assert!(status.storage_ready && status.library_ready && status.backup_overdue);
    assert_eq!(
        provider.searches.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    runtime.shutdown().await.unwrap();
}
