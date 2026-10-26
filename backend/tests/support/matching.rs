#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use branch_backend::{
    contracts::library::*,
    embeddings::*,
    knowledge::matching::{MatchCandidate, MatchSource},
    storage::{KnowledgeStore, StorageConfig},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio_util::sync::CancellationToken;
pub struct FakeEmbedder {
    pub manifest: ModelManifest,
    pub calls: AtomicUsize,
}
impl Embedder for FakeEmbedder {
    fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }
    fn embed(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(inputs
            .iter()
            .map(|_| {
                let mut v = vec![0.; 384];
                v[0] = 1.;
                v
            })
            .collect())
    }
}
pub struct Setup {
    pub root: tempfile::TempDir,
    pub store: Arc<KnowledgeStore>,
    pub embedder: Arc<FakeEmbedder>,
}
impl Setup {
    pub async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bytes = include_bytes!("../../../tests/fixtures/library/test-tokenizer.json");
        std::fs::write(root.path().join("tokenizer.json"), bytes).unwrap();
        let mut manifest = ModelManifest::pinned().unwrap();
        manifest.artifact_hashes = BTreeMap::from([(
            "tokenizer.json".into(),
            format!("{:x}", Sha256::digest(bytes)),
        )]);
        let manifest = manifest.with_tokenizer(root.path()).unwrap();
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
        Self {
            root,
            store,
            embedder: Arc::new(FakeEmbedder {
                manifest,
                calls: AtomicUsize::new(0),
            }),
        }
    }
    pub async fn create(&self, q: &str) -> (ExplorationId, AppearanceId, TopicId) {
        let saved = self
            .store
            .create_exploration(&command(
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
    pub async fn contextual(
        &self,
        q: &str,
        context: Option<&str>,
    ) -> (ExplorationId, AppearanceId, TopicId, u64) {
        if let Some(context) = context {
            let (e, a, _) = self.create(context).await;
            let saved = self
                .store
                .add_branch(&command(
                    AddBranch {
                        parent_appearance_id: a,
                        question: q.into(),
                    },
                    Some(0),
                ))
                .await
                .unwrap();
            let Some(OperationResult::Exploration { appearance_id, .. }) = saved.result else {
                unreachable!()
            };
            let topic = self
                .store
                .exploration(&e)
                .await
                .unwrap()
                .appearances
                .into_iter()
                .find(|v| v.id == appearance_id)
                .unwrap()
                .topic_id;
            (e, appearance_id, topic, 1)
        } else {
            let (e, a, t) = self.create(q).await;
            (e, a, t, 0)
        }
    }
    pub async fn source(&self, topic: &TopicId) -> EmbeddingSource {
        let mut cursor = None;
        loop {
            let page = self
                .store
                .embedding_sources(SourceKind::Question, cursor.as_deref(), 100)
                .await
                .unwrap();
            if let Some(source) = page.items.into_iter().find(|v| v.topic_id == *topic) {
                return source;
            }
            cursor = page.next_cursor;
            assert!(cursor.is_some());
        }
    }
    pub async fn candidate(&self, a: &TopicId, b: &TopicId) -> MatchCandidate {
        MatchCandidate::questions(
            &self.source(a).await,
            &self.source(b).await,
            &self.embedder.manifest.version(),
            0.95,
        )
        .unwrap()
    }
    pub async fn ready(&self) -> EmbeddingWorker {
        let worker = EmbeddingWorker::start(
            self.store.clone(),
            self.embedder.clone(),
            CancellationToken::new(),
        );
        until(async || {
            self.store.library_info().await.unwrap().discovery == DiscoveryStatus::Ready
        })
        .await;
        worker
    }
    pub fn question(topic: &TopicId) -> MatchSource {
        MatchSource::question(topic.clone(), 1)
    }
}
pub fn command<T>(payload: T, revision: Option<u64>) -> Command<T> {
    Command {
        operation_id: OperationId::generate(),
        library_epoch: 1,
        expected_revision: revision,
        payload,
    }
}
pub async fn until(mut test: impl AsyncFnMut() -> bool) {
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        while !test().await {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
