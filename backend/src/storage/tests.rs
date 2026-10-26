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
