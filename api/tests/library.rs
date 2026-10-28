#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::{
    contracts::library::*,
    error::ErrorCode,
    storage::{KnowledgeStore, StorageConfig},
};
async fn store(root: &std::path::Path) -> KnowledgeStore {
    let store = KnowledgeStore::open(&StorageConfig {
        data_dir: root.join("data"),
        backup_dir: root.join("backups"),
        model_dir: root.join("models"),
        asset_dir: root.join("public"),
    })
    .await
    .unwrap();
    store.install_schema().await.unwrap();
    store
}
fn command<T>(payload: T, revision: Option<u64>) -> Command<T> {
    Command {
        operation_id: OperationId::generate(),
        library_epoch: 1,
        expected_revision: revision,
        payload,
    }
}
fn result(view: OperationView) -> (ExplorationId, AppearanceId, u64) {
    assert_eq!(view.state, OperationState::Completed);
    match view.result.unwrap() {
        OperationResult::Exploration {
            exploration_id,
            appearance_id,
            revision,
        } => (exploration_id, appearance_id, revision),
        _ => unreachable!(),
    }
}
async fn create(store: &KnowledgeStore, question: &str) -> (ExplorationId, AppearanceId, u64) {
    result(
        store
            .create_exploration(&command(
                CreateExploration {
                    question: question.into(),
                    locale: "en".into(),
                },
                None,
            ))
            .await
            .unwrap(),
    )
}
#[tokio::test]
async fn shared_topic_has_distinct_appearances() {
    let root = tempfile::tempdir().unwrap();
    let store = store(root.path()).await;
    let (a, ar, _) = create(&store, "Memory").await;
    let (b, br, _) = create(&store, "Memory").await;
    let (_, ai, _) = result(
        store
            .add_branch(&command(
                AddBranch {
                    parent_appearance_id: ar,
                    question: "How does flash work?".into(),
                },
                Some(0),
            ))
            .await
            .unwrap(),
    );
    let (_, bi, _) = result(
        store
            .add_branch(&command(
                AddBranch {
                    parent_appearance_id: br,
                    question: "How does flash work?".into(),
                },
                Some(0),
            ))
            .await
            .unwrap(),
    );
    let first = store.exploration(&a).await.unwrap();
    let second = store.exploration(&b).await.unwrap();
    let first_appearance = first.appearances.iter().find(|v| v.id == ai).unwrap();
    let second_appearance = second.appearances.iter().find(|v| v.id == bi).unwrap();
    assert_eq!(first_appearance.topic_id, second_appearance.topic_id);
    assert_ne!(first_appearance.id, second_appearance.id);
    assert_eq!(first_appearance.question, "How does flash work?");
    assert_eq!(second_appearance.question, "How does flash work?");
    for snapshot in [first, second] {
        assert_eq!(snapshot.schema_version, 2);
        assert_eq!(snapshot.appearances.len(), 2);
        let root = snapshot
            .appearances
            .iter()
            .find(|a| a.parent_id.is_none())
            .unwrap();
        assert_eq!(root.child_ids.len(), 1);
        assert_eq!(
            snapshot
                .appearances
                .iter()
                .find(|a| a.parent_id.is_some())
                .unwrap()
                .parent_id,
            Some(root.id.clone())
        );
    }
    store.close().await.unwrap();
}
#[tokio::test]
async fn different_context_is_not_identity() {
    let root = tempfile::tempdir().unwrap();
    let store = store(root.path()).await;
    let (a, ar, _) = create(&store, "Fruit").await;
    let (b, br, _) = create(&store, "Computing").await;
    store
        .add_branch(&command(
            AddBranch {
                parent_appearance_id: ar,
                question: "What is Apple?".into(),
            },
            Some(0),
        ))
        .await
        .unwrap();
    store
        .add_branch(&command(
            AddBranch {
                parent_appearance_id: br,
                question: "What is Apple?".into(),
            },
            Some(0),
        ))
        .await
        .unwrap();
    let first = store.exploration(&a).await.unwrap();
    let second = store.exploration(&b).await.unwrap();
    assert_ne!(
        first
            .appearances
            .iter()
            .find(|a| a.parent_id.is_some())
            .unwrap()
            .topic_id,
        second
            .appearances
            .iter()
            .find(|a| a.parent_id.is_some())
            .unwrap()
            .topic_id
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn delete_preserves_library_and_purge_fences_replay() {
    let root = tempfile::tempdir().unwrap();
    let store = store(root.path()).await;
    let (a, _, _) = create(&store, "Memory").await;
    let (b, _, _) = create(&store, "Memory").await;
    let topic = store.exploration(&a).await.unwrap().appearances[0]
        .topic_id
        .clone();
    store
        .delete_exploration(&command(
            DeleteExploration {
                exploration_id: a.clone(),
            },
            Some(0),
        ))
        .await
        .unwrap();
    assert_eq!(
        store.exploration(&a).await.unwrap_err().kind,
        ErrorCode::NotFound
    );
    assert_eq!(store.topic(&topic, None).await.unwrap().id, topic);
    assert_eq!(
        store.exploration(&b).await.unwrap().appearances[0].topic_id,
        topic
    );
    let epoch_before_purge = store.library_info().await.unwrap().library_epoch;
    let purge = command(
        PurgeLibrary {
            confirmation: PURGE_CONFIRMATION.into(),
        },
        None,
    );
    let saved = store.purge_library(&purge).await.unwrap();
    let epoch_after_purge = store.library_info().await.unwrap().library_epoch;
    assert!(epoch_after_purge > epoch_before_purge);
    assert_eq!(store.purge_library(&purge).await.unwrap().id, saved.id);
    assert_eq!(
        store.library_info().await.unwrap().library_epoch,
        epoch_after_purge
    );
    assert!(
        matches!(saved.result,Some(OperationResult::Purge{library_epoch}) if library_epoch==epoch_after_purge)
    );
    assert_eq!(
        store
            .create_exploration(&command(
                CreateExploration {
                    question: "Memory".into(),
                    locale: "en".into()
                },
                None
            ))
            .await
            .unwrap_err()
            .kind,
        ErrorCode::RevisionConflict
    );
    assert_eq!(
        store.topic(&topic, None).await.unwrap_err().kind,
        ErrorCode::NotFound
    );
    assert!(
        store
            .explorations(None, 100)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn revision_checks_and_replays_leave_tree_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let store = store(root.path()).await;
    let create = command(
        CreateExploration {
            question: "Memory".into(),
            locale: "en".into(),
        },
        None,
    );
    let (a, ar, _) = result(store.create_exploration(&create).await.unwrap());
    assert_eq!(
        result(store.create_exploration(&create).await.unwrap()).0,
        a
    );
    let add = command(
        AddBranch {
            parent_appearance_id: ar.clone(),
            question: "Flash".into(),
        },
        Some(0),
    );
    let saved = result(store.add_branch(&add).await.unwrap());
    assert_eq!(result(store.add_branch(&add).await.unwrap()).1, saved.1);
    let stale = command(
        AddBranch {
            parent_appearance_id: ar,
            question: "RAM".into(),
        },
        Some(0),
    );
    assert_eq!(
        store.add_branch(&stale).await.unwrap_err().kind,
        ErrorCode::RevisionConflict
    );
    let missing = command(
        AddBranch {
            parent_appearance_id: AppearanceId::generate(),
            question: "RAM".into(),
        },
        Some(1),
    );
    assert_eq!(
        store.add_branch(&missing).await.unwrap_err().kind,
        ErrorCode::NotFound
    );
    let snapshot = store.exploration(&a).await.unwrap();
    assert_eq!(snapshot.exploration.revision, 1);
    assert_eq!(snapshot.appearances.len(), 2);
    for limit in [1, 100] {
        assert_eq!(
            store.explorations(None, limit).await.unwrap().items.len(),
            1
        );
    }
    assert!(store.explorations(None, 101).await.is_err());
    let mut changed = create.clone();
    changed.payload.question = "other".into();
    assert_eq!(
        store.create_exploration(&changed).await.unwrap_err().kind,
        ErrorCode::IntentConflict
    );
    store.close().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_create_reuses_exact_identity_without_losing_appearances() {
    let root = tempfile::tempdir().unwrap();
    let store = std::sync::Arc::new(store(root.path()).await);
    let mut handles = vec![];
    for _ in 0..12 {
        let store = store.clone();
        handles.push(tokio::spawn(async move {
            store
                .create_exploration(&command(
                    CreateExploration {
                        question: "Flash".into(),
                        locale: "en".into(),
                    },
                    None,
                ))
                .await
        }));
    }
    let mut topics = std::collections::HashSet::new();
    for h in handles {
        let (e, _, _) = result(h.await.unwrap().unwrap());
        topics.insert(
            store.exploration(&e).await.unwrap().appearances[0]
                .topic_id
                .clone(),
        );
    }
    assert_eq!(topics.len(), 1);
    assert_eq!(store.explorations(None, 100).await.unwrap().items.len(), 12);
    let first = store.explorations(None, 1).await.unwrap();
    let second = store
        .explorations(first.next_cursor.as_deref(), 1)
        .await
        .unwrap();
    assert_ne!(first.items[0].id, second.items[0].id);
    store.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn simultaneous_branch_retries_replay_and_revision_races_do_not_lose_children() {
    let root = tempfile::tempdir().unwrap();
    let store = std::sync::Arc::new(store(root.path()).await);
    let (e, a, _) = create(&store, "Memory").await;
    let add = command(
        AddBranch {
            parent_appearance_id: a.clone(),
            question: "Flash".into(),
        },
        Some(0),
    );
    let mut handles = vec![];
    for _ in 0..12 {
        let store = store.clone();
        let add = add.clone();
        handles.push(tokio::spawn(async move { store.add_branch(&add).await }));
    }
    let mut ids = std::collections::HashSet::new();
    for h in handles {
        ids.insert(result(h.await.unwrap().unwrap()).1);
    }
    assert_eq!(ids.len(), 1);
    let mut handles = vec![];
    for question in ["RAM", "ROM"] {
        let store = store.clone();
        let a = a.clone();
        handles.push(tokio::spawn(async move {
            store
                .add_branch(&command(
                    AddBranch {
                        parent_appearance_id: a,
                        question: question.into(),
                    },
                    Some(1),
                ))
                .await
        }));
    }
    let mut successful = 0;
    for h in handles {
        match h.await.unwrap() {
            Ok(_) => successful += 1,
            Err(e) => assert_eq!(e.kind, ErrorCode::RevisionConflict),
        }
    }
    assert_eq!(successful, 1);
    let snapshot = store.exploration(&e).await.unwrap();
    assert_eq!(snapshot.exploration.revision, 2);
    assert_eq!(snapshot.appearances.len(), 3);
    let delete = command(DeleteExploration { exploration_id: e }, Some(2));
    let mut handles = vec![];
    for _ in 0..12 {
        let store = store.clone();
        let delete = delete.clone();
        handles.push(tokio::spawn(async move {
            store.delete_exploration(&delete).await
        }));
    }
    for h in handles {
        assert_eq!(h.await.unwrap().unwrap().id, delete.operation_id);
    }
    store.close().await.unwrap();
}

#[tokio::test]
async fn exploration_view_survives_reopen_without_changing_content_revision() {
    let root = tempfile::tempdir().unwrap();
    let db = store(root.path()).await;
    let (id, node, revision) = create(&db, "Server view").await;
    assert!(db.exploration_view(&id).await.unwrap().is_none());
    let view: ExplorationView = serde_json::from_value(serde_json::json!({
        "selectedNodeId": node, "expandedNodeIds": [node],
        "viewport": {"x": 42.0, "y": -12.0, "zoom": 0.75}, "mobilePane": "reader"
    }))
    .unwrap();
    db.save_exploration_view(&command(
        SaveView {
            exploration_id: id.clone(),
            view: view.clone(),
        },
        None,
    ))
    .await
    .unwrap();
    assert_eq!(
        db.exploration(&id).await.unwrap().exploration.revision,
        revision
    );
    db.close().await.unwrap();
    drop(db);
    let db = store(root.path()).await;
    assert_eq!(db.exploration_view(&id).await.unwrap(), Some(view));
}

#[tokio::test]
async fn exploration_view_rejects_foreign_nodes_and_stale_epoch() {
    let root = tempfile::tempdir().unwrap();
    let db = store(root.path()).await;
    let (id, node, _) = create(&db, "First").await;
    let (_, foreign, _) = create(&db, "Second").await;
    let view = |selected, expanded| {
        serde_json::from_value::<ExplorationView>(serde_json::json!({
            "selectedNodeId": selected, "expandedNodeIds": expanded,
            "viewport": {"x": 0, "y": 0, "zoom": 1}, "mobilePane": "map"
        }))
        .unwrap()
    };
    for bad in [
        view(foreign.clone(), vec![]),
        view(node.clone(), vec![foreign]),
    ] {
        assert!(
            db.save_exploration_view(&command(
                SaveView {
                    exploration_id: id.clone(),
                    view: bad
                },
                None
            ))
            .await
            .is_err()
        );
    }
    let mut stale = command(
        SaveView {
            exploration_id: id.clone(),
            view: view(node, vec![]),
        },
        None,
    );
    stale.library_epoch = 2;
    assert_eq!(
        db.save_exploration_view(&stale).await.unwrap_err().kind,
        ErrorCode::RevisionConflict
    );
    assert!(db.exploration_view(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn pending_operations_are_paginated_by_stable_id_and_exclude_completed_work() {
    use branch_backend::storage::operations::OperationIntent;
    let root = tempfile::tempdir().unwrap();
    let db = store(root.path()).await;
    create(&db, "Already completed").await;
    for id in ["pending-a", "pending-b", "pending-c"] {
        db.claim_operation(
            &OperationIntent::new(
                OperationId::new(id).unwrap(),
                1,
                serde_json::json!({}),
                "test",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    }
    let first = db.pending_operations(None, 2).await.unwrap();
    assert_eq!(
        first
            .items
            .iter()
            .map(|op| op.id.as_str())
            .collect::<Vec<_>>(),
        vec!["pending-a", "pending-b"]
    );
    assert_eq!(first.next_cursor.as_deref(), Some("pending-b"));
    let next = db
        .pending_operations(first.next_cursor.as_deref(), 2)
        .await
        .unwrap();
    assert_eq!(next.items[0].id.as_str(), "pending-c");
    assert!(next.next_cursor.is_none());
    assert!(db.pending_operations(None, 101).await.is_err());
}
