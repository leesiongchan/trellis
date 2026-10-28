#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use std::collections::HashMap;

#[test]
fn packaged_storage_configuration_does_not_require_the_source_checkout() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    assert!(
        super::config::validate_repository_roots(
            &root.join("data"),
            &root.join("backups"),
            &root.join("absent-build-checkout")
        )
        .is_ok()
    );
}

#[tokio::test]
async fn embedded_nested_answer_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    let config = StorageConfig::from_values(
        &HashMap::from([(
            "APP_DATA_DIR".into(),
            directory.path().join("data").display().to_string(),
        )]),
        &directory.path().join("release/branch-backend"),
    )
    .unwrap();
    let store = KnowledgeStore::open(&config).await.unwrap();
    let before = {
        let guard = store.client.read().await;
        let db = guard.as_ref().unwrap();
        db.query("CREATE fixture:answer CONTENT {summary:[{text:'Flash retains charge.',evidence:[{sourceId:'s1',quote:'Flash retains charge.'}]}],sources:[{id:'s1',url:'https://example.org/flash',retrievedAt:'2026-10-27T00:00:00Z'}]};").await.unwrap().check().unwrap();
        db.query("SELECT * FROM ONLY fixture:answer;")
            .await
            .unwrap()
            .take::<surrealdb::types::Value>(0)
            .unwrap()
    };
    assert_eq!(
        before.clone().into_json_value()["summary"][0]["text"],
        "Flash retains charge."
    );
    store.close().await.unwrap();
    let store = KnowledgeStore::open(&config).await.unwrap();
    let after = {
        let guard = store.client.read().await;
        guard
            .as_ref()
            .unwrap()
            .query("SELECT * FROM ONLY fixture:answer;")
            .await
            .unwrap()
            .take::<surrealdb::types::Value>(0)
            .unwrap()
    };
    assert_eq!(after, before);
    store.close().await.unwrap();
}

#[test]
fn transaction_root_error_is_not_hidden_by_rollback_markers() {
    use surrealdb::types::{Error, QueryError};
    let errors = vec![
        (
            0,
            Error::query("not executed".into(), QueryError::NotExecuted),
        ),
        (
            1,
            Error::query("conflict".into(), QueryError::TransactionConflict),
        ),
    ];
    assert_eq!(
        super::root_query_error(errors).unwrap().query_details(),
        Some(&QueryError::TransactionConflict)
    );
}

