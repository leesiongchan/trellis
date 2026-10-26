#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::{
    contracts::{ResearchResult, library::*},
    embeddings::{
        Embedder, EmbeddingInput, InputKind, LocalEmbedder, ModelManifest, SearchVector,
        chunk_answer,
    },
    storage::{KnowledgeStore, StorageConfig},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
fn manifest() -> ModelManifest {
    serde_json::from_str(include_str!(
        "../../tests/fixtures/library/model-manifest.json"
    ))
    .unwrap()
}
fn content() -> branch_backend::contracts::TopicContent {
    let v: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/contracts/results.json")).unwrap();
    let ResearchResult::Topic { content } =
        ResearchResult::from_json(v[0]["input"].clone()).unwrap()
    else {
        unreachable!()
    };
    content
}
fn test_manifest() -> (tempfile::TempDir, ModelManifest) {
    let root = tempfile::tempdir().unwrap();
    let bytes = include_bytes!("../../tests/fixtures/library/test-tokenizer.json");
    std::fs::write(root.path().join("tokenizer.json"), bytes).unwrap();
    let mut m = manifest();
    m.artifact_hashes = BTreeMap::from([(
        "tokenizer.json".into(),
        format!("{:x}", Sha256::digest(bytes)),
    )]);
    let m = m.with_tokenizer(root.path()).unwrap();
    (root, m)
}
#[tokio::test]
async fn missing_or_corrupt_model_does_not_break_saves() {
    let root = tempfile::tempdir().unwrap();
    assert!(LocalEmbedder::load(&root.path().join("missing"), &manifest()).is_err());
    std::fs::create_dir(root.path().join("models")).unwrap();
    std::fs::write(root.path().join("models/model_optimized.onnx"), b"corrupt").unwrap();
    assert!(LocalEmbedder::load(&root.path().join("models"), &manifest()).is_err());
    let store = KnowledgeStore::open(&StorageConfig {
        data_dir: root.path().join("data"),
        backup_dir: root.path().join("backups"),
        model_dir: root.path().join("models"),
        asset_dir: root.path().join("public"),
    })
    .await
    .unwrap();
    store.install_schema().await.unwrap();
    let saved = store
        .create_exploration(&Command {
            operation_id: OperationId::generate(),
            library_epoch: 1,
            expected_revision: None,
            payload: CreateExploration {
                question: "How does flash store data?".into(),
                locale: "en".into(),
            },
        })
        .await
        .unwrap();
    assert_eq!(saved.state, OperationState::Completed);
    assert_eq!(
        store.library_info().await.unwrap().discovery,
        DiscoveryStatus::Unavailable
    );
    store.close().await.unwrap();
}
#[test]
fn chunking_preserves_locations_and_limits() {
    let (_root, manifest) = test_manifest();
    let mut c = content();
    c.summary[0].text = "flash ".repeat(1000);
    c.sections[0].paragraphs = vec![c.sections[0].paragraphs[0].clone(); 100];
    for (i, p) in c.sections[0].paragraphs.iter_mut().enumerate() {
        p.id = format!("long-{i}");
        p.text = "memory ".repeat(400);
    }
    let set = chunk_answer(&c, &manifest).unwrap();
    let chunks = &set.chunks;
    assert!(chunks.len() <= 64);
    assert_eq!(chunks.len(), 64);
    assert!(set.partial_coverage);
    assert_eq!(chunks[0].location.paragraph_id, "p1");
    assert!(chunks.iter().any(|c| c.location.section_index == Some(0)));
    for c in chunks {
        assert!(c.passage_tokens <= 350);
        let input_tokens = c.input_tokens;
        assert!(input_tokens <= manifest.max_tokens);
        assert_eq!(manifest.token_count(&c.input, true).unwrap(), input_tokens);
    }
}
#[test]
fn model_versions_do_not_mix() {
    let m = manifest();
    let version = m.version();
    assert!(SearchVector::new(version.clone(), vec![0.; 383], 384).is_err());
    assert!(SearchVector::new(version.clone(), vec![f32::NAN; 384], 384).is_err());
    assert!(SearchVector::new(version.clone(), vec![0.; 384], 384).is_err());
    let vector = SearchVector::new(version, vec![1.; 384], 384)
        .unwrap()
        .values;
    assert_eq!(vector.len(), 384);
    let mut other = m.clone();
    other.preprocessing_version.push_str("-next");
    assert_ne!(m.version(), other.version());
}
#[test]
#[ignore = "Explicit offline native gate; requires the pinned model and runtime assets"]
fn native_embedding_matches_pinned_public_fixture() {
    let root = std::env::var("BRANCH_EMBEDDING_MODEL_DIR").unwrap();
    let m = manifest();
    let embedder = LocalEmbedder::load(std::path::Path::new(&root), &m).unwrap();
    let fixture: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/library/embedding-cases.json"
    ))
    .unwrap();
    for (kind, cases) in [
        (InputKind::Document, &fixture["topics"]),
        (InputKind::Query, &fixture["queries"]),
    ] {
        for case in cases.as_array().unwrap().iter().take(2) {
            let result = embedder
                .embed(&[EmbeddingInput {
                    text: case["text"].as_str().unwrap().into(),
                    kind,
                }])
                .unwrap();
            let vector = &result[0];
            assert_eq!(vector.len(), 384);
            let expected = case["vector"].as_array().unwrap();
            let cosine: f64 = vector
                .iter()
                .zip(expected)
                .map(|(a, b)| *a as f64 * b.as_f64().unwrap())
                .sum();
            assert!(
                cosine > 0.995,
                "preprocessing parity: {}={cosine}",
                case["key"]
            );
        }
    }
    let chunks = chunk_answer(&content(), embedder.manifest()).unwrap();
    for c in chunks.chunks {
        assert!(c.input_tokens <= m.max_tokens);
    }
}

#[test]
fn token_budget_can_drop_context_completely() {
    let (_root, mut m) = test_manifest();
    m.max_tokens = 352;
    assert_eq!(m.truncate_input("flash memory", 0).unwrap(), "");
    let mut c = content();
    c.summary[0].text = "flash ".repeat(351);
    let set = chunk_answer(&c, &m).unwrap();
    assert_eq!(set.chunks[0].passage_tokens, 350);
    assert!(set.chunks[0].input_tokens <= 352);
}
