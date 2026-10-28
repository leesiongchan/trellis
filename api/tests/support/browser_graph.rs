//! Fixed public-data graph fixture, linked only into the ignored browser test executable.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "matching.rs"]
mod matching_fixture;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use branch_backend::{
    access::{authorize_cookie, cookie_value, now_ms},
    app::AppState,
    contracts::library::*,
    embeddings::EmbeddingWorker,
    jobs::Coordinator,
    knowledge::{KnowledgeService, matching::*, verification::*},
    providers::{HttpProviders, ProviderFuture},
    research::context::JobContext,
};
use serde_json::{Value, json};
use std::sync::Arc;
#[derive(Clone)]
pub struct GraphState {
    pub app: Arc<AppState>,
    pub provider: Arc<HttpProviders>,
}
struct Verifier;
impl ConnectionVerifier for Verifier {
    fn model_id(&self) -> &str {
        "public-browser-fixture"
    }
    fn verify<'a>(
        &'a self,
        candidates: &'a [MatchCandidate],
        _: &'a JobContext,
    ) -> ProviderFuture<'a, ConnectionVerdict> {
        Box::pin(async move {
            Ok(ConnectionVerdict { decisions: candidates.iter().map(|c| ConnectionDecision { candidate_id: c.id.clone(), verdict: match c.channel() { MatchChannel::QuestionQuestion => VerdictKind::Equivalent, MatchChannel::PassagePassage => VerdictKind::Related, _ => VerdictKind::Uncertain }, reason: "Saved public fixture evidence connects these topics. <script data-connection-injected>inert</script>".into(), paragraph_refs: if c.channel() == MatchChannel::PassagePassage { [&c.from,&c.to].into_iter().filter_map(|e| e.passage.as_ref()).map(|p| ParagraphReference { answer_version_id:p.location.answer_version_id.clone(), paragraph_id:p.location.paragraph_id.clone(), section_index:p.location.section_index }).collect() } else {vec![]} }).collect() })
        })
    }
}
async fn finish(app: &AppState, id: &OperationId) -> OperationView {
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            let op = app.store.operation(id).await.unwrap();
            if op.state != OperationState::Pending {
                assert_eq!(op.state, OperationState::Completed);
                return op;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

async fn create(
    state: &GraphState,
    service: &KnowledgeService,
    epoch: u64,
    question: &str,
) -> (ExplorationId, AppearanceId, TopicId) {
    let saved = state
        .app
        .store
        .create_exploration(&Command {
            operation_id: OperationId::generate(),
            library_epoch: epoch,
            expected_revision: None,
            payload: CreateExploration {
                question: question.into(),
                locale: "en".into(),
            },
        })
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
    let command = Command {
        operation_id: OperationId::generate(),
        library_epoch: epoch,
        expected_revision: Some(0),
        payload: ResearchInput {
            appearance_id: appearance_id.clone(),
            refresh: false,
            clarification: None,
        },
    };
    service.start_research(command.clone()).await.unwrap();
    finish(&state.app, &command.operation_id).await;
    let snapshot = state.app.store.exploration(&exploration_id).await.unwrap();
    let topic = snapshot
        .appearances
        .iter()
        .find(|a| a.id == appearance_id)
        .unwrap()
        .topic_id
        .clone();
    (exploration_id, appearance_id, topic)
}
pub async fn seed(
    State(state): State<GraphState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let expected = state.app.config.origin.origin().ascii_serialization();
    if !authorize_cookie(
        cookie_value(&headers),
        state.app.config.access_key.expose(),
        now_ms().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if headers.get("origin").and_then(|v| v.to_str().ok()) != Some(expected.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }
    let epoch = state.app.store.library_info().await.unwrap().library_epoch;
    let jobs = Coordinator::new(state.provider.clone(), state.app.config.limits.clone());
    let base = KnowledgeService::new(
        state.app.store.clone(),
        jobs.clone(),
        state.provider.clone(),
    );
    let source = create(&state, &base, epoch, "How does flash retain data?").await;
    let target = create(&state, &base, epoch, "How can flash retain data?").await;
    let dummy = matching_fixture::Setup::new().await;
    dummy.store.close().await.unwrap();
    let fixture = matching_fixture::Setup {
        root: dummy.root,
        store: state.app.store.clone(),
        embedder: dummy.embedder,
    };
    let worker = EmbeddingWorker::start(
        state.app.store.clone(),
        fixture.embedder.clone(),
        tokio_util::sync::CancellationToken::new(),
    );
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        while state.app.store.library_info().await.unwrap().discovery != DiscoveryStatus::Ready {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let matcher = Arc::new(MatchingService::new(
        state.app.store.clone(),
        fixture.embedder.clone(),
    ));
    let verified = KnowledgeService::new(
        state.app.store.clone(),
        jobs.clone(),
        state.provider.clone(),
    )
    .with_matching(matcher, Arc::new(Verifier));
    let revision = state
        .app
        .store
        .exploration(&source.0)
        .await
        .unwrap()
        .exploration
        .revision;
    let refresh = Command {
        operation_id: OperationId::generate(),
        library_epoch: epoch,
        expected_revision: Some(revision),
        payload: ResearchInput {
            appearance_id: source.1.clone(),
            refresh: true,
            clarification: None,
        },
    };
    verified.start_research(refresh.clone()).await.unwrap();
    finish(&state.app, &refresh.operation_id).await;
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        while jobs.snapshot().active > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    for q in [
        "Flash storage lifespan",
        "Flash controller behavior",
        "Flash memory cells",
        "Flash storage capacity",
        "Flash memory technology",
        "Flash storage architecture",
    ] {
        create(&state, &base, epoch, q).await;
    }
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            let links = state
                .app
                .store
                .connections(&source.2, None, 100)
                .await
                .unwrap()
                .items;
            if links.len() > 5
                && links
                    .iter()
                    .any(|c| c.kind == ConnectionKind::OverlappingAnswer)
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    worker.shutdown().await.unwrap();
    verified.drain().await.unwrap();
    base.drain().await.unwrap();
    let snapshot = state.app.store.exploration(&source.0).await.unwrap();
    let pin = snapshot
        .appearances
        .iter()
        .find(|a| a.id == source.1)
        .unwrap()
        .answer_version_id
        .clone();
    Ok(Json(
        json!({"sourceExplorationId":source.0,"sourceAppearanceId":source.1,"sourceTopicId":source.2,"sourceAnswerVersionId":pin,"targetExplorationId":target.0,"targetTopicId":target.2}),
    ))
}
