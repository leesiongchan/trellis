//! Durable idempotency. Database uniqueness chooses the winner, never an app mutex.
use super::*;
use crate::contracts::library::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationIntent {
    pub id: OperationId,
    pub library_epoch: u64,
    pub request_hash: String,
    pub target: String,
    pub created_at: String,
}
impl OperationIntent {
    pub fn new(
        id: OperationId,
        library_epoch: u64,
        payload: Value,
        target: &str,
    ) -> Result<Self, AppError> {
        Ok(Self {
            id,
            library_epoch,
            request_hash: records::canonical_hash(&json!({"target":target,"payload":payload})),
            target: target.into(),
            created_at: records::now()?,
        })
    }
}
#[derive(Debug)]
pub enum ClaimOutcome {
    Claimed(OperationView),
    Replay(OperationView),
}
impl KnowledgeStore {
    pub async fn library_info(&self) -> Result<LibraryInfo, AppError> {
        let mut result = self
            .query("SELECT * FROM ONLY library:main;", json!({}))
            .await?;
        let value: Value = result
            .take::<surrealdb::types::Value>(0)
            .map_err(|_| storage_error())?
            .into_json_value();
        Ok(LibraryInfo {
            protocol_version: 2,
            schema_version: 2,
            library_id: LibraryId::new(value["key"].as_str().ok_or_else(storage_error)?)?,
            library_epoch: value["epoch"].as_u64().ok_or_else(storage_error)?,
            discovery: match value["discoveryStatus"].as_str() {
                Some("ready") => DiscoveryStatus::Ready,
                Some("indexing") => DiscoveryStatus::Indexing,
                _ => DiscoveryStatus::Unavailable,
            },
        })
    }
    async fn operation_record(&self, id: &OperationId) -> Result<Option<Value>, AppError> {
        let mut result = self
            .query(
                "SELECT * FROM ONLY type::record('operation',$key);",
                json!({"key":id}),
            )
            .await?;
        let value: Value = result
            .take::<surrealdb::types::Value>(0)
            .map_err(|_| storage_error())?
            .into_json_value();
        Ok(if value.is_null() { None } else { Some(value) })
    }
    pub async fn operation(&self, id: &OperationId) -> Result<OperationView, AppError> {
        let record = self
            .operation_record(id)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound))?;
        serde_json::from_value(record["data"].clone()).map_err(|_| storage_error())
    }
    fn replay(intent: &OperationIntent, record: Value) -> Result<ClaimOutcome, AppError> {
        if record["libraryEpoch"].as_u64() != Some(intent.library_epoch) {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        if record["requestHash"] != intent.request_hash || record["target"] != intent.target {
            return Err(AppError::new(ErrorCode::IntentConflict));
        }
        Ok(ClaimOutcome::Replay(
            serde_json::from_value(record["data"].clone()).map_err(|_| storage_error())?,
        ))
    }
    pub async fn claim_operation(
        &self,
        intent: &OperationIntent,
    ) -> Result<ClaimOutcome, AppError> {
        let info = self.library_info().await?;
        if info.library_epoch != intent.library_epoch {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        if let Some(record) = self.operation_record(&intent.id).await? {
            return Self::replay(intent, record);
        }
        let view = OperationView {
            id: intent.id.clone(),
            state: OperationState::Pending,
            result: None,
            error: None,
            created_at: intent.created_at.clone(),
            updated_at: intent.created_at.clone(),
        };
        let result=self.query("BEGIN TRANSACTION; IF library:main.epoch != $epoch { THROW 'library_epoch_changed'; }; CREATE ONLY type::record('operation',$key) CONTENT $record; COMMIT TRANSACTION;",json!({"epoch":intent.library_epoch,"key":intent.id,"record":{"key":intent.id,"requestHash":intent.request_hash,"libraryEpoch":intent.library_epoch,"target":intent.target,"data":view}})).await;
        match result {
            Ok(_) => Ok(ClaimOutcome::Claimed(view)),
            Err(error) => {
                if self.library_info().await?.library_epoch != intent.library_epoch {
                    return Err(AppError::new(ErrorCode::RevisionConflict));
                }
                if let Some(record) = self.operation_record(&intent.id).await? {
                    return Self::replay(intent, record);
                }
                Err(error)
            }
        }
    }
}

impl KnowledgeStore {
    pub(crate) fn intent<T: Serialize + ValidatedInput + serde::de::DeserializeOwned>(
        command: &Command<T>,
        target: &str,
    ) -> Result<(Command<T>, OperationIntent), AppError> {
        let command = Command::from_json(
            serde_json::to_value(command).map_err(|_| AppError::invalid_question())?,
        )?;
        let intent = OperationIntent::new(
            command.operation_id.clone(),
            command.library_epoch,
            serde_json::to_value(&command).map_err(|_| storage_error())?,
            target,
        )?;
        Ok((command, intent))
    }
    pub(crate) async fn replay_existing(
        &self,
        intent: &OperationIntent,
        purge_receipt: bool,
    ) -> Result<Option<OperationView>, AppError> {
        let info = self.library_info().await?;
        let record = self.operation_record(&intent.id).await?;
        if info.library_epoch != intent.library_epoch
            && !(purge_receipt
                && record
                    .as_ref()
                    .is_some_and(|r| r["target"] == "purge" && r["data"]["state"] == "completed"))
        {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        match record {
            Some(record) => match Self::replay(intent, record)? {
                ClaimOutcome::Replay(view) => Ok(Some(view)),
                ClaimOutcome::Claimed(_) => unreachable!(),
            },
            None => Ok(None),
        }
    }
    pub(crate) async fn commit_mutation(
        &self,
        intent: &OperationIntent,
        body: &str,
        mut bindings: Value,
        result: OperationResult,
    ) -> Result<OperationView, AppError> {
        let view = OperationView {
            id: intent.id.clone(),
            state: OperationState::Completed,
            result: Some(result),
            error: None,
            created_at: intent.created_at.clone(),
            updated_at: intent.created_at.clone(),
        };
        bindings["epoch"] = json!(intent.library_epoch);
        bindings["operationKey"] = json!(intent.id);
        bindings["requestHash"] = json!(intent.request_hash);
        bindings["target"] = json!(intent.target);
        bindings["operationRecord"] = json!({"key":intent.id,"requestHash":intent.request_hash,"libraryEpoch":intent.library_epoch,"target":intent.target,"data":view});
        let sql = format!(
            "BEGIN TRANSACTION; IF library:main.epoch != $epoch {{ THROW 'library_epoch_changed'; }}; LET $existing=(SELECT * FROM ONLY type::record('operation',$operationKey)); IF $existing != NONE {{ IF $existing.requestHash != $requestHash OR $existing.target != $target {{ THROW 'intent_conflict'; }}; }} ELSE {{ {body} CREATE ONLY type::record('operation',$operationKey) CONTENT $operationRecord; }}; COMMIT TRANSACTION;"
        );
        match self.query(&sql, bindings).await {
            Ok(_) => self.operation(&intent.id).await,
            Err(error) => {
                if let Some(view) = self.replay_existing(intent, false).await? {
                    return Ok(view);
                }
                Err(error)
            }
        }
    }
}
