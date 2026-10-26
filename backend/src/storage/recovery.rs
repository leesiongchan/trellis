//! Restart recovery only closes interrupted intents. It never calls providers.
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySummary {
    pub interrupted_attempts: usize,
    pub interrupted_verifications: usize,
    pub resumed_derived_work: usize,
}
impl KnowledgeStore {
    pub async fn recover_interrupted(&self) -> Result<RecoverySummary, AppError> {
        let mut result=self.query("BEGIN TRANSACTION; LET $attempts=(SELECT VALUE key FROM research_attempt WHERE data.state='pending'); LET $verification=(SELECT VALUE key FROM verification_stage WHERE data.state='started'); LET $derived=array::len((SELECT VALUE key FROM embedding_cursor LIMIT 100)); UPDATE operation SET data.state='interrupted',data.error=$error,data.updatedAt=$now WHERE data.state='pending'; UPDATE research_attempt SET data.state='interrupted' WHERE data.state='pending'; UPDATE verification_stage SET data.state='interrupted',data.finishedAt=$now WHERE data.state='started'; DELETE active_attempt; DELETE research_follower; RETURN {interruptedAttempts:array::len($attempts),interruptedVerifications:array::len($verification),resumedDerivedWork:$derived}; COMMIT TRANSACTION;",json!({"error":crate::contracts::library::OperationError::from(AppError::new(ErrorCode::WorkInterrupted)),"now":records::now()?})).await?;
        tracing::debug!(
            event = "recovery_query_complete",
            statements = result.num_statements()
        );
        // The SDK includes BEGIN and COMMIT results; RETURN precedes COMMIT.
        let summary = result
            .num_statements()
            .checked_sub(2)
            .ok_or_else(storage_error)?;
        let value = result
            .take::<surrealdb::types::Value>(summary)
            .map_err(|_| storage_error())?
            .into_json_value();
        serde_json::from_value(value).map_err(|_| storage_error())
    }
}
