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
