#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/matching.rs"]
mod fixture;
#[path = "support/saved_provider.rs"]
mod provider_fixture;
use branch_backend::{
    config::Limits,
    contracts::library::*,
    jobs::Coordinator,
    knowledge::{KnowledgeService, matching::*, verification::*},
    providers::ProviderFuture,
    research::context::JobContext,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Semaphore;
struct Verifier {
    calls: AtomicUsize,
    release: Semaphore,
    verdict: VerdictKind,
}
impl ConnectionVerifier for Verifier {
    fn verify<'a>(
        &'a self,
        candidates: &'a [MatchCandidate],
        _: &'a JobContext,
    ) -> ProviderFuture<'a, ConnectionVerdict> {
        Box::pin(async move {
            assert!(candidates.len() <= 3);
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.release.acquire().await.unwrap().forget();
            Ok(ConnectionVerdict {
                decisions: candidates
                    .iter()
                    .map(|c| ConnectionDecision {
                        candidate_id: c.id.clone(),
                        verdict: self.verdict,
                        reason: "Synthetic fixture judgment".into(),
                        paragraph_refs: vec![],
                    })
                    .collect(),
            })
        })
    }
}
#[tokio::test]
async fn matching_preserves_originals_and_hard_negatives() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/library/matching.json")).unwrap();
    let mut accepted_equivalence_edges_for_hard_negatives = 0;
    for case in cases.as_array().unwrap() {
        let s = fixture::Setup::new().await;
        let (e, a, left, revision) = s
            .contextual(case["left"].as_str().unwrap(), case["leftContext"].as_str())
            .await;
        let (other, _, right, _) = s
            .contextual(
                case["right"].as_str().unwrap(),
                case["rightContext"].as_str(),
            )
            .await;
        let worker = s.ready().await;
        let matching = Arc::new(MatchingService::new(s.store.clone(), s.embedder.clone()));
        let verifier = Arc::new(Verifier {
            calls: AtomicUsize::new(0),
            release: Semaphore::new(0),
            verdict: if case["verdict"] == "equivalent" {
                VerdictKind::Equivalent
            } else {
                VerdictKind::Different
            },
        });
        matching
            .discover(&fixture::Setup::question(&left))
            .await
            .unwrap();
        assert_eq!(verifier.calls.load(Ordering::SeqCst), 0);
        let provider = provider_fixture::Provider::new();
        let jobs = Coordinator::new(provider.clone(), Limits::default());
        let service = KnowledgeService::new(s.store.clone(), jobs.clone(), provider.clone())
            .with_matching(matching, verifier.clone());
        let input = fixture::command(
            ResearchInput {
                appearance_id: a.clone(),
                refresh: false,
                clarification: None,
            },
            Some(revision),
        );
        service.start_research(input.clone()).await.unwrap();
        provider.release.add_permits(1);
        fixture::until(async || {
            s.store.operation(&input.operation_id).await.unwrap().state == OperationState::Completed
        })
        .await;
        fixture::until(async || verifier.calls.load(Ordering::SeqCst) > 0).await;
        assert_eq!(
            jobs.snapshot().active,
            1,
            "verifier retains original admission"
        );
        assert_eq!(
            service.start_research(input.clone()).await.unwrap().state,
            OperationState::Completed
        );
        assert_eq!(
            verifier.calls.load(Ordering::SeqCst),
            1,
            "replay must not reverify"
        );
        let current_revision = s.store.exploration(&e).await.unwrap().exploration.revision;
        let refresh = fixture::command(
            ResearchInput {
                appearance_id: a.clone(),
                refresh: true,
                clarification: None,
            },
            Some(current_revision),
        );
        assert_eq!(
            service.start_research(refresh).await.unwrap_err().code(),
            "RESEARCH_LIMIT"
        );
        let before_left = s.store.exploration(&e).await.unwrap();
        let before_right = s.store.exploration(&other).await.unwrap();
        verifier.release.add_permits(1);
        service.drain().await.unwrap();
        assert_eq!(
            serde_json::to_value(s.store.exploration(&e).await.unwrap()).unwrap(),
            serde_json::to_value(before_left).unwrap()
        );
        assert_eq!(
            serde_json::to_value(s.store.exploration(&other).await.unwrap()).unwrap(),
            serde_json::to_value(before_right).unwrap()
        );
        let edges = s.store.connections(&left, None, 100).await.unwrap().items;
        let equivalent = edges
            .iter()
            .filter(|c| {
                c.kind == ConnectionKind::EquivalentQuestion
                    && c.status == ConnectionStatus::Accepted
                    && (c.from_topic_id == right || c.to_topic_id == right)
            })
            .count();
        println!(
            "matching fixture {}: accepted_equivalence={equivalent}, provider_calls={}",
            case["name"],
            verifier.calls.load(Ordering::SeqCst)
        );
        if case["equivalent"] == true {
            assert_eq!(equivalent, 1, "{}", case["name"]);
        } else {
            accepted_equivalence_edges_for_hard_negatives += equivalent;
        }
        assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
        worker.shutdown().await.unwrap();
        s.store.close().await.unwrap();
    }
    assert_eq!(accepted_equivalence_edges_for_hard_negatives, 0);
}
#[test]
fn verifier_rejects_record_updates_and_invented_candidate_ids() {
    assert!(
        ConnectionVerdict::from_json(
            serde_json::json!({"decisions":[],"recordUpdates":[{"id":"topic:x"}]})
        )
        .is_err()
    );
    let verdict=ConnectionVerdict::from_json(serde_json::json!({"decisions":[{"candidateId":"invented","verdict":"equivalent","reason":"Wrong identifier","paragraphRefs":[]}]})).unwrap();
    assert!(verdict.validate(&[]).is_err());
}

