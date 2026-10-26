#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/saved_provider.rs"]
mod fixture;
use branch_backend::{
    config::Limits,
    contracts::library::*,
    error::ErrorCode,
    jobs::Coordinator,
    knowledge::KnowledgeService,
    storage::{KnowledgeStore, StorageConfig},
};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
struct Setup {
    _root: tempfile::TempDir,
    store: Arc<KnowledgeStore>,
    provider: Arc<fixture::Provider>,
    service: Arc<KnowledgeService>,
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
        let provider = fixture::Provider::new();
        let jobs = Coordinator::new(provider.clone(), Limits::default());
        let service = Arc::new(KnowledgeService::new(store.clone(), jobs, provider.clone()));
        Self {
            _root: root,
            store,
            provider,
            service,
        }
    }
    async fn create(&self, q: &str) -> (ExplorationId, AppearanceId) {
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
        (exploration_id, appearance_id)
    }
    async fn finish(&self, id: &OperationId) -> OperationView {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let value = self.store.operation(id).await.unwrap();
                if value.state != OperationState::Pending {
                    return value;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap()
    }
    async fn shutdown(&self) {
        self.service.drain().await.unwrap();
        self.store.close().await.unwrap();
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
fn research(id: &AppearanceId, revision: u64, refresh: bool) -> Command<ResearchInput> {
    cmd(
        ResearchInput {
            appearance_id: id.clone(),
            refresh,
            clarification: None,
        },
        Some(revision),
    )
}
async fn pin(s: &Setup, e: &ExplorationId, a: &AppearanceId) -> Option<AnswerVersionId> {
    s.store
        .exploration(e)
        .await
        .unwrap()
        .appearances
        .into_iter()
        .find(|v| v.id == *a)
        .unwrap()
        .answer_version_id
}
#[tokio::test]
async fn lost_response_replays_without_research() {
    let s = Setup::new().await;
    let (e, a) = s.create("Memory").await;
    let input = research(&a, 0, false);
    let admitted = s.service.start_research(input.clone()).await.unwrap();
    assert_eq!(admitted.state, OperationState::Pending);
    s.provider.release.add_permits(1);
    let saved = s.finish(&input.operation_id).await;
    assert_eq!(saved.state, OperationState::Completed);
    let pinned = pin(&s, &e, &a).await.unwrap();
    let replay = s.service.start_research(input).await.unwrap();
    assert_eq!(replay.id, saved.id);
    assert_eq!(pin(&s, &e, &a).await, Some(pinned));
    let search_calls = s.provider.searches.load(Ordering::SeqCst);
    let generation_calls = s.provider.generations.load(Ordering::SeqCst);
    assert_eq!(search_calls, 1);
    assert_eq!(generation_calls, 1);
    let snapshot = s.store.exploration(&e).await.unwrap();
    assert_eq!(snapshot.answers.len(), 1);
    assert!(snapshot.appearances.len() > 1);
    assert_eq!(snapshot.exploration.revision, 1);
    s.shutdown().await;
}
#[tokio::test]
async fn refresh_keeps_other_pins() {
    let s = Setup::new().await;
    let (e, a) = s.create("Memory").await;
    let first = research(&a, 0, false);
    s.service.start_research(first.clone()).await.unwrap();
    s.provider.release.add_permits(1);
    s.finish(&first.operation_id).await;
    let (other, oa) = s.create("Memory").await;
    let other_appearance_pin_before_refresh = pin(&s, &other, &oa).await;
    let refresh = research(&a, 1, true);
    s.service.start_research(refresh.clone()).await.unwrap();
    s.provider.release.add_permits(1);
    s.finish(&refresh.operation_id).await;
    let other_appearance_pin_after_refresh = pin(&s, &other, &oa).await;
    assert_eq!(
        other_appearance_pin_after_refresh,
        other_appearance_pin_before_refresh
    );
    let refreshed = s.store.exploration(&e).await.unwrap();
    let suggestion = refreshed
        .appearances
        .iter()
        .find(|v| v.parent_id.as_ref() == Some(&a))
        .unwrap();
    assert_eq!(
        suggestion.reason_answer_version_id,
        other_appearance_pin_before_refresh
    );
    assert_eq!(
        suggestion.reason_sources[0].url,
        "https://example.org/memory"
    );

    assert_ne!(pin(&s, &e, &a).await, other_appearance_pin_before_refresh);
    let snapshot = s.store.exploration(&e).await.unwrap();
    let topic = &snapshot
        .appearances
        .iter()
        .find(|v| v.id == a)
        .unwrap()
        .topic_id;
    assert_eq!(s.store.topic(topic, None).await.unwrap().versions.len(), 2);
    s.shutdown().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_refreshes_share_one_paid_attempt() {
    let s = Setup::new().await;
    let (e, a) = s.create("Memory").await;
    let mut handles = vec![];
    for _ in 0..12 {
        let service = s.service.clone();
        let command = research(&a, 0, true);
        handles.push(tokio::spawn(async move {
            let id = command.operation_id.clone();
            service.start_research(command).await.map(|_| id)
        }));
    }
    let mut ids = vec![];
    for h in handles {
        ids.push(h.await.unwrap().unwrap());
    }
    s.provider.release.add_permits(1);
    for id in ids {
        assert_eq!(s.finish(&id).await.state, OperationState::Completed);
    }
    assert_eq!(s.provider.searches.load(Ordering::SeqCst), 1);
    assert_eq!(s.provider.generations.load(Ordering::SeqCst), 1);
    assert!(pin(&s, &e, &a).await.is_some());
    s.shutdown().await;
}
#[tokio::test]
async fn failures_and_clarification_preserve_the_previous_answer() {
    let s = Setup::new().await;
    let (e, a) = s.create("Memory").await;
    let first = research(&a, 0, false);
    s.service.start_research(first.clone()).await.unwrap();
    s.provider.release.add_permits(1);
    s.finish(&first.operation_id).await;
    let saved = pin(&s, &e, &a).await;
    for mode in [1, 2, 3] {
        s.provider.mode.store(mode, Ordering::SeqCst);
        let revision = s.store.exploration(&e).await.unwrap().exploration.revision;
        let input = research(&a, revision, true);
        s.service.start_research(input.clone()).await.unwrap();
        s.provider.release.add_permits(1);
        let result = s.finish(&input.operation_id).await;
        assert_ne!(result.state, OperationState::Pending);
        assert_eq!(pin(&s, &e, &a).await, saved);
        let calls = s.provider.searches.load(Ordering::SeqCst);
        s.service.start_research(input).await.unwrap();
        assert_eq!(s.provider.searches.load(Ordering::SeqCst), calls);
    }
    s.shutdown().await;
}
#[tokio::test]
async fn completion_after_delete_or_purge_cannot_resurrect_content() {
    for purge in [false, true] {
        let s = Setup::new().await;
        let (e, a) = s.create("Memory").await;
        let input = research(&a, 0, false);
        s.service.start_research(input.clone()).await.unwrap();
        if purge {
            s.store
                .purge_library(&cmd(
                    PurgeLibrary {
                        confirmation: PURGE_CONFIRMATION.into(),
                    },
                    None,
                ))
                .await
                .unwrap();
        } else {
            s.store
                .delete_exploration(&cmd(
                    DeleteExploration {
                        exploration_id: e.clone(),
                    },
                    Some(0),
                ))
                .await
                .unwrap();
        }
        s.provider.release.add_permits(1);
        s.service.drain().await.unwrap();
        assert_eq!(
            s.store.exploration(&e).await.unwrap_err().kind,
            ErrorCode::NotFound
        );
        assert!(
            s.store
                .explorations(None, 100)
                .await
                .unwrap()
                .items
                .is_empty()
        );
        s.store.close().await.unwrap();
    }
}
#[tokio::test]
async fn chosen_versions_must_belong_to_the_appearance_topic() {
    let s = Setup::new().await;
    let (e, a) = s.create("Memory").await;
    let input = research(&a, 0, false);
    s.service.start_research(input.clone()).await.unwrap();
    s.provider.release.add_permits(1);
    s.finish(&input.operation_id).await;
    let version = pin(&s, &e, &a).await.unwrap();
    let (other, oa) = s.create("Fruit").await;
    assert_eq!(
        s.store
            .choose_answer(&cmd(
                ChooseAnswer {
                    appearance_id: oa,
                    answer_version_id: version.clone()
                },
                Some(0)
            ))
            .await
            .unwrap_err()
            .kind,
        ErrorCode::NotFound
    );
    assert_eq!(
        s.store
            .exploration(&other)
            .await
            .unwrap()
            .exploration
            .revision,
        0
    );
    s.store
        .choose_answer(&cmd(
            ChooseAnswer {
                appearance_id: a,
                answer_version_id: version,
            },
            Some(1),
        ))
        .await
        .unwrap();
    s.shutdown().await;
}

#[tokio::test]
async fn clarification_uses_stored_context_and_rebinds_only_selected_appearance() {
    let s = Setup::new().await;
    let mut children = vec![];
    for _ in 0..2 {
        let (e, a) = s.create("Memory").await;
        let saved = s
            .store
            .add_branch(&cmd(
                AddBranch {
                    parent_appearance_id: a,
                    question: "Flash".into(),
                },
                Some(0),
            ))
            .await
            .unwrap();
        let Some(OperationResult::Exploration { appearance_id, .. }) = saved.result else {
            unreachable!()
        };
        children.push((e, appearance_id));
    }
    let before = s
        .store
        .exploration(&children[0].0)
        .await
        .unwrap()
        .appearances
        .into_iter()
        .find(|a| a.id == children[0].1)
        .unwrap()
        .topic_id;
    let mut input = research(&children[0].1, 1, false);
    input.payload.clarification = Some("In a smartphone".into());
    s.service.start_research(input.clone()).await.unwrap();
    s.provider.release.add_permits(1);
    assert_eq!(
        s.finish(&input.operation_id).await.state,
        OperationState::Completed
    );
    let selected = s
        .store
        .exploration(&children[0].0)
        .await
        .unwrap()
        .appearances
        .into_iter()
        .find(|a| a.id == children[0].1)
        .unwrap();
    assert_ne!(selected.topic_id, before);
    assert!(selected.answer_version_id.is_some());
    let other = s
        .store
        .exploration(&children[1].0)
        .await
        .unwrap()
        .appearances
        .into_iter()
        .find(|a| a.id == children[1].1)
        .unwrap();
    assert_eq!(other.topic_id, before);
    assert!(other.answer_version_id.is_none());
    {
        let requests = s.provider.requests.lock().unwrap();
        assert_eq!(
            requests[0].clarification.as_deref(),
            Some("In a smartphone")
        );
        assert_eq!(requests[0].ancestors[0].title, "Memory");
    }
    s.shutdown().await;
}
#[tokio::test]
async fn storage_failure_keeps_old_answer_and_never_retries_provider_work() {
    let s = Setup::new().await;
    let (e, a) = s.create("Memory").await;
    let first = research(&a, 0, false);
    s.service.start_research(first.clone()).await.unwrap();
    s.provider.release.add_permits(1);
    s.finish(&first.operation_id).await;
    let saved = pin(&s, &e, &a).await;
    let refresh = research(&a, 1, true);
    s.service.start_research(refresh).await.unwrap();
    s.store.close().await.unwrap();
    s.provider.release.add_permits(1);
    assert_eq!(
        s.service.drain().await.unwrap_err().kind,
        ErrorCode::StorageUnavailable
    );
    let root = s._root.path();
    let reopened = KnowledgeStore::open(&StorageConfig {
        data_dir: root.join("data"),
        backup_dir: root.join("backups"),
        model_dir: root.join("models"),
    })
    .await
    .unwrap();
    reopened.install_schema().await.unwrap();
    let appearance = reopened
        .exploration(&e)
        .await
        .unwrap()
        .appearances
        .into_iter()
        .find(|v| v.id == a)
        .unwrap();
    assert_eq!(appearance.answer_version_id, saved);
    assert_eq!(s.provider.searches.load(Ordering::SeqCst), 2);
    assert_eq!(s.provider.generations.load(Ordering::SeqCst), 2);
    reopened.close().await.unwrap();
}
#[tokio::test]
async fn saved_research_survives_an_initiating_task_disconnect() {
    let s = Setup::new().await;
    let (e, a) = s.create("Memory").await;
    let input = research(&a, 0, false);
    let id = input.operation_id.clone();
    let service = s.service.clone();
    let request = tokio::spawn(async move { service.start_research(input).await });
    tokio::time::timeout(Duration::from_secs(3), async {
        while s.provider.searches.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    request.abort();
    s.provider.release.add_permits(1);
    assert_eq!(s.finish(&id).await.state, OperationState::Completed);
    assert!(pin(&s, &e, &a).await.is_some());
    s.shutdown().await;
}

#[tokio::test]
async fn explicit_answer_choice_fences_an_already_running_refresh() {
    let s = Setup::new().await;
    let (e, a) = s.create("Memory").await;
    let first = research(&a, 0, false);
    s.service.start_research(first.clone()).await.unwrap();
    s.provider.release.add_permits(1);
    s.finish(&first.operation_id).await;
    let original = pin(&s, &e, &a).await.unwrap();
    let refresh = research(&a, 1, true);
    s.service.start_research(refresh.clone()).await.unwrap();
    s.store
        .choose_answer(&cmd(
            ChooseAnswer {
                appearance_id: a.clone(),
                answer_version_id: original.clone(),
            },
            Some(1),
        ))
        .await
        .unwrap();
    s.provider.release.add_permits(1);
    assert_eq!(
        s.finish(&refresh.operation_id).await.state,
        OperationState::Completed
    );
    assert_eq!(pin(&s, &e, &a).await, Some(original));
    assert_eq!(
        s.store.exploration(&e).await.unwrap().exploration.revision,
        2
    );
    s.shutdown().await;
}
#[tokio::test]
async fn exact_answer_reuse_needs_no_provider_and_populates_suggestions() {
    let s = Setup::new().await;
    let (_, a) = s.create("Memory").await;
    let first = research(&a, 0, false);
    s.service.start_research(first.clone()).await.unwrap();
    s.provider.release.add_permits(1);
    s.finish(&first.operation_id).await;
    let (other, oa) = s.create("Memory").await;
    let input = research(&oa, 0, false);
    assert_eq!(
        s.service.start_research(input).await.unwrap().state,
        OperationState::Completed
    );
    let snapshot = s.store.exploration(&other).await.unwrap();
    assert!(snapshot.appearances.len() > 1);
    assert_eq!(snapshot.answers.len(), 1);
    assert_eq!(snapshot.exploration.revision, 1);
    assert_eq!(s.provider.searches.load(Ordering::SeqCst), 1);
    s.shutdown().await;
}

#[tokio::test]
async fn opening_a_cached_answer_during_refresh_keeps_its_existing_pin() {
    let s = Setup::new().await;
    let (original_exploration, a) = s.create("Memory").await;
    let first = research(&a, 0, false);
    s.service.start_research(first.clone()).await.unwrap();
    s.provider.release.add_permits(1);
    s.finish(&first.operation_id).await;
    let old_original = pin(&s, &original_exploration, &a).await;
    let refresh = research(&a, 1, true);
    s.service.start_research(refresh.clone()).await.unwrap();
    assert_eq!(
        s.service
            .start_research(research(&a, 1, false))
            .await
            .unwrap()
            .state,
        OperationState::Completed
    );
    let (other, oa) = s.create("Memory").await;
    let original = pin(&s, &other, &oa).await;
    let opened = s
        .service
        .start_research(research(&oa, 0, false))
        .await
        .unwrap();
    assert_eq!(opened.state, OperationState::Completed);
    assert_eq!(pin(&s, &other, &oa).await, original);
    s.provider.release.add_permits(1);
    s.finish(&refresh.operation_id).await;
    assert_ne!(pin(&s, &original_exploration, &a).await, old_original);
    assert_eq!(pin(&s, &other, &oa).await, original);
    let revision = s
        .store
        .exploration(&other)
        .await
        .unwrap()
        .exploration
        .revision;
    s.service
        .start_research(research(&oa, revision, false))
        .await
        .unwrap();
    assert_eq!(pin(&s, &other, &oa).await, original);
    assert_eq!(s.provider.searches.load(Ordering::SeqCst), 2);
    s.shutdown().await;
}
