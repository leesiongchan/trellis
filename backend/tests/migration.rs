#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::{
    contracts::{legacy::LegacyExploration, library::*},
    storage::{KnowledgeStore, StorageConfig},
};
use serde_json::{Value, json};
use std::sync::Arc;
struct Setup {
    _root: tempfile::TempDir,
    store: Arc<KnowledgeStore>,
}
impl Setup {
    async fn new() -> Self {
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
        Self { _root: root, store }
    }
}
fn command<T>(payload: T, epoch: u64) -> Command<T> {
    Command {
        operation_id: OperationId::generate(),
        library_epoch: epoch,
        expected_revision: None,
        payload,
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/library/migration.json")).unwrap()
}
async fn begin(s: &Setup, value: &Value, epoch: u64) -> (ImportView, String) {
    let encoded = value.to_string();
    let legacy = LegacyExploration::validate(value.clone()).unwrap();
    let view = s
        .store
        .begin_import(&command(
            BeginImport {
                legacy_exploration_id: value["id"].as_str().unwrap().into(),
                content_digest: legacy.digest(),
                expected_bytes: encoded.len() as u64,
                expected_nodes: value["nodes"].as_object().unwrap().len() as u64,
                explicit_restore: false,
            },
            epoch,
        ))
        .await
        .unwrap();
    (view, encoded)
}
async fn stage(s: &Setup, view: &ImportView, encoded: &str, epoch: u64) {
    s.store
        .stage_import(&command(
            ImportBatch {
                import_id: view.id.clone(),
                batch_index: 0,
                records: vec![json!(encoded)],
            },
            epoch,
        ))
        .await
        .unwrap();
}
async fn finish(s: &Setup, view: &ImportView, epoch: u64) -> ImportView {
    s.store
        .finalize_import(&command(
            FinalizeImport {
                import_id: view.id.clone(),
            },
            epoch,
        ))
        .await
        .unwrap()
}
#[tokio::test]
async fn import_is_atomic_and_idempotent() {
    let s = Setup::new().await;
    let raw = fixture();
    let (v, encoded) = begin(&s, &raw, 1).await;
    stage(&s, &v, &encoded, 1).await;
    let visible_records_before_finalize =
        s.store.explorations(None, 100).await.unwrap().items.len();
    assert_eq!(visible_records_before_finalize, 0);
    assert_eq!(
        s.store.import_status(&v.id).await.unwrap().state,
        ImportState::Staging
    );
    let first_import = finish(&s, &v, 1).await;
    let snapshot = s
        .store
        .exploration(first_import.exploration_id.as_ref().unwrap())
        .await
        .unwrap();
    assert_eq!(snapshot.appearances.len(), 3);
    assert_eq!(snapshot.answers.len(), 3);
    assert_eq!(snapshot.exploration.created_at, raw["createdAt"]);
    assert_eq!(snapshot.exploration.updated_at, raw["updatedAt"]);
    for (id, n) in raw["nodes"].as_object().unwrap() {
        let a = snapshot
            .appearances
            .iter()
            .find(|a| a.id.as_str() == first_import.mapping[id])
            .unwrap();
        assert_eq!(a.question, n["question"]);
        assert_eq!(a.teaser, n["teaser"]);
        assert_eq!(
            a.parent_id.as_ref().map(|p| p.as_str()),
            n["parentId"]
                .as_str()
                .map(|p| first_import.mapping[p].as_str())
        );
        let answer = snapshot
            .answers
            .iter()
            .find(|v| Some(&v.id) == a.answer_version_id.as_ref())
            .unwrap();
        assert_eq!(serde_json::to_value(&answer.content).unwrap(), n["content"]);
        assert_eq!(answer.origin, AnswerOrigin::Legacy);
    }
    let mut other = raw.clone();
    other["revision"] = json!(999);
    other["updatedAt"] = json!("2026-10-26T00:00:00.000Z");
    other["view"]["viewport"]["x"] = json!(12.5);
    other["nodes"]["root"]["isRefreshing"] = json!(true);
    let (repeated_import, _) = begin(&s, &other, 1).await;
    assert_eq!(repeated_import.mapping, first_import.mapping);
    assert_eq!(finish(&s, &v, 1).await.mapping, first_import.mapping);
    assert_eq!(
        s.store.explorations(None, 100).await.unwrap().items.len(),
        1
    );
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn changed_legacy_copy_is_preserved() {
    let s = Setup::new().await;
    let raw = fixture();
    let (a, x) = begin(&s, &raw, 1).await;
    stage(&s, &a, &x, 1).await;
    let a = finish(&s, &a, 1).await;
    let mut changed = raw;
    changed["nodes"]["root"]["content"]["summary"][0]["text"] =
        json!("Changed on another legacy device");
    let (b, x) = begin(&s, &changed, 1).await;
    stage(&s, &b, &x, 1).await;
    let b = finish(&s, &b, 1).await;
    assert_ne!(a.exploration_id, b.exploration_id);
    assert_ne!(a.mapping, b.mapping);
    assert_eq!(
        s.store.explorations(None, 100).await.unwrap().items.len(),
        2
    );
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn batches_conflicts_validation_and_concurrency_are_bounded() {
    let s = Setup::new().await;
    let raw = fixture();
    let (a, x) = begin(&s, &raw, 1).await;
    stage(&s, &a, &x, 1).await;
    stage(&s, &a, &x, 1).await;
    assert!(
        s.store
            .stage_import(&command(
                ImportBatch {
                    import_id: a.id.clone(),
                    batch_index: 0,
                    records: vec![json!("different")]
                },
                1
            ))
            .await
            .is_err()
    );
    let mut second = raw.clone();
    second["id"] = json!("second");
    let (b, _) = begin(&s, &second, 1).await;
    let mut third = second.clone();
    third["id"] = json!("third");
    let encoded = third.to_string();
    assert!(
        s.store
            .begin_import(&command(
                BeginImport {
                    legacy_exploration_id: "third".into(),
                    content_digest: LegacyExploration::validate(third).unwrap().digest(),
                    expected_bytes: encoded.len() as u64,
                    expected_nodes: 3,
                    explicit_restore: false
                },
                1
            ))
            .await
            .is_err()
    );
    assert_eq!(finish(&s, &b, 1).await.state, ImportState::Failed);
    assert_eq!(
        s.store.explorations(None, 100).await.unwrap().items.len(),
        0
    );
    assert_eq!(finish(&s, &a, 1).await.state, ImportState::Completed);
    let mut demo = raw.clone();
    demo["mode"] = json!("demo");
    assert!(LegacyExploration::validate(demo).is_err());
    let mut invalid = raw;
    invalid["nodes"]["root"]["childIds"] = json!([]);
    assert!(LegacyExploration::validate(invalid).is_err());
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn deletion_and_purge_suppress_automatic_imports_including_unknown_copies() {
    let s = Setup::new().await;
    let raw = fixture();
    let (a, x) = begin(&s, &raw, 1).await;
    stage(&s, &a, &x, 1).await;
    let a = finish(&s, &a, 1).await;
    let mut delete = command(
        DeleteExploration {
            exploration_id: a.exploration_id.unwrap(),
        },
        1,
    );
    delete.expected_revision = Some(0);
    s.store.delete_exploration(&delete).await.unwrap();
    assert_eq!(begin(&s, &raw, 1).await.0.state, ImportState::Suppressed);
    s.store
        .purge_library(&command(
            PurgeLibrary {
                confirmation: PURGE_CONFIRMATION.into(),
            },
            1,
        ))
        .await
        .unwrap();
    assert!(s.store.import_status(&a.id).await.is_err());
    assert_eq!(begin(&s, &raw, 2).await.0.state, ImportState::Suppressed);
    let mut unknown = raw;
    unknown["id"] = json!("never-connected-before");
    assert_eq!(
        begin(&s, &unknown, 2).await.0.state,
        ImportState::Suppressed
    );
    assert!(
        s.store
            .finalize_import(&command(FinalizeImport { import_id: a.id }, 1))
            .await
            .is_err()
    );
    s.store.close().await.unwrap();
}
#[test]
fn legacy_unicode_and_digest_ignore_only_device_state() {
    let mut raw = fixture();
    raw["nodes"]["root"]["question"] = json!("😀".repeat(500));
    let legacy = LegacyExploration::validate(raw.clone()).unwrap();
    let d = legacy.digest();
    raw["view"]["viewport"]["x"] = json!(77.7);
    raw["revision"] = json!(99);
    raw["nodes"]["root"]["status"] = json!("researching");
    assert_eq!(
        LegacyExploration::validate(raw.clone()).unwrap().digest(),
        d
    );
    raw["nodes"]["root"]["question"] = json!("😀".repeat(501));
    assert!(LegacyExploration::validate(raw).is_err());
}
#[test]
fn import_contract_byte_and_node_boundaries() {
    let begin = BeginImport {
        legacy_exploration_id: "legacy".into(),
        content_digest: "a".repeat(64),
        expected_bytes: 32 * 1024 * 1024,
        expected_nodes: 2000,
        explicit_restore: false,
    };
    let mut value = serde_json::to_value(command(begin, 1)).unwrap();
    assert!(Command::<BeginImport>::from_json(value.clone()).is_ok());
    value["payload"]["expectedBytes"] = json!(32 * 1024 * 1024 + 1);
    assert!(Command::<BeginImport>::from_json(value.clone()).is_err());
    value["payload"]["expectedBytes"] = json!(1);
    value["payload"]["expectedNodes"] = json!(2001);
    assert!(Command::<BeginImport>::from_json(value).is_err());
    let batch = ImportBatch {
        import_id: ImportId::generate(),
        batch_index: 0,
        records: vec![json!("x".repeat(2 * 1024 * 1024 - 4))],
    };
    let mut value = serde_json::to_value(command(batch, 1)).unwrap();
    assert!(Command::<ImportBatch>::from_json(value.clone()).is_ok());
    value["payload"]["records"][0] = json!("x".repeat(2 * 1024 * 1024 - 3));
    assert!(Command::<ImportBatch>::from_json(value).is_err());
}

#[test]
fn digest_matches_browser_fixture() {
    assert_eq!(
        LegacyExploration::validate(fixture()).unwrap().digest(),
        "9508538a67a945ddecd250143319ab11f85dc9150ae54b4205d0fc2fdec68132"
    );
}

fn many_nodes(count: usize, large: bool) -> Value {
    let mut value = fixture();
    let template = value["nodes"]["root"].clone();
    let mut nodes = serde_json::Map::new();
    for index in 0..count {
        let key = format!("node-{index}");
        let mut node = template.clone();
        node["id"] = json!(key);
        node["parentId"] = if index == 0 {
            Value::Null
        } else {
            json!("node-0")
        };
        node["question"] = json!(format!("Question {index}"));
        node["childIds"] = if index == 0 {
            json!((1..count).map(|i| format!("node-{i}")).collect::<Vec<_>>())
        } else {
            json!([])
        };
        node["teaser"] = json!("");
        if large {
            node["content"]["summary"] = json!(
                (0..4)
                    .map(|i| {
                        let mut p = node["content"]["summary"][0].clone();
                        p["id"] = json!(format!("summary-{i}"));
                        p["text"] = json!("x".repeat(6000));
                        p
                    })
                    .collect::<Vec<_>>()
            );
            node["content"]["sections"][0]["paragraphs"][0]["text"] = json!("x".repeat(6000));
        } else {
            node["content"] = Value::Null;
            node["status"] = json!("suggested");
        }
        nodes.insert(key, node);
    }
    value["nodes"] = Value::Object(nodes);
    value["rootId"] = json!("node-0");
    value["view"]["selectedNodeId"] = json!("node-0");
    value["view"]["expandedNodeIds"] = json!(["node-0"]);
    value
}
#[tokio::test]
async fn two_thousand_nodes_publish_and_one_more_is_rejected() {
    let s = Setup::new().await;
    let raw = many_nodes(2000, false);
    let (v, encoded) = begin(&s, &raw, 1).await;
    stage(&s, &v, &encoded, 1).await;
    let done = finish(&s, &v, 1).await;
    assert_eq!(done.mapping.len(), 2000);
    assert_eq!(
        s.store
            .exploration(done.exploration_id.as_ref().unwrap())
            .await
            .unwrap()
            .appearances
            .len(),
        2000
    );
    assert!(LegacyExploration::validate(many_nodes(2001, false)).is_err());
    s.store.close().await.unwrap();
}
#[tokio::test]
#[ignore = "Explicit 32 MiB embedded-engine maintenance and peak-memory gate"]
async fn thirty_two_mib_native_finalization_is_atomic_and_bounded() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .try_init();
    let s = Setup::new().await;
    let mut raw = many_nodes(1000, true);
    let mut remaining = 32 * 1024 * 1024 - raw.to_string().len();
    for node in raw["nodes"].as_object_mut().unwrap().values_mut() {
        let take = remaining.min(6000);
        node["teaser"] = json!("x".repeat(take));
        remaining -= take;
    }
    assert_eq!(remaining, 0);
    assert_eq!(raw.to_string().len(), 32 * 1024 * 1024);
    let (v, encoded) = begin(&s, &raw, 1).await;
    for (index, chunk) in encoded.as_bytes().chunks(512 * 1024).enumerate() {
        s.store
            .stage_import(&command(
                ImportBatch {
                    import_id: v.id.clone(),
                    batch_index: index as u64,
                    records: vec![json!(std::str::from_utf8(chunk).unwrap())],
                },
                1,
            ))
            .await
            .unwrap();
    }
    assert!(
        s.store
            .explorations(None, 100)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let start = std::time::Instant::now();
    let done = finish(&s, &v, 1).await;
    let elapsed = start.elapsed();
    assert_eq!(done.state, ImportState::Completed);
    assert!(elapsed < std::time::Duration::from_secs(30));
    assert_eq!(done.mapping.len(), 1000);
    println!("atomic finalization completed in {elapsed:?}; reading snapshot next");
    let snapshot = s
        .store
        .exploration(done.exploration_id.as_ref().unwrap())
        .await
        .unwrap();
    assert_eq!(snapshot.answers.len(), 1000);
    println!(
        "32MiB native finalization: {elapsed:?}, appearances={}, answers={}",
        snapshot.appearances.len(),
        snapshot.answers.len()
    );
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn explicit_restore_creates_a_new_preserved_copy_after_purge() {
    let s = Setup::new().await;
    let raw = fixture();
    let (a, x) = begin(&s, &raw, 1).await;
    stage(&s, &a, &x, 1).await;
    let first = finish(&s, &a, 1).await;
    s.store
        .purge_library(&command(
            PurgeLibrary {
                confirmation: PURGE_CONFIRMATION.into(),
            },
            1,
        ))
        .await
        .unwrap();
    let (blocked, _) = begin(&s, &raw, 2).await;
    assert_eq!(blocked.state, ImportState::Suppressed);
    let restore = s
        .store
        .begin_import(&command(
            BeginImport {
                legacy_exploration_id: raw["id"].as_str().unwrap().into(),
                content_digest: LegacyExploration::validate(raw.clone()).unwrap().digest(),
                expected_bytes: x.len() as u64,
                expected_nodes: 3,
                explicit_restore: true,
            },
            2,
        ))
        .await
        .unwrap();
    assert_eq!(restore.state, ImportState::Staging);
    stage(&s, &restore, &x, 2).await;
    let restored = finish(&s, &restore, 2).await;
    assert_ne!(restored.mapping, first.mapping);
    assert_eq!(restored.state, ImportState::Completed);
    assert_eq!(begin(&s, &raw, 2).await.0.mapping, restored.mapping);
    s.store.close().await.unwrap();
}

#[tokio::test]
async fn concurrent_origins_share_one_upload_owner_and_one_publication() {
    let s = Setup::new().await;
    let raw = fixture();
    let encoded = raw.to_string();
    let input = BeginImport {
        legacy_exploration_id: raw["id"].as_str().unwrap().into(),
        content_digest: LegacyExploration::validate(raw).unwrap().digest(),
        expected_bytes: encoded.len() as u64,
        expected_nodes: 3,
        explicit_restore: false,
    };
    let left = command(input.clone(), 1);
    let right = command(input, 1);
    let (a, b) = tokio::join!(s.store.begin_import(&left), s.store.begin_import(&right));
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.id, b.id);
    assert_eq!(a.upload_operation_id, b.upload_operation_id);
    assert!([left.operation_id, right.operation_id].contains(&a.upload_operation_id));
    stage(&s, &a, &encoded, 1).await;
    let (a, b) = tokio::join!(finish(&s, &a, 1), finish(&s, &b, 1));
    assert_eq!(a.mapping, b.mapping);
    assert_eq!(
        s.store.explorations(None, 100).await.unwrap().items.len(),
        1
    );
    assert_eq!(
        s.store
            .exploration(a.exploration_id.as_ref().unwrap())
            .await
            .unwrap()
            .answers
            .len(),
        3
    );
    s.store.close().await.unwrap();
}

#[path = "support/saved_provider.rs"]
mod provider_fixture;
#[tokio::test]
async fn valid_v1_unicode_questions_can_reopen_and_refresh_after_import() {
    use branch_backend::{config::Limits, jobs::Coordinator, knowledge::KnowledgeService};
    use std::sync::atomic::Ordering;
    let s = Setup::new().await;
    let mut raw = fixture();
    raw["nodes"]["root"]["question"] = json!("😀".repeat(500));
    let (v, x) = begin(&s, &raw, 1).await;
    stage(&s, &v, &x, 1).await;
    let done = finish(&s, &v, 1).await;
    let appearance = AppearanceId::new(done.mapping["root"].clone()).unwrap();
    let exploration = done.exploration_id.unwrap();
    let provider = provider_fixture::Provider::new();
    let service = KnowledgeService::new(
        s.store.clone(),
        Coordinator::new(provider.clone(), Limits::default()),
        provider.clone(),
    );
    let mut open = command(
        ResearchInput {
            appearance_id: appearance.clone(),
            refresh: false,
            clarification: None,
        },
        1,
    );
    open.expected_revision = Some(0);
    assert_eq!(
        service.start_research(open).await.unwrap().state,
        OperationState::Completed
    );
    assert_eq!(provider.searches.load(Ordering::SeqCst), 0);
    let mut refresh = command(
        ResearchInput {
            appearance_id: appearance,
            refresh: true,
            clarification: None,
        },
        1,
    );
    refresh.expected_revision = Some(
        s.store
            .exploration(&exploration)
            .await
            .unwrap()
            .exploration
            .revision,
    );
    service.start_research(refresh.clone()).await.unwrap();
    provider.release.add_permits(1);
    service.drain().await.unwrap();
    assert_eq!(
        s.store
            .operation(&refresh.operation_id)
            .await
            .unwrap()
            .state,
        OperationState::Completed
    );
    assert_eq!(
        provider.requests.lock().unwrap()[0].question,
        "😀".repeat(500)
    );
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn tampered_staged_content_fails_closed_and_preserves_the_original_route() {
    let s = Setup::new().await;
    let raw = fixture();
    let digest = LegacyExploration::validate(raw.clone()).unwrap().digest();
    let mut bad = raw.clone();
    bad["nodes"]["root"]["question"] = json!("Tampered question");
    let encoded = bad.to_string();
    let input = BeginImport {
        legacy_exploration_id: raw["id"].as_str().unwrap().into(),
        content_digest: digest,
        expected_bytes: encoded.len() as u64,
        expected_nodes: 3,
        explicit_restore: false,
    };
    let v = s
        .store
        .begin_import(&command(input.clone(), 1))
        .await
        .unwrap();
    stage(&s, &v, &encoded, 1).await;
    assert_eq!(finish(&s, &v, 1).await.state, ImportState::Failed);
    assert!(
        s.store
            .explorations(None, 100)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let mut restore = input;
    restore.expected_bytes = raw.to_string().len() as u64;
    restore.explicit_restore = true;
    let retried = s.store.begin_import(&command(restore, 1)).await.unwrap();
    assert_ne!(retried.id, v.id);
    stage(&s, &retried, &raw.to_string(), 1).await;
    assert_eq!(finish(&s, &retried, 1).await.state, ImportState::Completed);
    s.store.close().await.unwrap();
}