#[tokio::test]
async fn passage_overlap_records_both_versions_and_new_versions_get_new_candidates() {
    let s = fixture::Setup::new().await;
    let (e, a, left) = s.create("How does NAND flash store data?").await;
    let (_, b, right) = s.create("How do SSDs store data?").await;
    let worker = s.ready().await;
    let provider = provider_fixture::Provider::new();
    let service = KnowledgeService::new(
        s.store.clone(),
        Coordinator::new(provider.clone(), Limits::default()),
        provider.clone(),
    );
    let mut versions = vec![];
    for appearance in [&a, &b] {
        let input = fixture::command(
            ResearchInput {
                appearance_id: appearance.clone(),
                refresh: false,
                clarification: None,
            },
            Some(0),
        );
        service.start_research(input.clone()).await.unwrap();
        provider.release.add_permits(1);
        fixture::until(async || {
            s.store.operation(&input.operation_id).await.unwrap().state == OperationState::Completed
        })
        .await;
        let saved = s.store.operation(&input.operation_id).await.unwrap();
        let Some(OperationResult::Research {
            answer_version_id: Some(version),
            ..
        }) = saved.result
        else {
            unreachable!()
        };
        versions.push(version);
    }
    let matching = MatchingService::new(s.store.clone(), s.embedder.clone());
    matching
        .discover(&MatchSource::answer(right.clone(), versions[1].clone(), 1))
        .await
        .unwrap();
    let source = MatchSource::answer(left.clone(), versions[0].clone(), 1);
    matching.discover(&source).await.unwrap();
    let edges = s.store.connections(&left, None, 100).await.unwrap().items;
    let overlap = edges
        .into_iter()
        .find(|e| {
            e.kind == ConnectionKind::OverlappingAnswer
                && (e.from_topic_id == right || e.to_topic_id == right)
        })
        .unwrap();
    assert!(
        overlap
            .passages
            .iter()
            .any(|p| p.answer_version_id == versions[0])
    );
    assert!(
        overlap
            .passages
            .iter()
            .any(|p| p.answer_version_id == versions[1])
    );
    assert!(overlap.passages.iter().all(|p| !p.paragraph_id.is_empty()));
    let old = overlap.id.clone();
    s.store
        .dismiss_connection(&fixture::command(
            DismissConnection {
                connection_id: old.clone(),
            },
            None,
        ))
        .await
        .unwrap();
    matching.discover(&source).await.unwrap();
    assert!(
        s.store
            .connections(&left, None, 100)
            .await
            .unwrap()
            .items
            .iter()
            .all(|c| c.id != old)
    );
    let refresh = fixture::command(
        ResearchInput {
            appearance_id: a,
            refresh: true,
            clarification: None,
        },
        Some(1),
    );
    service.start_research(refresh.clone()).await.unwrap();
    provider.release.add_permits(1);
    fixture::until(async || {
        s.store
            .operation(&refresh.operation_id)
            .await
            .unwrap()
            .state
            == OperationState::Completed
    })
    .await;
    let saved = s.store.operation(&refresh.operation_id).await.unwrap();
    let Some(OperationResult::Research {
        answer_version_id: Some(version),
        ..
    }) = saved.result
    else {
        unreachable!()
    };
    matching
        .discover(&MatchSource::answer(left.clone(), version.clone(), 1))
        .await
        .unwrap();
    let edges = s.store.connections(&left, None, 100).await.unwrap().items;
    assert!(
        edges
            .iter()
            .any(|c| c.kind == ConnectionKind::OverlappingAnswer
                && c.id != old
                && c.passages.iter().any(|p| p.answer_version_id == version))
    );
    assert!(
        s.store
            .exploration(&e)
            .await
            .unwrap()
            .answers
            .iter()
            .any(|v| v.id == version)
    );
    service.drain().await.unwrap();
    worker.shutdown().await.unwrap();
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn verifier_citing_absent_paragraphs_fails_closed() {
    let s = fixture::Setup::new().await;
    let (_, _, a) = s.create("Flash memory").await;
    let (_, _, b) = s.create("NAND memory").await;
    let candidate = s.candidate(&a, &b).await;
    let value = serde_json::json!({"decisions":[{"candidateId":candidate.id,"verdict":"related","reason":"Invented citation","paragraphRefs":[{"answerVersionId":"missing-version","paragraphId":"missing-paragraph","sectionIndex":null}]}]});
    let verdict = ConnectionVerdict::from_json(value).unwrap();
    assert!(verdict.validate(&[candidate]).is_err());
    s.store.close().await.unwrap();
}

#[tokio::test]
async fn a_new_answer_can_help_an_existing_unopened_question() {
    let s = fixture::Setup::new().await;
    let (_, a, left) = s.create("Explain flash memory controllers").await;
    let (_, _, right) = s.create("Why does flash retain data without power?").await;
    let worker = s.ready().await;
    let provider = provider_fixture::Provider::new();
    let service = KnowledgeService::new(
        s.store.clone(),
        Coordinator::new(provider.clone(), Limits::default()),
        provider.clone(),
    );
    let input = fixture::command(
        ResearchInput {
            appearance_id: a,
            refresh: false,
            clarification: None,
        },
        Some(0),
    );
    service.start_research(input.clone()).await.unwrap();
    provider.release.add_permits(1);
    fixture::until(async || {
        s.store.operation(&input.operation_id).await.unwrap().state == OperationState::Completed
    })
    .await;
    let Some(OperationResult::Research {
        answer_version_id: Some(version),
        ..
    }) = s.store.operation(&input.operation_id).await.unwrap().result
    else {
        unreachable!()
    };
    let matching = MatchingService::new(s.store.clone(), s.embedder.clone());
    matching
        .discover(&MatchSource::answer(left.clone(), version.clone(), 1))
        .await
        .unwrap();
    let edges = s.store.connections(&right, None, 100).await.unwrap().items;
    assert!(edges.iter().any(
        |c| c.passages.iter().any(|p| p.answer_version_id == version)
            && (c.from_topic_id == left || c.to_topic_id == left)
    ));
    service.drain().await.unwrap();
    worker.shutdown().await.unwrap();
    s.store.close().await.unwrap();
}

#[tokio::test]
async fn verifier_timeout_keeps_saved_answer_and_replay_never_reverifies() {
    let s = fixture::Setup::new().await;
    let (e, a, _) = s.create("How does NAND flash store data?").await;
    s.create("How is data stored in NAND flash?").await;
    let worker = s.ready().await;
    let provider = provider_fixture::Provider::new();
    let verifier = Arc::new(Verifier {
        calls: AtomicUsize::new(0),
        release: Semaphore::new(0),
        verdict: VerdictKind::Equivalent,
    });
    let limits = Limits {
        research: std::time::Duration::from_secs(3),
        ..Limits::default()
    };
    let jobs = Coordinator::new(provider.clone(), limits);
    let service = KnowledgeService::new(s.store.clone(), jobs.clone(), provider.clone())
        .with_matching(
            Arc::new(MatchingService::new(s.store.clone(), s.embedder.clone())),
            verifier.clone(),
        );
    let input = fixture::command(
        ResearchInput {
            appearance_id: a,
            refresh: false,
            clarification: None,
        },
        Some(0),
    );
    service.start_research(input.clone()).await.unwrap();
    provider.release.add_permits(1);
    fixture::until(async || verifier.calls.load(Ordering::SeqCst) == 1).await;
    assert_eq!(
        s.store.operation(&input.operation_id).await.unwrap().state,
        OperationState::Completed
    );
    let saved = serde_json::to_value(s.store.exploration(&e).await.unwrap()).unwrap();
    fixture::until(async || jobs.snapshot().active == 0).await;
    service.drain().await.unwrap();
    let replay = KnowledgeService::new(
        s.store.clone(),
        Coordinator::new(provider.clone(), Limits::default()),
        provider,
    )
    .with_matching(
        Arc::new(MatchingService::new(s.store.clone(), s.embedder.clone())),
        verifier.clone(),
    );
    assert_eq!(
        replay.start_research(input).await.unwrap().state,
        OperationState::Completed
    );
    replay.drain().await.unwrap();
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        serde_json::to_value(s.store.exploration(&e).await.unwrap()).unwrap(),
        saved
    );
    worker.shutdown().await.unwrap();
    s.store.close().await.unwrap();
}

