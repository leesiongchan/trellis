#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/matching.rs"]
mod fixture;
#[path = "support/saved_provider.rs"]
mod provider_fixture;
use branch_backend::{
    config::Limits,
    contracts::library::*,
    embeddings::{Embedder, SearchFilter, SearchVector},
    jobs::Coordinator,
    knowledge::KnowledgeService,
    storage::{KnowledgeStore, StorageConfig},
};
use serde_json::{Value, json};
#[tokio::test]
async fn backup_restores_graph_versions_sources_and_vectors_into_fresh_storage() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("branch_backend=debug")
        .try_init();
    let s = fixture::Setup::new().await;
    let (e, a, topic) = s.create("How does NAND flash store data?").await;
    let provider = provider_fixture::Provider::new();
    let service = KnowledgeService::new(
        s.store.clone(),
        Coordinator::new(provider.clone(), Limits::default()),
        provider.clone(),
    );
    let input = fixture::command(
        ResearchInput {
            appearance_id: a.clone(),
            refresh: false,
            clarification: None,
        },
        Some(0),
    );
    service.start_research(input).await.unwrap();
    provider.release.add_permits(1);
    service.drain().await.unwrap();
    let worker = s.ready().await;
    fixture::until(async || {
        !s.store
            .nearest(
                &SearchVector::new(s.embedder.manifest().version(), vec![1.; 384], 384).unwrap(),
                &SearchFilter::passages(),
                10,
            )
            .await
            .unwrap()
            .is_empty()
    })
    .await;
    worker.shutdown().await.unwrap();
    let before = s.store.exploration(&e).await.unwrap();
    let output = s.root.path().join("snapshot");
    let original = s.store.backup(&output).await.unwrap();
    assert!(original.counts["answer_version"] > 0);
    assert!(original.counts["connection"] > 0);
    assert!(original.counts["embedding"] > 0);
    assert_eq!(original.source_library_epoch, 1);
    let target = s.root.path().join("restored");
    let restored = KnowledgeStore::restore(&output, &target).await.unwrap();
    let original_counts = original.counts;
    let restored_counts = restored.counts;
    assert_eq!(restored_counts, original_counts);
    let store = KnowledgeStore::open(&StorageConfig {
        data_dir: target,
        backup_dir: s.root.path().join("other-backups"),
        model_dir: s.root.path().join("other-models"),
        asset_dir: s.root.path().join("other-public"),
    })
    .await
    .unwrap();
    store.install_schema().await.unwrap();
    let after = store.exploration(&e).await.unwrap();
    assert_ne!(after.library_epoch, before.library_epoch);
    assert_eq!(
        serde_json::to_value(after.appearances).unwrap(),
        serde_json::to_value(before.appearances).unwrap()
    );
    assert_eq!(
        serde_json::to_value(after.answers).unwrap(),
        serde_json::to_value(before.answers).unwrap()
    );
    assert!(
        store
            .topic(&topic, None)
            .await
            .unwrap()
            .selected_answer
            .is_some()
    );
    let hits = store
        .nearest(
            &SearchVector::new(s.embedder.manifest().version(), vec![1.; 384], 384).unwrap(),
            &SearchFilter::passages(),
            10,
        )
        .await
        .unwrap();
    assert!(!hits.is_empty());
    assert!(
        KnowledgeStore::restore(&output, &store_path(&s, "restored"))
            .await
            .is_err()
    );
    store.close().await.unwrap();
    s.store.close().await.unwrap();
}
fn store_path(s: &fixture::Setup, name: &str) -> std::path::PathBuf {
    s.root.path().join(name)
}
#[tokio::test]
async fn restore_rejects_tampered_wrong_version_symlink_and_overlapping_targets() {
    let s = fixture::Setup::new().await;
    s.create("Question").await;
    let backup = s.root.path().join("snapshot");
    s.store.backup(&backup).await.unwrap();
    assert!(s.store.backup(&backup).await.is_err());
    assert!(
        s.store
            .backup(&s.root.path().join("data/inside"))
            .await
            .is_err()
    );
    assert!(
        KnowledgeStore::restore(&backup, &backup.join("inside"))
            .await
            .is_err()
    );
    std::os::unix::fs::symlink(s.root.path().join("empty"), s.root.path().join("linked")).unwrap();
    assert!(
        KnowledgeStore::restore(&backup, &s.root.path().join("linked"))
            .await
            .is_err()
    );
    let manifest = backup.join("manifest.json");
    let original = std::fs::read(&manifest).unwrap();
    let mut bad: Value = serde_json::from_slice(&original).unwrap();
    bad["engineVersion"] = json!("0.0.0");
    std::fs::write(&manifest, bad.to_string()).unwrap();
    assert!(
        KnowledgeStore::restore(&backup, &s.root.path().join("bad-engine"))
            .await
            .is_err()
    );
    std::fs::write(&manifest, &original).unwrap();
    let dump = backup.join("database.surql");
    let mut bytes = std::fs::read(&dump).unwrap();
    bytes.push(b' ');
    std::fs::write(&dump, bytes).unwrap();
    assert!(
        KnowledgeStore::restore(&backup, &s.root.path().join("bad-checksum"))
            .await
            .is_err()
    );
    assert!(!s.root.path().join("bad-checksum").exists());
    s.store.close().await.unwrap();
}
#[tokio::test]
async fn backup_refuses_active_paid_research() {
    let s = fixture::Setup::new().await;
    let (_, a, _) = s.create("Question").await;
    let p = provider_fixture::Provider::new();
    let jobs = Coordinator::new(p.clone(), Limits::default());
    let service = KnowledgeService::new(s.store.clone(), jobs.clone(), p.clone());
    service
        .start_research(fixture::command(
            ResearchInput {
                appearance_id: a,
                refresh: false,
                clarification: None,
            },
            Some(0),
        ))
        .await
        .unwrap();
    fixture::until(async || jobs.snapshot().active == 1).await;
    assert!(s.store.backup(&s.root.path().join("busy")).await.is_err());
    assert!(!s.root.path().join("busy").exists());
    p.release.add_permits(1);
    service.drain().await.unwrap();
    s.store.close().await.unwrap();
}