#[tokio::test]
async fn branch_edges_retain_only_live_occurrence_support() {
    use crate::contracts::library::*;
    use serde_json::json;
    let root = tempfile::tempdir().unwrap();
    let config = StorageConfig {
        data_dir: root.path().join("data"),
        backup_dir: root.path().join("backups"),
        model_dir: root.path().join("models"),
        asset_dir: root.path().join("public"),
    };
    let store = KnowledgeStore::open(&config).await.unwrap();
    store.install_schema().await.unwrap();
    fn command<T>(payload: T, revision: Option<u64>) -> Command<T> {
        Command {
            operation_id: OperationId::generate(),
            library_epoch: 1,
            expected_revision: revision,
            payload,
        }
    }
    let mut explorations = vec![];
    for _ in 0..2 {
        let saved = store
            .create_exploration(&command(
                CreateExploration {
                    question: "Memory".into(),
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
        store
            .add_branch(&command(
                AddBranch {
                    parent_appearance_id: appearance_id,
                    question: "Flash".into(),
                },
                Some(0),
            ))
            .await
            .unwrap();
        explorations.push(exploration_id);
    }
    async fn counts(store: &KnowledgeStore) -> serde_json::Value {
        store.query_value("RETURN {edges:array::len((SELECT VALUE key FROM connection LIMIT 10)),supports:array::len((SELECT VALUE key FROM edge_support LIMIT 10)),topics:array::len((SELECT VALUE key FROM topic LIMIT 10))};",json!({})).await.unwrap()
    }
    assert_eq!(
        counts(&store).await,
        json!({"edges":1,"supports":2,"topics":2})
    );
    store
        .delete_exploration(&command(
            DeleteExploration {
                exploration_id: explorations[0].clone(),
            },
            Some(1),
        ))
        .await
        .unwrap();
    assert_eq!(
        counts(&store).await,
        json!({"edges":1,"supports":1,"topics":2})
    );
    store
        .delete_exploration(&command(
            DeleteExploration {
                exploration_id: explorations[1].clone(),
            },
            Some(1),
        ))
        .await
        .unwrap();
    assert_eq!(
        counts(&store).await,
        json!({"edges":0,"supports":0,"topics":2})
    );
    let receipts = store
        .query_value("SELECT VALUE data FROM import_receipt LIMIT 10;", json!({}))
        .await
        .unwrap();
    assert_eq!(receipts.as_array().unwrap().len(), 2);
    assert!(!receipts.to_string().contains("Memory"));
    store.close().await.unwrap();
}

#[tokio::test]
async fn failed_mutation_rolls_back_content_and_operation_together() {
    use crate::contracts::library::*;
    use serde_json::json;
    let root = tempfile::tempdir().unwrap();
    let config = StorageConfig {
        data_dir: root.path().join("data"),
        backup_dir: root.path().join("backups"),
        model_dir: root.path().join("models"),
        asset_dir: root.path().join("public"),
    };
    let store = KnowledgeStore::open(&config).await.unwrap();
    store.install_schema().await.unwrap();
    let intent = super::operations::OperationIntent::new(
        OperationId::generate(),
        1,
        json!({}),
        "create-exploration",
    )
    .unwrap();
    let result=store.commit_mutation(&intent,"CREATE topic:orphan CONTENT {key:'orphan',data:{question:'Must roll back'}}; THROW 'revision_conflict';",json!({}),OperationResult::Deletion{exploration_id:ExplorationId::generate()}).await;
    assert_eq!(result.unwrap_err().kind, ErrorCode::RevisionConflict);
    assert!(
        store
            .query_value("SELECT * FROM topic LIMIT 10;", json!({}))
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store.operation(&intent.id).await.unwrap_err().kind,
        ErrorCode::NotFound
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn topic_versions_are_scoped_and_exact_reuse_keeps_the_pin() {
    use crate::contracts::library::*;
    use serde_json::json;
    let root = tempfile::tempdir().unwrap();
    let store = KnowledgeStore::open(&StorageConfig {
        data_dir: root.path().join("data"),
        backup_dir: root.path().join("backups"),
        model_dir: root.path().join("models"),
        asset_dir: root.path().join("public"),
    })
    .await
    .unwrap();
    store.install_schema().await.unwrap();
    fn command<T>(payload: T) -> Command<T> {
        Command {
            operation_id: OperationId::generate(),
            library_epoch: 1,
            expected_revision: None,
            payload,
        }
    }
    let mut topics = vec![];
    for question in ["Memory", "Computing"] {
        let saved = store
            .create_exploration(&command(CreateExploration {
                question: question.into(),
                locale: "en".into(),
            }))
            .await
            .unwrap();
        let Some(OperationResult::Exploration { exploration_id, .. }) = saved.result else {
            unreachable!()
        };
        topics.push(
            store
                .exploration(&exploration_id)
                .await
                .unwrap()
                .appearances[0]
                .topic_id
                .clone(),
        );
    }
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/results.json"
    ))
    .unwrap();
    let content =
        crate::contracts::ResearchResult::from_json(fixtures[0]["input"].clone()).unwrap();
    let crate::contracts::ResearchResult::Topic { content } = content else {
        unreachable!()
    };
    let version = AnswerVersionId::generate();
    let answer = AnswerVersionView {
        provenance: AnswerProvenance::Unavailable,
        origin: AnswerOrigin::Research,
        id: version.clone(),
        topic_id: topics[0].clone(),
        revision: 1,
        content,
        created_at: records::now().unwrap(),
    };
    store.query("CREATE ONLY type::record('answer_version',$key) CONTENT $record; UPDATE ONLY type::record('topic',$topic) SET data.currentAnswerVersionId=$key;",json!({"key":version,"topic":topics[0],"record":{"key":version,"topicKey":topics[0],"revision":1,"data":answer}})).await.unwrap();
    assert_eq!(
        store
            .topic(&topics[1], Some(&version))
            .await
            .unwrap_err()
            .kind,
        ErrorCode::NotFound
    );
    let saved = store
        .create_exploration(&command(CreateExploration {
            question: "Memory".into(),
            locale: "en".into(),
        }))
        .await
        .unwrap();
    let Some(OperationResult::Exploration { exploration_id, .. }) = saved.result else {
        unreachable!()
    };
    let snapshot = store.exploration(&exploration_id).await.unwrap();
    assert_eq!(
        snapshot.appearances[0].answer_version_id,
        Some(version.clone())
    );
    assert_eq!(snapshot.answers[0].id, version);
    assert_eq!(
        snapshot.answers[0].content.sources[0].url,
        "https://example.org/memory"
    );
    store.close().await.unwrap();
}

#[path = "../../tests/support/saved_provider.rs"]
mod saved_provider;
#[tokio::test]
async fn commit_precedes_completion() {
    use crate::{
        config::Limits, contracts::library::*, jobs::Coordinator, knowledge::KnowledgeService,
    };
    use serde_json::json;
    use std::sync::{Arc, atomic::Ordering};
    let root = tempfile::tempdir().unwrap();
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
    let created = store
        .create_exploration(&Command {
            operation_id: OperationId::generate(),
            library_epoch: 1,
            expected_revision: None,
            payload: CreateExploration {
                question: "Memory".into(),
                locale: "en".into(),
            },
        })
        .await
        .unwrap();
    let Some(OperationResult::Exploration {
        exploration_id,
        appearance_id,
        ..
    }) = created.result
    else {
        unreachable!()
    };
    let provider = saved_provider::Provider::new();
    let service = KnowledgeService::new(
        store.clone(),
        Coordinator::new(provider.clone(), Limits::default()),
        provider.clone(),
    );
    let command = Command {
        operation_id: OperationId::generate(),
        library_epoch: 1,
        expected_revision: Some(0),
        payload: ResearchInput {
            appearance_id: appearance_id.clone(),
            refresh: false,
            clarification: None,
        },
    };
    service.start_research(command.clone()).await.unwrap();
    let pause = store.admission.acquire_many(16).await.unwrap();
    provider.release.add_permits(1);
    tokio::time::timeout(Duration::from_secs(3), async {
        while provider.verifications.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    // Bypass the admission gate only in this private test to inspect the actual durable state.
    let value = {
        let guard = store.client.read().await;
        let mut response=guard.as_ref().unwrap().query("RETURN {operation:(SELECT VALUE data FROM ONLY type::record('operation',$key)),answers:array::len((SELECT VALUE key FROM answer_version LIMIT 10))};").bind(json!({"key":command.operation_id})).await.unwrap().check().unwrap();
        response
            .take::<surrealdb::types::Value>(0)
            .unwrap()
            .into_json_value()
    };
    assert_eq!(value["operation"]["state"], "pending");
    assert_eq!(value["answers"], 0);
    drop(pause);
    tokio::time::timeout(Duration::from_secs(3), async {
        while store.operation(&command.operation_id).await.unwrap().state == OperationState::Pending
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        store.operation(&command.operation_id).await.unwrap().state,
        OperationState::Completed
    );
    assert!(
        store
            .exploration(&exploration_id)
            .await
            .unwrap()
            .appearances
            .iter()
            .find(|a| a.id == appearance_id)
            .unwrap()
            .answer_version_id
            .is_some()
    );
    service.drain().await.unwrap();
    store.close().await.unwrap();
}

#[tokio::test]
async fn schema_probe_before_initial_install() {
    let root = tempfile::tempdir().unwrap();
    let store = KnowledgeStore::open(&StorageConfig {
        data_dir: root.path().join("data"),
        backup_dir: root.path().join("backups"),
        model_dir: root.path().join("models"),
        asset_dir: root.path().join("public"),
    })
    .await
    .unwrap();
    assert!(store.check_schema().await.is_ok());
    store.close().await.unwrap();
}

#[tokio::test]
async fn native_backup_counts_are_scalar() {
    let root = tempfile::tempdir().unwrap();
    let store = KnowledgeStore::open(&StorageConfig {
        data_dir: root.path().join("data"),
        backup_dir: root.path().join("backups"),
        model_dir: root.path().join("models"),
        asset_dir: root.path().join("public"),
    })
    .await
    .unwrap();
    store.install_schema().await.unwrap();
    let counts=store.query_value("RETURN {library:((SELECT count() AS total FROM library GROUP ALL)[0].total ?? 0),topic:((SELECT count() AS total FROM topic GROUP ALL)[0].total ?? 0)};",serde_json::json!({})).await.unwrap();
    assert_eq!(counts, serde_json::json!({"library":1,"topic":0}));
    store.close().await.unwrap();
}

#[tokio::test]
async fn maintenance_keeps_reads_available_and_defers_writes() {
    use crate::contracts::library::*;
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
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
    let lease = store.maintenance.write().await;
    tokio::time::timeout(std::time::Duration::from_millis(100), store.library_info())
        .await
        .unwrap()
        .unwrap();
    let db = store.clone();
    let task = tokio::spawn(async move {
        db.create_exploration(&Command {
            operation_id: OperationId::generate(),
            library_epoch: 1,
            expected_revision: None,
            payload: CreateExploration {
                question: "Pending write".into(),
                locale: "en".into(),
            },
        })
        .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(!task.is_finished());
    drop(lease);
    assert_eq!(
        task.await.unwrap().unwrap().state,
        OperationState::Completed
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn incompatible_versions_fail_before_schema_mutation_or_engine_reopen() {
    let root = tempfile::tempdir().unwrap();
    let config = StorageConfig {
        data_dir: root.path().join("data"),
        backup_dir: root.path().join("backups"),
        model_dir: root.path().join("models"),
        asset_dir: root.path().join("public"),
    };
    let store = KnowledgeStore::open(&config).await.unwrap();
    store.install_schema().await.unwrap();
    store
        .query(
            "UPDATE library:main SET schemaVersion=999;",
            serde_json::json!({}),
        )
        .await
        .unwrap();
    assert_eq!(
        store.install_schema().await.unwrap_err().code(),
        "STORAGE_VERSION_MISMATCH"
    );
    assert_eq!(
        store
            .query_value(
                "SELECT VALUE schemaVersion FROM ONLY library:main;",
                serde_json::json!({})
            )
            .await
            .unwrap(),
        serde_json::json!(999)
    );
    store.close().await.unwrap();
    std::fs::write(
        config.data_dir.join(".branch-storage-version.json"),
        b"{\"engineVersion\":\"999\",\"schemaVersion\":1}",
    )
    .unwrap();
    assert_eq!(
        KnowledgeStore::open(&config).await.err().unwrap().code(),
        "STORAGE_VERSION_MISMATCH"
    );
}

#[tokio::test]
async fn preupgrade_backup_records_the_old_release_and_restores_to_fresh_storage() {
    use crate::contracts::library::*;
    let root = tempfile::tempdir().unwrap();
    let config = StorageConfig {
        data_dir: root.path().join("data"),
        backup_dir: root.path().join("backups"),
        model_dir: root.path().join("models"),
        asset_dir: root.path().join("public"),
    };
    let store = KnowledgeStore::open(&config).await.unwrap();
    store.install_schema().await.unwrap();
    store
        .create_exploration(&Command {
            operation_id: OperationId::generate(),
            library_epoch: 1,
            expected_revision: None,
            payload: CreateExploration {
                question: "Preupgrade retained topic".into(),
                locale: "en".into(),
            },
        })
        .await
        .unwrap();
    let old = "a".repeat(64);
    store
        .query(
            "UPDATE library:main SET releaseChecksum=$old;",
            serde_json::json!({"old":old}),
        )
        .await
        .unwrap();
    store.initialize_schema().await.unwrap();
    let backups = std::fs::read_dir(&config.backup_dir)
        .unwrap()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 1);
    assert!(
        backups[0]
            .file_name()
            .to_str()
            .unwrap()
            .starts_with("pre-upgrade-")
    );
    let manifest: super::BackupManifest =
        serde_json::from_slice(&std::fs::read(backups[0].path().join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest.application_checksum, old);
    let restored = KnowledgeStore::restore(&backups[0].path(), &root.path().join("rollback"))
        .await
        .unwrap();
    assert_eq!(restored.counts, manifest.counts);
    store.close().await.unwrap();
}

#[tokio::test]
async fn local_worker_resumes_after_a_long_maintenance_pause() {
    let s = super::matching_fixture::Setup::new().await;
    s.create("Local maintenance question").await;
    let lease = s.store.maintenance.write().await;
    tokio::time::pause();
    let worker = crate::embeddings::EmbeddingWorker::start(
        s.store.clone(),
        s.embedder.clone(),
        tokio_util::sync::CancellationToken::new(),
    );
    tokio::task::yield_now().await;
    tokio::time::advance(std::time::Duration::from_secs(6)).await;
    tokio::task::yield_now().await;
    tokio::time::resume();
    drop(lease);
    super::matching_fixture::until(async || {
        s.store.library_info().await.unwrap().discovery
            == crate::contracts::library::DiscoveryStatus::Ready
    })
    .await;
    worker.shutdown().await.unwrap();
    s.store.close().await.unwrap();
}

#[tokio::test]
async fn graph_deadline_bounds_waiting_for_storage_admission() {
    use crate::{contracts::library::TopicId, knowledge::matching::GraphBounds};
    let root = tempfile::tempdir().unwrap();
    let config = StorageConfig::from_values(
        &HashMap::from([(
            "APP_DATA_DIR".into(),
            root.path().join("data").display().to_string(),
        )]),
        &root.path().join("release/branch-backend"),
    )
    .unwrap();
    let store = KnowledgeStore::open(&config).await.unwrap();
    // Native SDK setup uses external worker threads; pause only the deadline under test.
    tokio::time::pause();
    let permit = store.admission.acquire_many(16).await.unwrap();
    let started = tokio::time::Instant::now();
    let error = store
        .neighborhood(
            &TopicId::new("root").unwrap(),
            GraphBounds {
                hops: 3,
                nodes: 100,
                edges: 200,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), "STORAGE_BUSY");
    // Tokio's timer wheel rounds a deadline up to its one-millisecond tick.
    let elapsed = started.elapsed();
    assert!(elapsed >= std::time::Duration::from_secs(2));
    assert!(elapsed <= std::time::Duration::from_millis(2001));
    drop(permit);
    store.close().await.unwrap();
}

#[tokio::test]
async fn accepted_connections_keep_their_provenance_after_model_replacement() {
    use crate::contracts::library::ConnectionStatus;
    use serde_json::json;
    let s = super::matching_fixture::Setup::new().await;
    let (_, _, a) = s.create("Flash retention mechanism").await;
    let (_, _, b) = s.create("How flash retains information").await;
    let old = s.candidate(&a, &b).await;
    s.store
        .save_candidates(std::slice::from_ref(&old))
        .await
        .unwrap();
    s.store.query("UPDATE ONLY type::record('connection',$key) SET data.status='accepted',data.kind='equivalent_question'; UPDATE library:main SET activeModelVersion='replacement-model-generation';",json!({"key":old.id})).await.unwrap();
    let saved = s.store.connections(&a, None, 100).await.unwrap();
    assert_eq!(saved.items.len(), 1);
    assert_eq!(saved.items[0].status, ConnectionStatus::Accepted);
    assert_eq!(
        saved.items[0].model_version.as_deref(),
        Some(old.model_version.as_str())
    );
    s.store.close().await.unwrap();
}
