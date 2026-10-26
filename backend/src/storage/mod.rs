//! The sole embedded datastore and its bounded access boundary.
pub(crate) mod answers;
mod config;
mod connections;
mod embeddings;
mod imports;
mod library;
pub mod operations;
pub mod records;
mod schema;
pub use config::StorageConfig;

use crate::error::{AppError, ErrorCode};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    sync::Mutex,
    time::Duration,
};
use surrealdb::{
    Surreal,
    engine::local::{Db, RocksDb},
    opt::{Config, capabilities::Capabilities},
};
use tokio::sync::{RwLock, Semaphore};

const OWNER: &str = "branch-knowledge-storage-v1\n";
const OWNER_FILE: &str = ".branch-storage-owner";
const LOCK_FILE: &str = ".branch-storage.lock";
pub(crate) const STORAGE_TIMEOUT: Duration = Duration::from_secs(5);

fn storage_error() -> AppError {
    AppError::new(ErrorCode::StorageUnavailable)
}

// SurrealDB marks earlier transaction statements as NotExecuted after rollback.
// Select the substantive error so only genuine serialization conflicts retry.
fn root_query_error(mut errors: Vec<(usize, surrealdb::Error)>) -> Option<surrealdb::Error> {
    use surrealdb::types::QueryError;
    errors.sort_by_key(|(index, _)| *index);
    let substantive = errors.iter().position(|(_, e)| {
        !matches!(
            e.query_details(),
            Some(QueryError::NotExecuted | QueryError::Cancelled)
        )
    });
    substantive
        .or_else(|| (!errors.is_empty()).then_some(0))
        .map(|index| errors.remove(index).1)
}

pub struct KnowledgeStore {
    client: RwLock<Option<Surreal<Db>>>,
    pub(crate) admission: Semaphore,
    lock: Mutex<Option<File>>,
}