struct InvalidVerifier(bool);
impl ConnectionVerifier for InvalidVerifier {
    fn verify<'a>(
        &'a self,
        _: &'a [MatchCandidate],
        _: &'a JobContext,
    ) -> ProviderFuture<'a, ConnectionVerdict> {
        Box::pin(async move {
            if self.0 {
                Err(branch_backend::error::AppError::new(
                    branch_backend::error::ErrorCode::UpstreamFailure,
                ))
            } else {
                Ok(ConnectionVerdict {
                    decisions: vec![ConnectionDecision {
                        candidate_id: ConnectionId::new("invented").unwrap(),
                        verdict: VerdictKind::Equivalent,
                        reason: "Unsupported identity".into(),
                        paragraph_refs: vec![],
                    }],
                })
            }
        })
    }
}
#[tokio::test]
async fn invalid_or_failed_verifier_keeps_candidates_and_saved_research() {
    for failure in [false, true] {
        let s = fixture::Setup::new().await;
        let (e, a, topic) = s.create("How does NAND flash store data?").await;
        s.create("How is data stored in NAND flash?").await;
        let worker = s.ready().await;
        let provider = provider_fixture::Provider::new();
        let service = KnowledgeService::new(
            s.store.clone(),
            Coordinator::new(provider.clone(), Limits::default()),
            provider.clone(),
        )
        .with_matching(
            Arc::new(MatchingService::new(s.store.clone(), s.embedder.clone())),
            Arc::new(InvalidVerifier(failure)),
        );
        let input = fixture::command(
            ResearchInput {
                appearance_id: a,
                refresh: false,
                clarification: None,
            },
            Some(0),
        );
        service.start_research(input.clone()).await.unwrap();
        provider.release.add_permits(1);
        service.drain().await.unwrap();
        assert_eq!(
            s.store.operation(&input.operation_id).await.unwrap().state,
            OperationState::Completed
        );
        assert_eq!(s.store.exploration(&e).await.unwrap().answers.len(), 1);
        let edges = s.store.connections(&topic, None, 100).await.unwrap().items;
        assert!(!edges.is_empty());
        assert!(
            edges
                .iter()
                .filter(|e| e.kind != ConnectionKind::BranchesTo)
                .all(|e| e.status == ConnectionStatus::Candidate)
        );
        worker.shutdown().await.unwrap();
        s.store.close().await.unwrap();
    }
}
