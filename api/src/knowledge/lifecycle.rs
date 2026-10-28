//! Runtime-owned local work and daily backups. No paid stage is resumed here.
use super::KnowledgeService;
use crate::{
    config::Config,
    contracts::library::DiscoveryStatus,
    embeddings::EmbeddingWorker,
    error::{AppError, ErrorCode},
    storage::{BackupManifest, KnowledgeStore},
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
#[derive(Debug, Clone)]
pub struct KnowledgeStatus {
    pub storage_ready: bool,
    pub library_ready: bool,
    pub discovery: DiscoveryStatus,
    pub backup_age_seconds: Option<u64>,
    pub backup_overdue: bool,
    pub backup_failed: bool,
}
#[derive(Default)]
struct BackupState {
    last_success: Option<i64>,
    failed: bool,
}
pub struct KnowledgeRuntime {
    store: Arc<KnowledgeStore>,
    service: Arc<KnowledgeService>,
    cancel: CancellationToken,
    worker: Mutex<Option<EmbeddingWorker>>,
    task: Mutex<Option<JoinHandle<()>>>,
    backup: Arc<Mutex<BackupState>>,
}
fn completed_daily(root: &Path) -> Vec<(PathBuf, BackupManifest)> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return vec![];
    };
    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_name().to_str()?.starts_with("daily-")
                || !entry.file_type().ok()?.is_dir()
            {
                return None;
            }
            let path = entry.path();
            let marker = path.join(".branch-backup-owner");
            if std::fs::symlink_metadata(&marker)
                .ok()?
                .file_type()
                .is_symlink()
                || std::fs::read_to_string(marker).ok()?.trim() != "branch-native-backup-v1"
            {
                return None;
            }
            let bytes = std::fs::read(path.join("manifest.json")).ok()?;
            if bytes.len() > 65536 {
                return None;
            }
            let manifest: BackupManifest = serde_json::from_slice(&bytes).ok()?;
            Some((path, manifest))
        })
        .collect()
}
fn seconds(timestamp: &str) -> Option<i64> {
    time::OffsetDateTime::parse(timestamp, &time::format_description::well_known::Rfc3339)
        .ok()
        .map(|v| v.unix_timestamp())
}
impl KnowledgeRuntime {
    pub async fn start(
        config: Arc<Config>,
        store: Arc<KnowledgeStore>,
        service: Arc<KnowledgeService>,
        worker: Option<EmbeddingWorker>,
        cancel: CancellationToken,
    ) -> Result<Self, AppError> {
        store.initialize_schema().await?;
        store.recover_interrupted().await?;
        store.cleanup_imports().await?;
        if worker.is_none() {
            store
                .set_discovery(DiscoveryStatus::Unavailable, None)
                .await?;
        }
        let info = store.library_info().await?;
        let prior = completed_daily(&config.storage.backup_dir)
            .into_iter()
            .filter(|(_, m)| m.library_id == info.library_id)
            .filter_map(|(_, m)| seconds(&m.created_at))
            .max();
        let backup = Arc::new(Mutex::new(BackupState {
            last_success: prior,
            failed: false,
        }));
        let owned = cancel.child_token();
        let loop_cancel = owned.clone();
        let state = backup.clone();
        let db = store.clone();
        let root = config.storage.backup_dir.clone();
        let library_id = info.library_id;
        let task = tokio::spawn(async move {
            loop {
                let now = time::OffsetDateTime::now_utc().unix_timestamp();
                let due = state
                    .lock()
                    .map(|s| {
                        s.last_success
                            .is_none_or(|v| now.saturating_sub(v) >= 86400)
                    })
                    .unwrap_or(true);
                if due {
                    let path = root.join(format!(
                        "daily-{}-{}",
                        time::OffsetDateTime::now_utc().date(),
                        uuid::Uuid::new_v4()
                    ));
                    let result = tokio::select! {_=loop_cancel.cancelled()=>return,result=db.backup(&path)=>result};
                    match result {
                        Ok(manifest) => {
                            if let Ok(mut state) = state.lock() {
                                state.last_success = seconds(&manifest.created_at);
                                state.failed = false;
                            }
                            let mut backups = completed_daily(&root)
                                .into_iter()
                                .filter(|(_, m)| m.library_id == library_id)
                                .collect::<Vec<_>>();
                            backups.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));
                            for (path, _) in backups.into_iter().skip(7) {
                                if std::fs::remove_dir_all(path).is_err() {
                                    tracing::warn!(
                                        event = "backup_retention_failed",
                                        code = "STORAGE_UNAVAILABLE"
                                    );
                                }
                            }
                            tracing::info!(event = "knowledge_backup_completed");
                        }
                        Err(error) => {
                            if error.kind != ErrorCode::StorageBusy {
                                if let Ok(mut state) = state.lock() {
                                    state.failed = true;
                                }
                                tracing::warn!(
                                    event = "knowledge_backup_failed",
                                    code = error.code()
                                );
                            }
                        }
                    }
                }
                tokio::select! {_=loop_cancel.cancelled()=>return,_=tokio::time::sleep(std::time::Duration::from_secs(60))=>{}}
            }
        });
        Ok(Self {
            store,
            service,
            cancel: owned,
            worker: Mutex::new(worker),
            task: Mutex::new(Some(task)),
            backup,
        })
    }
    pub async fn status(&self) -> KnowledgeStatus {
        let info = self.store.library_info().await.ok();
        let age = self
            .backup
            .lock()
            .ok()
            .and_then(|s| s.last_success)
            .map(|t| {
                time::OffsetDateTime::now_utc()
                    .unix_timestamp()
                    .saturating_sub(t)
                    .max(0) as u64
            });
        let failed = self.backup.lock().map(|s| s.failed).unwrap_or(true);
        KnowledgeStatus {
            storage_ready: info.is_some() && !self.cancel.is_cancelled(),
            library_ready: info.is_some()
                && !self.cancel.is_cancelled()
                && !self
                    .service
                    .failed
                    .load(std::sync::atomic::Ordering::SeqCst),
            discovery: info
                .map(|i| i.discovery)
                .unwrap_or(DiscoveryStatus::Unavailable),
            backup_age_seconds: age,
            backup_overdue: age.is_none_or(|v| v >= 86400),
            backup_failed: failed,
        }
    }
    pub async fn shutdown(&self) -> Result<(), AppError> {
        self.cancel.cancel();
        let task = self
            .task
            .lock()
            .map_err(|_| AppError::new(ErrorCode::InternalFailure))?
            .take();
        if let Some(task) = task {
            task.await
                .map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
        }
        let service_result = self.service.drain().await;
        let worker = self
            .worker
            .lock()
            .map_err(|_| AppError::new(ErrorCode::InternalFailure))?
            .take();
        let worker_result = if let Some(worker) = worker {
            worker.shutdown().await
        } else {
            Ok(())
        };
        let storage_result = self.store.close().await;
        service_result.and(worker_result).and(storage_result)
    }
}