impl KnowledgeStore {
    pub async fn open(config: &StorageConfig) -> Result<Self, AppError> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&config.data_dir)
            .map_err(|_| storage_error())?;
        let owner = config.data_dir.join(OWNER_FILE);
        match std::fs::symlink_metadata(&owner) {
            Ok(metadata) => {
                if !metadata.is_file()
                    || metadata.file_type().is_symlink()
                    || std::fs::read_to_string(&owner).map_err(|_| storage_error())? != OWNER
                {
                    return Err(storage_error());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                for entry in std::fs::read_dir(&config.data_dir).map_err(|_| storage_error())? {
                    if entry.map_err(|_| storage_error())?.file_name() != LOCK_FILE {
                        return Err(storage_error());
                    }
                }
            }
            Err(_) => return Err(storage_error()),
        }
        let lock_path = config.data_dir.join(LOCK_FILE);
        if std::fs::symlink_metadata(&lock_path)
            .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err(storage_error());
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(lock_path)
            .map_err(|_| storage_error())?;
        rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)
            .map_err(|_| storage_error())?;
        // Recheck after acquiring the lock: another initializer may have written the marker.
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&owner)
        {
            Ok(mut marker) => {
                marker
                    .write_all(OWNER.as_bytes())
                    .map_err(|_| storage_error())?;
                marker.sync_all().map_err(|_| storage_error())?;
                File::open(&config.data_dir)
                    .and_then(|d| d.sync_all())
                    .map_err(|_| storage_error())?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if std::fs::read_to_string(&owner).map_err(|_| storage_error())? != OWNER {
                    return Err(storage_error());
                }
            }
            Err(_) => return Err(storage_error()),
        }
        std::fs::set_permissions(&config.data_dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| storage_error())?;
        let path = config.data_dir.join("store");
        let options = Config::new()
            .query_timeout(Duration::from_secs(30))
            .transaction_timeout(Duration::from_secs(30))
            .capabilities(Capabilities::default());
        let deadline = tokio::time::Instant::now() + STORAGE_TIMEOUT;
        let db = loop {
            match Surreal::new::<RocksDb>((path.as_path(), options.clone()))
                .with_capacity(32)
                .sync("every")
                .await
            {
                Ok(db) => break db,
                // SDK shutdown happens asynchronously when the last handle drops. Only
                // retry that engine lock after our own process lock has been acquired.
                Err(error)
                    if error.to_string().to_lowercase().contains("lock")
                        && tokio::time::Instant::now() < deadline =>
                {
                    tokio::time::sleep(Duration::from_millis(20)).await
                }
                Err(_) => return Err(storage_error()),
            }
        };
        db.use_ns("branch")
            .use_db("knowledge")
            .await
            .map_err(|_| storage_error())?;
        db.query("DEFINE NAMESPACE IF NOT EXISTS branch; DEFINE DATABASE IF NOT EXISTS knowledge;")
            .await
            .map_err(|_| storage_error())?
            .check()
            .map_err(|_| storage_error())?;
        Ok(Self {
            client: RwLock::new(Some(db)),
            admission: Semaphore::new(16),
            lock: Mutex::new(Some(lock)),
        })
    }

    /// Only repository-owned SQL enters here; values always use bound parameters.
    /// Retry local serialization conflicts only. No external effect is in this boundary.
    pub(crate) async fn query(
        &self,
        sql: &str,
        bindings: serde_json::Value,
    ) -> Result<surrealdb::IndexedResults, AppError> {
        let _permit = tokio::time::timeout(STORAGE_TIMEOUT, self.admission.acquire())
            .await
            .map_err(|_| AppError::new(ErrorCode::StorageBusy))?
            .map_err(|_| storage_error())?;
        let guard = self.client.read().await;
        let db = guard.as_ref().ok_or_else(storage_error)?;
        for attempt in 0..3 {
            let result = db
                .query(sql)
                .bind(bindings.clone())
                .await
                .and_then(|mut response| {
                    match root_query_error(response.take_errors().into_iter().collect()) {
                        Some(error) => Err(error),
                        None => Ok(response),
                    }
                });
            match result {
                Ok(value) => return Ok(value),
                Err(error)
                    if error.query_details()
                        == Some(&surrealdb::types::QueryError::TransactionConflict) =>
                {
                    if attempt == 2 {
                        return Err(AppError::new(ErrorCode::StorageBusy));
                    }
                    tokio::task::yield_now().await;
                }
                Err(error) => {
                    if error.is_thrown() {
                        let kind = match error
                            .message()
                            .strip_prefix("An error occurred: ")
                            .unwrap_or(error.message())
                        {
                            "library_epoch_changed" | "revision_conflict" => {
                                ErrorCode::RevisionConflict
                            }
                            "intent_conflict" => ErrorCode::IntentConflict,
                            "missing_reference" => ErrorCode::NotFound,
                            "content_limit" => ErrorCode::BodyTooLarge,
                            "research_limit" => ErrorCode::ResearchLimit,
                            "work_interrupted" => ErrorCode::WorkInterrupted,
                            "invalid_import" | "import_limit" => ErrorCode::InvalidImport,
                            _ => ErrorCode::StorageUnavailable,
                        };
                        return Err(AppError::new(kind));
                    }
                    tracing::error!(
                        error_kind = error.kind_str(),
                        query_timed_out = matches!(
                            error.query_details(),
                            Some(surrealdb::types::QueryError::TimedOut { .. })
                        ),
                        "library query failed"
                    );
                    return Err(storage_error());
                }
            }
        }
        Err(AppError::new(ErrorCode::StorageBusy))
    }

    /// Finish outstanding local queries and drop the sole SDK connection.
    /// Every successful transaction already used the engine's `sync=every`.
    pub async fn close(&self) -> Result<(), AppError> {
        self.admission.close();
        drop(self.client.write().await.take());
        tokio::task::yield_now().await;
        self.lock.lock().map_err(|_| storage_error())?.take();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