#[tokio::test]
async fn daily_retention_keeps_seven_completed_snapshots_and_the_preupgrade_backup() {
    use branch_backend::{config::Config, knowledge::KnowledgeRuntime};
    use std::sync::Arc;
    let s = fixture::Setup::new().await;
    s.create("Retained question").await;
    let root = s.root.path();
    let pre = s
        .store
        .backup(&root.join("backups/pre-upgrade-kept"))
        .await
        .unwrap();
    for day in 1..=8 {
        let path = root.join(format!("backups/daily-fixture-{day}"));
        s.store.backup(&path).await.unwrap();
        let file = path.join("manifest.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        manifest["createdAt"] = json!(format!("2026-10-{day:02}T00:00:00.000000000Z"));
        std::fs::write(file, manifest.to_string()).unwrap();
    }
    let config = Arc::new(
        Config::from_values(
            &std::collections::HashMap::from([
                ("APP_ORIGIN".into(), "http://127.0.0.1:3000".into()),
                (
                    "APP_DATA_DIR".into(),
                    root.join("data").display().to_string(),
                ),
            ]),
            &root.join("release/app"),
        )
        .unwrap(),
    );
    let provider = provider_fixture::Provider::new();
    let service = Arc::new(KnowledgeService::new(
        s.store.clone(),
        Coordinator::new(provider.clone(), config.limits.clone()),
        provider,
    ));
    let runtime = KnowledgeRuntime::start(
        config,
        s.store.clone(),
        service,
        None,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .unwrap();
    fixture::until(async || {
        let status = runtime.status().await;
        !status.backup_overdue
            && std::fs::read_dir(root.join("backups"))
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .is_some_and(|v| v.starts_with("daily-"))
                })
                .count()
                == 7
    })
    .await;
    assert!(root.join("backups/pre-upgrade-kept/manifest.json").exists());
    assert_eq!(pre.source_library_epoch, 1);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn standalone_cli_requires_exclusive_source_and_restores_without_web_configuration() {
    let s = fixture::Setup::new().await;
    let (e, _, _) = s.create("CLI retained topic").await;
    let output = s.root.path().join("cli-backup");
    let run_backup = || {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_branch-backend"));
        command
            .args(["storage", "backup", "--output"])
            .arg(&output)
            .env_clear()
            .env("APP_ENV_FILE", "")
            .env("APP_DATA_DIR", s.root.path().join("data"))
            .env("TOKIO_WORKER_THREADS", "2")
            .env("PATH", "/nonexistent");
        command
    };
    let mut locked_command = run_backup();
    let locked = tokio::task::spawn_blocking(move || locked_command.output())
        .await
        .unwrap()
        .unwrap();
    assert!(!locked.status.success());
    assert!(String::from_utf8_lossy(&locked.stderr).contains("STORAGE_LOCKED"));
    assert!(!String::from_utf8_lossy(&locked.stderr).contains(s.root.path().to_str().unwrap()));
    s.store.close().await.unwrap();
    // SDK drop releases RocksDB asynchronously; keep this test runtime moving
    // while the standalone process acquires the now-unowned directory.
    let mut backup_command = run_backup();
    let saved = tokio::task::spawn_blocking(move || backup_command.output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );
    let manifest: Value = serde_json::from_slice(&saved.stdout).unwrap();
    assert_eq!(manifest["engineVersion"], "3.3.4");
    let target = s.root.path().join("cli-restored");
    let mut restore_command = std::process::Command::new(env!("CARGO_BIN_EXE_branch-backend"));
    restore_command
        .args(["storage", "restore", "--input"])
        .arg(&output)
        .arg("--data-dir")
        .arg(&target)
        .env_clear()
        .env("APP_ENV_FILE", "")
        .env("TOKIO_WORKER_THREADS", "2")
        .env("PATH", "/nonexistent");
    let restored = tokio::task::spawn_blocking(move || restore_command.output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let db = KnowledgeStore::open(&StorageConfig {
        data_dir: target,
        backup_dir: s.root.path().join("verify-backups"),
        model_dir: s.root.path().join("verify-models"),
        asset_dir: s.root.path().join("verify-public"),
    })
    .await
    .unwrap();
    assert_eq!(
        db.exploration(&e)
            .await
            .unwrap()
            .exploration
            .original_prompt,
        "CLI retained topic"
    );
    assert!(db.library_info().await.unwrap().library_epoch > 1);
    db.close().await.unwrap();
}
