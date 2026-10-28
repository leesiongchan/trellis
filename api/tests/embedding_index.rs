#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/saved_provider.rs"]
mod provider_fixture;
use branch_backend::{
    config::Limits,
    contracts::library::*,
    embeddings::*,
    error::ErrorCode,
    jobs::Coordinator,
    knowledge::KnowledgeService,
    storage::{KnowledgeStore, StorageConfig},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
struct Setup {
    _root: tempfile::TempDir,
    store: Arc<KnowledgeStore>,
    manifest: ModelManifest,
}
impl Setup {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bytes = include_bytes!("../../tests/fixtures/library/test-tokenizer.json");
        std::fs::write(root.path().join("tokenizer.json"), bytes).unwrap();
        let mut m = ModelManifest::pinned().unwrap();
        m.artifact_hashes =
            BTreeMap::from([("tokenizer.json".into(), hex::encode(Sha256::digest(bytes)))]);
        let manifest = m.with_tokenizer(root.path()).unwrap();
        let store = Arc::new(
            KnowledgeStore::open(&StorageConfig {
                data_dir: root.path().join("data"),
                backup_dir: root.path().join("backups"),
                model_dir: root.path().join("models"),
                asset_dir: root.path().join("public"),
            })
            .await
            .unwrap(),
        );
        store.install_schema().await.unwrap();
        Self {
            _root: root,
            store,
            manifest,
        }
    }
    async fn create(&self, q: &str) -> (ExplorationId, AppearanceId, TopicId) {
        let saved = self
            .store
            .create_exploration(&cmd(
                CreateExploration {
                    question: q.into(),
                    locale: "en".into(),
                },
                None,
            ))
            .await
            .unwrap();
        let Some(OperationResult::Exploration {
            exploration_id,
            appearance_id,
            ..
        }) = saved.result
        else {
            unreachable!()
        };
        let topic = self
            .store
            .exploration(&exploration_id)
            .await
            .unwrap()
            .appearances[0]
            .topic_id
            .clone();
        (exploration_id, appearance_id, topic)
    }
}
fn cmd<T>(payload: T, revision: Option<u64>) -> Command<T> {
    Command {
        operation_id: OperationId::generate(),
        library_epoch: 1,
        expected_revision: revision,
        payload,
    }
}
fn vector() -> Vec<f32> {
    let mut v = vec![0.; 384];
    v[0] = 1.;
    v
}
struct Fake {
    manifest: ModelManifest,
    calls: AtomicUsize,
    bad: bool,
    max_batch: AtomicUsize,
}
impl Fake {
    fn new(m: ModelManifest, bad: bool) -> Arc<Self> {
        Arc::new(Self {
            manifest: m,
            calls: AtomicUsize::new(0),
            bad,
            max_batch: AtomicUsize::new(0),
        })
    }
}
impl Embedder for Fake {
    fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }
    fn embed(&self, input: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.max_batch.fetch_max(input.len(), Ordering::SeqCst);
        Ok(input
            .iter()
            .map(|_| if self.bad { vec![0.; 3] } else { vector() })
            .collect())
    }
}
async fn until(mut condition: impl AsyncFnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !condition().await {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn public_ranking_cases_and_model_generations_remain_separate() {
    let s = Setup::new().await;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/library/embedding-cases.json"
    ))
    .unwrap();
    let mut names = HashMap::new();
    for case in fixture["topics"].as_array().unwrap() {
        let (_, _, topic) = s.create(case["text"].as_str().unwrap()).await;
        names.insert(topic.clone(), case["key"].as_str().unwrap().to_owned());
        let source = s
            .store
            .embedding_sources(SourceKind::Question, None, 100)
            .await
            .unwrap()
            .items
            .into_iter()
            .find(|v| v.topic_id == topic)
            .unwrap();
        let values = serde_json::from_value::<Vec<f32>>(case["vector"].clone()).unwrap();
        s.store
            .index_source(&source, &s.manifest, &[values])
            .await
            .unwrap();
    }
    for query in fixture["queries"].as_array().unwrap() {
        let input = SearchVector::new(
            s.manifest.version(),
            serde_json::from_value(query["vector"].clone()).unwrap(),
            384,
        )
        .unwrap();
        let hits = s
            .store
            .nearest(&input, &SearchFilter::questions(), 3)
            .await
            .unwrap();
        let expected = query["expected"].as_array().unwrap();
        assert!(
            hits.iter().any(|hit| expected
                .iter()
                .any(|key| names[&hit.topic_id] == key.as_str().unwrap())),
            "{}",
            query["key"]
        );
        assert!(hits.iter().all(|h| h.model_version == s.manifest.version()));
    }
    let source = s
        .store
        .embedding_sources(SourceKind::Question, None, 1)
        .await
        .unwrap()
        .items
        .remove(0);
    let mut next = s.manifest.clone();
    next.preprocessing_version.push_str("-next");
    s.store
        .index_source(&source, &next, &[vector()])
        .await
        .unwrap();
    let hits = s
        .store
        .nearest(
            &SearchVector::new(next.version(), vector(), 384).unwrap(),
            &SearchFilter::questions(),
            10,
        )
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert!(
        s.store
            .index_source(&source, &next, &[vec![1.; 383]])
            .await
            .is_err()
    );
    let invalid = SearchVector {
        model_version: s.manifest.version(),
        values: vec![f32::NAN; 384],
    };
    assert!(
        s.store
            .nearest(&invalid, &SearchFilter::questions(), 10)
            .await
            .is_err()
    );
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn derived_indexing_resumes_without_repeating_completed_sources() {
    let s = Setup::new().await;
    s.create("Memory").await;
    let first = Fake::new(s.manifest.clone(), false);
    let worker = EmbeddingWorker::start(s.store.clone(), first.clone(), CancellationToken::new());
    until(async || s.store.library_info().await.unwrap().discovery == DiscoveryStatus::Ready).await;
    worker.shutdown().await.unwrap();
    assert_eq!(first.calls.load(Ordering::SeqCst), 1);
    s.create("Flash").await;
    let second = Fake::new(s.manifest.clone(), false);
    let worker = EmbeddingWorker::start(s.store.clone(), second.clone(), CancellationToken::new());
    until(async || {
        second.calls.load(Ordering::SeqCst) > 0
            && s.store.library_info().await.unwrap().discovery == DiscoveryStatus::Ready
    })
    .await;
    worker.shutdown().await.unwrap();
    assert_eq!(second.calls.load(Ordering::SeqCst), 1);
    assert!(second.max_batch.load(Ordering::SeqCst) <= 32);
    let hits = s
        .store
        .nearest(
            &SearchVector::new(s.manifest.version(), vector(), 384).unwrap(),
            &SearchFilter::questions(),
            10,
        )
        .await
        .unwrap();
    assert_eq!(hits.len(), 2);
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn inference_failure_and_stale_generation_cannot_break_content_saves() {
    let s = Setup::new().await;
    s.create("Memory").await;
    let fake = Fake::new(s.manifest.clone(), true);
    let worker = EmbeddingWorker::start(s.store.clone(), fake.clone(), CancellationToken::new());
    until(async || {
        fake.calls.load(Ordering::SeqCst) > 0
            && s.store.library_info().await.unwrap().discovery == DiscoveryStatus::Unavailable
    })
    .await;
    assert!(worker.shutdown().await.is_err());
    s.create("Saved despite discovery failure").await;
    let source = s
        .store
        .embedding_sources(SourceKind::Question, None, 1)
        .await
        .unwrap()
        .items
        .remove(0);
    s.store
        .purge_library(&cmd(
            PurgeLibrary {
                confirmation: PURGE_CONFIRMATION.into(),
            },
            None,
        ))
        .await
        .unwrap();
    assert_eq!(
        s.store
            .index_source(&source, &s.manifest, &[vector()])
            .await
            .unwrap_err()
            .kind,
        ErrorCode::RevisionConflict
    );
    assert!(
        s.store
            .nearest(
                &SearchVector::new(s.manifest.version(), vector(), 384).unwrap(),
                &SearchFilter::questions(),
                10
            )
            .await
            .unwrap()
            .is_empty()
    );
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn passage_retrieval_includes_current_and_pinned_versions_only() {
    let s = Setup::new().await;
    let provider = provider_fixture::Provider::new();
    let service = KnowledgeService::new(
        s.store.clone(),
        Coordinator::new(provider.clone(), Limits::default()),
        provider.clone(),
    );
    let (_, appearance, _) = s.create("Memory").await;
    let mut versions = vec![];
    for revision in [0, 1] {
        let input = cmd(
            ResearchInput {
                appearance_id: appearance.clone(),
                refresh: revision > 0,
                clarification: None,
            },
            Some(revision),
        );
        service.start_research(input.clone()).await.unwrap();
        provider.release.add_permits(1);
        until(async || {
            s.store.operation(&input.operation_id).await.unwrap().state == OperationState::Completed
        })
        .await;
        if revision == 0 {
            let (e, _, _) = s.create("Memory").await;
            versions.push(e);
        }
    }
    let sources = s
        .store
        .embedding_sources(SourceKind::Passage, None, 100)
        .await
        .unwrap()
        .items;
    assert_eq!(sources.len(), 2);
    for source in &sources {
        let prepared = source.prepare(&s.manifest).unwrap();
        s.store
            .index_source(source, &s.manifest, &vec![vector(); prepared.inputs.len()])
            .await
            .unwrap();
    }
    let search = SearchVector::new(s.manifest.version(), vector(), 384).unwrap();
    let filter = SearchFilter::passages();
    let hits = s.store.nearest(&search, &filter, 10).await.unwrap();
    let version_ids = hits
        .iter()
        .filter_map(|h| h.answer_version_id.clone())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(version_ids.len(), 2);
    s.store
        .delete_exploration(&cmd(
            DeleteExploration {
                exploration_id: versions[0].clone(),
            },
            Some(0),
        ))
        .await
        .unwrap();
    let hits = s.store.nearest(&search, &filter, 10).await.unwrap();
    let version_ids = hits
        .iter()
        .filter_map(|h| h.answer_version_id.clone())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(version_ids.len(), 1);
    service.drain().await.unwrap();
    s.store.close().await.unwrap();
}

struct PausedEmbedder {
    manifest: ModelManifest,
    calls: AtomicUsize,
    release: std::sync::Mutex<bool>,
    wake: std::sync::Condvar,
}
impl Embedder for PausedEmbedder {
    fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }
    fn embed(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut ready = self.release.lock().unwrap();
        while !*ready {
            ready = self.wake.wait(ready).unwrap();
        }
        Ok(inputs.iter().map(|_| vector()).collect())
    }
}
#[tokio::test]
async fn cancelled_inference_does_not_checkpoint_uncommitted_sources() {
    let s = Setup::new().await;
    s.create("Memory").await;
    s.create("Flash").await;
    let paused = Arc::new(PausedEmbedder {
        manifest: s.manifest.clone(),
        calls: AtomicUsize::new(0),
        release: std::sync::Mutex::new(false),
        wake: std::sync::Condvar::new(),
    });
    let cancel = CancellationToken::new();
    let worker = EmbeddingWorker::start(s.store.clone(), paused.clone(), cancel.clone());
    until(async || paused.calls.load(Ordering::SeqCst) > 0).await;
    cancel.cancel();
    *paused.release.lock().unwrap() = true;
    paused.wake.notify_all();
    worker.shutdown().await.unwrap();
    let resumed = Fake::new(s.manifest.clone(), false);
    let worker = EmbeddingWorker::start(s.store.clone(), resumed.clone(), CancellationToken::new());
    until(async || s.store.library_info().await.unwrap().discovery == DiscoveryStatus::Ready).await;
    assert_eq!(resumed.calls.load(Ordering::SeqCst), 2);
    worker.shutdown().await.unwrap();
    s.store.close().await.unwrap();
}
