#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/matching.rs"]
mod fixture;
use branch_backend::{contracts::library::*, knowledge::matching::GraphBounds};
#[tokio::test]
async fn symmetric_candidates_are_unique_and_dismissed_pairs_stay_dismissed() {
    let s = fixture::Setup::new().await;
    let (_, _, a) = s.create("How does NAND flash store data?").await;
    let (_, _, b) = s.create("How is data represented in NAND flash?").await;
    let candidate = s.candidate(&a, &b).await;
    let saved = s
        .store
        .save_candidates(std::slice::from_ref(&candidate))
        .await
        .unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].status, ConnectionStatus::Candidate);
    let page = s.store.connections(&a, None, 5).await.unwrap();
    let view = serde_json::to_value(&page.items[0]).unwrap();
    assert!(view["fromQuestion"].as_str().is_some_and(|q| !q.is_empty()));
    assert!(view["toQuestion"].as_str().is_some_and(|q| !q.is_empty()));

    let reversed = s.candidate(&b, &a).await;
    assert_eq!(candidate.id, reversed.id);
    s.store.save_candidates(&[reversed]).await.unwrap();
    assert_eq!(
        s.store
            .connections(&a, None, 100)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    let complete = s
        .store
        .neighborhood(
            &a,
            GraphBounds {
                hops: 1,
                nodes: 100,
                edges: 200,
            },
        )
        .await
        .unwrap();
    assert!(!complete.truncated);
    let dismiss = fixture::command(
        DismissConnection {
            connection_id: candidate.id.clone(),
        },
        None,
    );
    s.store.dismiss_connection(&dismiss).await.unwrap();
    assert_eq!(
        s.store.dismiss_connection(&dismiss).await.unwrap().state,
        OperationState::Completed
    );
    let saved = s.store.save_candidates(&[candidate]).await.unwrap();
    assert_eq!(saved[0].status, ConnectionStatus::Dismissed);
    assert!(
        s.store
            .connections(&a, None, 5)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn cycle_walk_respects_bounds() {
    let s = fixture::Setup::new().await;
    let mut nodes = vec![];
    for n in 0..103 {
        nodes.push(s.create(&format!("Graph node {n}")).await.2);
    }
    for n in 1..nodes.len() {
        s.store
            .save_candidates(&[s.candidate(&nodes[0], &nodes[n]).await])
            .await
            .unwrap();
    }
    for a in 1..22 {
        for b in a + 1..22 {
            s.store
                .save_candidates(&[s.candidate(&nodes[a], &nodes[b]).await])
                .await
                .unwrap();
        }
    }
    let walk = s
        .store
        .neighborhood(
            &nodes[0],
            GraphBounds {
                hops: 3,
                nodes: 100,
                edges: 200,
            },
        )
        .await
        .unwrap();
    assert!(walk.nodes.len() <= 100);
    assert!(walk.edges.len() <= 200);
    assert!(walk.truncated);
    let ids = walk
        .nodes
        .iter()
        .map(|n| n.id.clone())
        .collect::<std::collections::HashSet<_>>();
    assert!(
        walk.edges
            .iter()
            .all(|e| ids.contains(&e.from_topic_id) && ids.contains(&e.to_topic_id))
    );
    assert!(
        s.store
            .neighborhood(
                &nodes[0],
                GraphBounds {
                    hops: 4,
                    nodes: 100,
                    edges: 200
                }
            )
            .await
            .is_err()
    );
    assert!(s.store.connections(&nodes[0], None, 101).await.is_err());
    s.store.close().await.unwrap();
}

#[tokio::test]
async fn vector_evidence_upgrades_a_keyword_only_candidate() {
    let s = fixture::Setup::new().await;
    let (_, _, a) = s.create("How does NAND flash store data?").await;
    let (_, _, b) = s.create("How is data represented in NAND flash?").await;
    let mut candidate = s.candidate(&a, &b).await;
    candidate.provenance[0].vector_score = None;
    candidate.provenance[0].vector_rank = None;
    candidate.provenance[0].keyword_rank = Some(1);
    assert_eq!(
        s.store
            .save_candidates(std::slice::from_ref(&candidate))
            .await
            .unwrap()[0]
            .score,
        None
    );
    candidate.provenance[0].vector_score = Some(0.9);
    candidate.provenance[0].vector_rank = Some(1);
    assert_eq!(
        s.store.save_candidates(&[candidate]).await.unwrap()[0].score,
        Some(0.9)
    );
    s.store.close().await.unwrap();
}
