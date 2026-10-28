#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/matching.rs"]
mod fixture;
use branch_backend::{contracts::library::*, knowledge::matching::*};
#[tokio::test]
async fn local_discovery_connects_suggestions_without_a_provider() {
    let s = fixture::Setup::new().await;
    let (e, _, a) = s.create("How does NAND flash store data?").await;
    let (other, _, b) = s.create("How is data stored in NAND flash?").await;
    let worker = s.ready().await;
    let before = s.store.exploration(&e).await.unwrap();
    let before_other = s.store.exploration(&other).await.unwrap();
    let service = MatchingService::new(s.store.clone(), s.embedder.clone());
    let summary = service
        .discover(&fixture::Setup::question(&a))
        .await
        .unwrap();
    assert!(!summary.skipped);
    assert!(summary.candidates.len() <= 5);
    let edges = s.store.connections(&a, None, 5).await.unwrap().items;
    assert!(
        edges
            .iter()
            .any(|v| v.from_topic_id == b || v.to_topic_id == b)
    );
    assert!(
        edges
            .iter()
            .all(|v| v.status == ConnectionStatus::Candidate)
    );
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(s.store.exploration(&e).await.unwrap()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(before_other).unwrap(),
        serde_json::to_value(s.store.exploration(&other).await.unwrap()).unwrap()
    );
    worker.shutdown().await.unwrap();
    s.store.close().await.unwrap();
}

#[tokio::test]
async fn background_backfill_connects_unopened_questions_locally() {
    let s = fixture::Setup::new().await;
    let (e, _, a) = s.create("How does NAND flash store data?").await;
    let (_, _, b) = s.create("How is data stored in NAND flash?").await;
    let worker = s.ready().await;
    fixture::until(async || {
        s.store
            .connections(&a, None, 5)
            .await
            .unwrap()
            .items
            .iter()
            .any(|c| c.from_topic_id == b || c.to_topic_id == b)
    })
    .await;
    let saved = s.store.exploration(&e).await.unwrap();
    assert!(saved.answers.is_empty());
    assert!(
        saved
            .appearances
            .iter()
            .all(|a| a.answer_version_id.is_none())
    );
    assert!(
        s.store
            .connections(&a, None, 100)
            .await
            .unwrap()
            .items
            .iter()
            .all(|c| c.status == ConnectionStatus::Candidate)
    );
    worker.shutdown().await.unwrap();
    s.store.close().await.unwrap();
}
