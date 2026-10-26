//! The sole embedded datastore and its bounded access boundary.
mod config;
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
    AppError::new(ErrorCode::InternalFailure)
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
