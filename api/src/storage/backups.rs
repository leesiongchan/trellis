//! Native consistent exports, verified imports, and a fenced fresh-directory restore.
use super::*;
use crate::contracts::library::MAX_SAFE_INTEGER;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;
pub const ENGINE_VERSION: &str = "3.3.4";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupManifest {
    pub library_id: crate::contracts::library::LibraryId,
    pub format_version: u8,
    pub schema_version: u64,
    pub engine_version: String,
    pub application_version: String,
    pub application_checksum: String,
    pub model_version: Option<String>,
    pub created_at: String,
    pub source_library_epoch: u64,
    pub sha256: String,
    pub bytes: u64,
    pub counts: BTreeMap<String, u64>,
}
const TABLES: &[&str] = &[
    "library",
    "topic",
    "question_variant",
    "exploration",
    "appearance",
    "answer_version",
    "operation",
    "research_attempt",
    "research_follower",
    "active_attempt",
    "connection",
    "edge_support",
    "embedding",
    "embedding_model",
    "embedding_state",
    "embedding_cursor",
    "match_state",
    "verification_stage",
    "migration_manifest",
    "migration_stage",
    "import_receipt",
];
async fn file_hash(path: PathBuf) -> Result<(String, u64), AppError> {
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut file = File::open(path).map_err(|_| storage_error())?;
        let mut hash = Sha256::new();
        let mut size = 0;
        let mut buffer = [0u8; 65536];
        loop {
            let n = file.read(&mut buffer).map_err(|_| storage_error())?;
            if n == 0 {
                break;
            }
            size += n as u64;
            hash.update(&buffer[..n]);
        }
        Ok((hex::encode(hash.finalize()), size))
    })
    .await
    .map_err(|_| storage_error())?
}
pub(crate) async fn executable_hash() -> Result<String, AppError> {
    static CHECKSUM: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();
    CHECKSUM
        .get_or_try_init(|| async {
            Ok(
                file_hash(std::env::current_exe().map_err(|_| storage_error())?)
                    .await?
                    .0,
            )
        })
        .await
        .cloned()
}
struct PendingBackup(PathBuf);
impl Drop for PendingBackup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fresh_path(path: &Path) -> Result<PathBuf, AppError> {
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(AppError::new(ErrorCode::InvalidConfig));
    }
    config::absolute(path)
}
impl KnowledgeStore {
    async fn backup_counts(&self) -> Result<BTreeMap<String, u64>, AppError> {
        let entries = TABLES
            .iter()
            .map(|t| format!("{t}:((SELECT count() AS total FROM {t} GROUP ALL)[0].total ?? 0)"))
            .collect::<Vec<_>>()
            .join(",");
        let value = self
            .query_value(&format!("RETURN {{{entries}}};"), json!({}))
            .await?;
        serde_json::from_value(value).map_err(|_| storage_error())
    }
    pub async fn backup(&self, destination: &Path) -> Result<BackupManifest, AppError> {
        self.check_schema().await?;
        let destination = fresh_path(destination)?;
        for reserved in [
            &self.config.data_dir,
            &self.config.model_dir,
            &self.config.asset_dir,
        ] {
            let reserved = config::absolute(reserved)?;
            if destination.starts_with(&reserved) || reserved.starts_with(&destination) {
                return Err(AppError::new(ErrorCode::InvalidConfig));
            }
        }
        config::validate_repository_roots(
            &destination,
            &destination,
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .ok_or_else(storage_error)?,
        )?;
        let _lease = tokio::time::timeout(STORAGE_TIMEOUT, self.maintenance.write())
            .await
            .map_err(|_| AppError::new(ErrorCode::StorageBusy))?;
        let active = self
            .query_value("RETURN array::concat((SELECT VALUE key FROM active_attempt LIMIT 1),(SELECT VALUE key FROM research_attempt WHERE data.state='pending' LIMIT 1));", json!({}))
            .await?;
        if active.as_array().is_none_or(|a| !a.is_empty()) {
            return Err(AppError::new(ErrorCode::StorageBusy));
        }
        let parent = destination.parent().ok_or_else(storage_error)?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)
            .map_err(|_| storage_error())?;
        let temporary = parent.join(format!(".branch-backup-{}", uuid::Uuid::new_v4()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&temporary)
            .map_err(|_| storage_error())?;
        let _cleanup = PendingBackup(temporary.clone());
        let result = async {
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(temporary.join("database.surql"))
                .await
                .map_err(|_| storage_error())?;
            let guard = self.client.read().await;
            let db = guard.as_ref().ok_or_else(storage_error)?;
            tracing::debug!(event = "backup_export_started");
            let mut stream = db
                .export(())
                .with_config()
                .tables(true)
                .records(true)
                .versions(false)
                .users(false)
                .accesses(false)
                .await
                .map_err(|_| storage_error())?;
            let mut hash = Sha256::new();
            let mut bytes = 0;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|_| storage_error())?;
                file.write_all(&chunk).await.map_err(|_| storage_error())?;
                hash.update(&chunk);
                bytes += chunk.len() as u64;
            }
            tracing::debug!(event = "backup_export_finished");
            file.sync_all().await.map_err(|_| storage_error())?;
            drop(file);
            drop(guard);
            let info = self.library_info().await?;
            tracing::debug!(event = "backup_library_metadata_read");
            let model = self
                .query_value(
                    "SELECT VALUE activeModelVersion FROM ONLY library:main;",
                    json!({}),
                )
                .await?;
            tracing::debug!(event = "backup_model_metadata_read");
            let previous_release = self
                .query_value(
                    "SELECT VALUE releaseChecksum FROM ONLY library:main;",
                    json!({}),
                )
                .await?;
            let checksum = match previous_release.as_str() {
                Some(value) => value.to_owned(),
                None => executable_hash().await?,
            };
            let manifest = BackupManifest {
                library_id: info.library_id,
                format_version: 1,
                schema_version: schema::SCHEMA_VERSION,
                engine_version: ENGINE_VERSION.into(),
                application_version: env!("CARGO_PKG_VERSION").into(),
                application_checksum: checksum,
                model_version: model.as_str().map(str::to_owned),
                created_at: records::now()?,
                source_library_epoch: info.library_epoch,
                sha256: hex::encode(hash.finalize()),
                bytes,
                counts: self.backup_counts().await?,
            };
            let mut metadata = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(temporary.join("manifest.json"))
                .map_err(|_| storage_error())?;
            metadata
                .write_all(&serde_json::to_vec_pretty(&manifest).map_err(|_| storage_error())?)
                .map_err(|_| storage_error())?;
            metadata.sync_all().map_err(|_| storage_error())?;
            let mut owner = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(temporary.join(".branch-backup-owner"))
                .map_err(|_| storage_error())?;
            owner
                .write_all(b"branch-native-backup-v1\n")
                .map_err(|_| storage_error())?;
            owner.sync_all().map_err(|_| storage_error())?;
            File::open(&temporary)
                .and_then(|f| f.sync_all())
                .map_err(|_| storage_error())?;
            // The destination is fresh. Rename only after export, hash, manifest and fsync.
            if std::fs::symlink_metadata(&destination).is_ok() {
                return Err(storage_error());
            }
            rustix::fs::renameat_with(
                rustix::fs::CWD,
                &temporary,
                rustix::fs::CWD,
                &destination,
                rustix::fs::RenameFlags::NOREPLACE,
            )
            .map_err(|_| storage_error())?;
            File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|_| storage_error())?;
            Ok(manifest)
        }
        .await;
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&temporary);
        }
        result
    }
    pub async fn restore(source: &Path, destination: &Path) -> Result<BackupManifest, AppError> {
        if std::fs::symlink_metadata(source)
            .map_or(true, |m| !m.is_dir() || m.file_type().is_symlink())
        {
            return Err(AppError::new(ErrorCode::InvalidConfig));
        }
        let source = std::fs::canonicalize(source).map_err(|_| storage_error())?;
        let destination = fresh_path(destination)?;
        if source.starts_with(&destination) || destination.starts_with(&source) {
            return Err(AppError::new(ErrorCode::InvalidConfig));
        }
        for name in ["manifest.json", "database.surql"] {
            if std::fs::symlink_metadata(source.join(name))
                .map_or(true, |m| !m.is_file() || m.file_type().is_symlink())
            {
                return Err(storage_error());
            }
        }
        let metadata = std::fs::read(source.join("manifest.json")).map_err(|_| storage_error())?;
        if metadata.len() > 65536 {
            return Err(storage_error());
        }
        let manifest: BackupManifest = serde_json::from_slice(&metadata)
            .map_err(|_| AppError::new(ErrorCode::InvalidBackup))?;
        if manifest.format_version != 1 {
            return Err(AppError::new(ErrorCode::InvalidBackup));
        }
        if manifest.schema_version != schema::SCHEMA_VERSION
            || manifest.engine_version != ENGINE_VERSION
        {
            return Err(AppError::new(ErrorCode::StorageVersionMismatch));
        }
        let (hash, bytes) = file_hash(source.join("database.surql")).await?;
        if hash != manifest.sha256 || bytes != manifest.bytes {
            return Err(AppError::new(ErrorCode::InvalidBackup));
        }
        let executable = std::env::current_exe().map_err(|_| storage_error())?;
        let storage = StorageConfig::from_values(
            &std::collections::HashMap::from([(
                "APP_DATA_DIR".into(),
                destination.display().to_string(),
            )]),
            &executable,
        )?;
        let store = Self::open(&storage).await?;
        let marker = destination.join(".branch-restore-in-progress");
        let mut progress = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&marker)
            .map_err(|_| storage_error())?;
        progress
            .write_all(b"branch-native-restore-v1\n")
            .map_err(|_| storage_error())?;
        progress.sync_all().map_err(|_| storage_error())?;
        let result=async{
            {let guard=store.client.read().await;let db=guard.as_ref().ok_or_else(storage_error)?;db.import(source.join("database.surql")).await.map_err(|error|{tracing::error!(event="native_restore_failed",stage="import",kind=error.kind_str());storage_error()})?;}
            tracing::debug!(event="restore_import_finished");
            store.check_schema().await?;
            tracing::debug!(event="restore_schema_checked");
            if store.library_info().await?.library_id!=manifest.library_id || store.backup_counts().await?!=manifest.counts||store.library_info().await?.library_epoch!=manifest.source_library_epoch{return Err(AppError::new(ErrorCode::InvalidBackup));}
            let model=store.query_value("RETURN library:main.activeModelVersion;",json!({})).await?;
            if model.as_str()!=manifest.model_version.as_deref(){return Err(AppError::new(ErrorCode::InvalidBackup));}
            let now=u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp_nanos()/1_000_000).map_err(|_|storage_error())?;
            let epoch=now.max(manifest.source_library_epoch.checked_add(1).ok_or_else(storage_error)?);if epoch>MAX_SAFE_INTEGER{return Err(storage_error());}
            store.query("BEGIN TRANSACTION; UPDATE library:main SET epoch=$epoch; UPDATE migration_manifest SET data.libraryEpoch=$epoch; UPDATE migration_manifest SET data.view.state='failed',data.view.error=$error WHERE data.view.state='staging'; DELETE migration_stage; COMMIT TRANSACTION;",json!({"epoch":epoch,"error":crate::contracts::library::OperationError::from(AppError::new(ErrorCode::WorkInterrupted))})).await?;
            tracing::debug!(event="restore_counts_verified");
            store.recover_interrupted().await?;
            tracing::debug!(event="restore_recovery_finished");Ok(manifest.clone())
        }.await;
        let close = store.close().await;
        result?;
        close?;
        std::fs::remove_file(marker).map_err(|_| storage_error())?;
        File::open(&destination)
            .and_then(|f| f.sync_all())
            .map_err(|_| storage_error())?;
        Ok(manifest)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::contracts::{legacy::LegacyExploration, library::*};
    use crate::embeddings::Embedder;
    use serde_json::Value;
    fn command<T>(payload: T) -> Command<T> {
        Command {
            operation_id: OperationId::generate(),
            library_epoch: 1,
            expected_revision: None,
            payload,
        }
    }
    #[tokio::test]
    async fn native_restored_retrieval_uses_normalized_paths() {
        let s = super::super::matching_fixture::Setup::new().await;
        let raw: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/library/migration.json"
        ))
        .unwrap();
        let legacy = LegacyExploration::validate(raw.clone()).unwrap();
        let encoded = raw.to_string();
        let v = s
            .store
            .begin_import(&command(BeginImport {
                legacy_exploration_id: legacy.id.clone(),
                content_digest: legacy.digest(),
                expected_bytes: encoded.len() as u64,
                expected_nodes: 3,
                explicit_restore: false,
            }))
            .await
            .unwrap();
        s.store
            .stage_import(&command(ImportBatch {
                import_id: v.id.clone(),
                batch_index: 0,
                records: vec![json!(encoded)],
            }))
            .await
            .unwrap();
        s.store
            .finalize_import(&command(FinalizeImport { import_id: v.id }))
            .await
            .unwrap();
        let worker = s.ready().await;
        worker.shutdown().await.unwrap();
        let output = s.root.path().join("backup");
        s.store.backup(&output).await.unwrap();
        let destination = s.root.path().join("restored");
        KnowledgeStore::restore(&output, &destination)
            .await
            .unwrap();
        let store = KnowledgeStore::open(&StorageConfig {
            data_dir: destination,
            backup_dir: s.root.path().join("other-backups"),
            model_dir: s.root.path().join("other-models"),
            asset_dir: s.root.path().join("other-public"),
        })
        .await
        .unwrap();
        store.install_schema().await.unwrap();
        let sql = format!(
            "SELECT key,sourceKey,inputHash,data FROM embedding WHERE {} ORDER BY key ASC LIMIT 512;",
            super::super::embeddings::RETRIEVAL_FILTER
        );
        let result = {
            let guard = store.client.read().await;
            let db = guard.as_ref().unwrap();
            db.query(sql).bind(json!({"model":s.embedder.manifest().version(),"kind":"passage","exclude":null,"cursor":null,"visible":true})).await.unwrap().check()
        };
        assert!(result.is_ok());
        store.close().await.unwrap();
        s.store.close().await.unwrap();
    }
}
