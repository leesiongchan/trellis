//! Bounded JSON staging followed by one atomic publication transaction.
use super::{library::appearance_record, *};
use crate::contracts::{Ancestor, legacy::*, library::*, truncate_utf16};
use operations::OperationIntent;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    view: ImportView,
    planned_exploration_id: ExplorationId,
    library_epoch: u64,
    identity: String,
    legacy_hash: String,
    input: BeginImport,
    received_bytes: usize,
    batches: usize,
    created_at: String,
}
fn invalid() -> AppError {
    AppError::new(ErrorCode::InvalidImport)
}
fn cutoff() -> Result<String, AppError> {
    (time::OffsetDateTime::now_utc() - time::Duration::hours(24))
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| storage_error())
}
impl KnowledgeStore {
    async fn commit_import(
        &self,
        intent: &OperationIntent,
        body: &str,
        mut bindings: Value,
    ) -> Result<ImportView, AppError> {
        bindings["epoch"] = json!(intent.library_epoch);
        bindings["operationKey"] = json!(intent.id);
        bindings["requestHash"] = json!(intent.request_hash);
        bindings["target"] = json!(intent.target);
        let view = OperationView {
            id: intent.id.clone(),
            state: OperationState::Completed,
            result: None,
            error: None,
            created_at: intent.created_at.clone(),
            updated_at: intent.created_at.clone(),
        };
        bindings["operationRecord"] = json!({"key":intent.id,"requestHash":intent.request_hash,"libraryEpoch":intent.library_epoch,"target":intent.target,"data":view});
        let sql = format!(
            "BEGIN TRANSACTION; IF library:main.epoch!=$epoch {{ THROW 'library_epoch_changed'; }}; LET $old=(SELECT * FROM ONLY type::record('operation',$operationKey)); IF $old!=NONE {{ IF $old.requestHash!=$requestHash OR $old.target!=$target {{ THROW 'intent_conflict'; }}; }} ELSE {{ {body} LET $result={{kind:'import',importId:$selected.key,explorationId:$selected.data.plannedExplorationId,revision:0}}; LET $record=object::extend($operationRecord,{{data:object::extend($operationRecord.data,{{result:$result}})}}); CREATE ONLY type::record('operation',$operationKey) CONTENT $record; }}; COMMIT TRANSACTION;"
        );
        self.query(&sql, bindings).await?;
        self.import_operation(&intent.id).await
    }
    async fn import_operation(&self, id: &OperationId) -> Result<ImportView, AppError> {
        let op = self.operation(id).await?;
        let Some(OperationResult::Import { import_id, .. }) = op.result else {
            return Err(storage_error());
        };
        self.import_status(&import_id).await
    }
    pub async fn import_status(&self, id: &ImportId) -> Result<ImportView, AppError> {
        let manifest: Manifest = self.record_data("migration_manifest", id.as_str()).await?;
        if manifest.library_epoch != self.library_info().await?.library_epoch {
            return Err(AppError::new(ErrorCode::NotFound));
        }
        let mut view = manifest.view;
        if view.state == ImportState::Completed {
            let receipt = self
                .query_value(
                    "SELECT VALUE data.deleted FROM ONLY type::record('import_receipt',$key);",
                    json!({"key":manifest.identity}),
                )
                .await?;
            if receipt == true {
                view.state = ImportState::Suppressed;
                view.mapping.clear();
                view.exploration_id = None;
                view.server_revision = None;
            }
        }
        Ok(view)
    }
    /// Expired, unpublished fragments only. Completed content is never swept here.
    pub async fn cleanup_imports(&self) -> Result<(), AppError> {
        self.query("BEGIN TRANSACTION; LET $expired=(SELECT VALUE key FROM migration_manifest WHERE data.view.state='staging' AND data.createdAt<$cutoff LIMIT 3); FOR $key IN $expired { DELETE migration_stage WHERE importKey=$key; UPDATE ONLY type::record('migration_manifest',$key) SET data.view.state='failed',data.view.error=$error; }; COMMIT TRANSACTION;",json!({"cutoff":cutoff()?,"error":OperationError::from(invalid())})).await?;
        Ok(())
    }
    pub async fn begin_import(
        &self,
        command: &Command<BeginImport>,
    ) -> Result<ImportView, AppError> {
        let (command, intent) = Self::intent(command, "begin-import")?;
        if self.replay_existing(&intent, false).await?.is_some() {
            return self.import_operation(&intent.id).await;
        }
        if command.expected_revision.is_some() {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        self.cleanup_imports().await?;
        let identity = records::canonical_hash(&json!([
            "legacy-content-v1",
            command.payload.legacy_exploration_id,
            command.payload.content_digest
        ]));
        let legacy_hash = records::canonical_hash(&json!([
            "legacy-id-v1",
            command.payload.legacy_exploration_id
        ]));
        let id = ImportId::generate();
        let planned = ExplorationId::generate();
        let view = ImportView {
            upload_operation_id: intent.id.clone(),
            id: id.clone(),
            exploration_id: None,
            state: ImportState::Staging,
            mapping: BTreeMap::new(),
            server_revision: None,
            error: None,
        };
        let manifest = Manifest {
            view,
            planned_exploration_id: planned,
            library_epoch: command.library_epoch,
            identity: identity.clone(),
            legacy_hash: legacy_hash.clone(),
            input: command.payload.clone(),
            received_bytes: 0,
            batches: 0,
            created_at: intent.created_at.clone(),
        };
        let body = "UPDATE library:main SET importSerial=(importSerial ?? 0)+1; LET $prior=(SELECT * FROM migration_manifest WHERE identityKey=$identity LIMIT 1)[0]; LET $receipt=(SELECT * FROM ONLY type::record('import_receipt',$identity)); LET $deleted=$receipt.data.deleted=true OR array::len((SELECT VALUE key FROM import_receipt WHERE data.legacyHash=$legacyHash AND data.deleted=true LIMIT 1))>0; LET $restore=$restoreRequested AND ($deleted OR $prior.data.view.state IN ['failed','suppressed']); IF $restore AND $prior!=NONE { UPDATE ONLY $prior.id SET identityKey='archived:'+$prior.key; }; IF $prior=NONE OR $restore { LET $suppressed=!$restoreRequested AND ($deleted OR library:main.epoch>1); IF !$suppressed AND array::len((SELECT VALUE key FROM migration_manifest WHERE data.view.state='staging' LIMIT 3))>=2 { THROW 'import_limit'; }; LET $data=object::extend($manifest,{view:object::extend($manifest.view,{state:IF $suppressed { 'suppressed' } ELSE { 'staging' }})}); CREATE ONLY type::record('migration_manifest',$newKey) CONTENT {key:$newKey,identityKey:$identity,data:$data}; }; LET $selected=(SELECT * FROM migration_manifest WHERE identityKey=$identity LIMIT 1)[0];";
        self.commit_import(&intent,body,json!({"identity":identity,"legacyHash":legacy_hash,"restoreRequested":command.payload.explicit_restore,"newKey":id,"manifest":manifest})).await
    }
    pub async fn stage_import(
        &self,
        command: &Command<ImportBatch>,
    ) -> Result<ImportView, AppError> {
        let (command, intent) = Self::intent(command, "stage-import")?;
        if self.replay_existing(&intent, false).await?.is_some() {
            return self.import_operation(&intent.id).await;
        }
        if command.expected_revision.is_some() {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        let mut fragment = String::new();
        for record in &command.payload.records {
            fragment.push_str(record.as_str().ok_or_else(invalid)?);
        }
        let key = records::canonical_hash(&json!([
            command.payload.import_id,
            command.payload.batch_index
        ]));
        let hash = records::canonical_hash(&json!(command.payload.records));
        let body = "LET $manifest=(SELECT * FROM ONLY type::record('migration_manifest',$importKey)); IF $manifest=NONE { THROW 'missing_reference'; }; IF $manifest.data.libraryEpoch!=$epoch { THROW 'library_epoch_changed'; }; IF $manifest.data.view.state!='staging' OR $manifest.data.createdAt<$cutoff { THROW 'invalid_import'; }; LET $old=(SELECT * FROM ONLY type::record('migration_stage',$batchKey)); IF $old!=NONE { IF $old.data.hash!=$hash { THROW 'intent_conflict'; }; } ELSE { IF $manifest.data.batches!=$index OR $manifest.data.receivedBytes+$bytes>$manifest.data.input.expectedBytes { THROW 'invalid_import'; }; CREATE ONLY type::record('migration_stage',$batchKey) CONTENT {key:$batchKey,importKey:$importKey,batchIndex:$index,data:{hash:$hash,fragment:$fragment}}; UPDATE ONLY type::record('migration_manifest',$importKey) SET data.receivedBytes=data.receivedBytes+$bytes,data.batches=data.batches+1; }; LET $selected=(SELECT * FROM ONLY type::record('migration_manifest',$importKey));";
        self.commit_import(&intent,body,json!({"importKey":command.payload.import_id,"batchKey":key,"hash":hash,"index":command.payload.batch_index,"bytes":fragment.len(),"fragment":fragment,"cutoff":cutoff()?})).await
    }
    pub async fn finalize_import(
        &self,
        command: &Command<FinalizeImport>,
    ) -> Result<ImportView, AppError> {
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.finalize_import_inner(command),
        )
        .await
        .map_err(|_| AppError::new(ErrorCode::StorageBusy))?
    }
    async fn finalize_import_inner(
        &self,
        command: &Command<FinalizeImport>,
    ) -> Result<ImportView, AppError> {
        let (command, intent) = Self::intent(command, "finalize-import")?;
        if self.replay_existing(&intent, false).await?.is_some() {
            return self.import_operation(&intent.id).await;
        }
        if command.expected_revision.is_some() {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        let id = &command.payload.import_id;
        let manifest: Manifest = self.record_data("migration_manifest", id.as_str()).await?;
        if manifest.library_epoch != command.library_epoch {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        if manifest.view.state != ImportState::Staging {
            return self.commit_import(&intent,"LET $selected=(SELECT * FROM ONLY type::record('migration_manifest',$importKey));",json!({"importKey":id})).await;
        }
        let fragments=self.query_value("SELECT VALUE data.fragment FROM migration_stage WHERE importKey=$key ORDER BY batchIndex ASC LIMIT 2001;",json!({"key":id})).await?;
        let fragments: Vec<String> =
            serde_json::from_value(fragments).map_err(|_| storage_error())?;
        let copy = manifest.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            if copy.created_at < cutoff()?
                || copy.received_bytes != copy.input.expected_bytes as usize
                || fragments.len() != copy.batches
                || copy.received_bytes > MAX_IMPORT_BYTES
            {
                return Err(invalid());
            }
            let raw = fragments.concat();
            if raw.len() != copy.received_bytes {
                return Err(invalid());
            }
            let legacy =
                LegacyExploration::validate(serde_json::from_str(&raw).map_err(|_| invalid())?)?;
            if legacy.id != copy.input.legacy_exploration_id
                || legacy.nodes.len() != copy.input.expected_nodes as usize
                || legacy.digest() != copy.input.content_digest
            {
                return Err(invalid());
            }
            prepare_publication(legacy, &copy)
        })
        .await
        .map_err(|_| storage_error())?;
        let mut bindings = match prepared {
            Ok(bindings) => bindings,
            Err(error) => {
                return self.commit_import(&intent,"IF (SELECT VALUE data.view.state FROM ONLY type::record('migration_manifest',$importKey))='staging' { UPDATE ONLY type::record('migration_manifest',$importKey) SET data.view.state='failed',data.view.error=$error; DELETE migration_stage WHERE importKey=$importKey; }; LET $selected=(SELECT * FROM ONLY type::record('migration_manifest',$importKey));",json!({"importKey":id,"error":OperationError::from(error)})).await;
            }
        };
        bindings["importKey"] = json!(id);
        bindings["batches"] = json!(manifest.batches);
        bindings["bytes"] = json!(manifest.received_bytes);
        bindings["cutoff"] = json!(cutoff()?);
        let body = "LET $manifest=(SELECT * FROM ONLY type::record('migration_manifest',$importKey)); IF $manifest.data.view.state='staging' { IF $manifest.data.batches!=$batches OR $manifest.data.receivedBytes!=$bytes OR $manifest.data.createdAt<$cutoff { THROW 'invalid_import'; }; FOR $entry IN $entries { LET $topicKey=$entry.topicKey; IF !record::exists(type::record('topic',$topicKey)) { CREATE ONLY type::record('topic',$topicKey) CONTENT $entry.topicRecord; CREATE ONLY type::record('question_variant',$topicKey) CONTENT $entry.variantRecord; }; IF $entry.answer!=NULL { LET $previous=(SELECT VALUE revision FROM answer_version WHERE topicKey=$topicKey ORDER BY revision DESC LIMIT 1)[0]; LET $revision=($previous ?? -1)+1; CREATE ONLY type::record('answer_version',$entry.answer.key) CONTENT object::extend($entry.answer,{revision:$revision,data:object::extend($entry.answer.data,{revision:$revision})}); UPDATE ONLY type::record('topic',$topicKey) SET data.currentAnswerVersionId=(data.currentAnswerVersionId ?? $entry.answer.key); }; CREATE ONLY type::record('appearance',$entry.appearance.key) CONTENT $entry.appearance; }; FOR $edge IN $edges { IF $edge.from!=$edge.to { LET $from=type::record('topic',$edge.from); LET $to=type::record('topic',$edge.to); LET $link=type::record('connection',$edge.record.key); IF !record::exists($link) { RELATE $from->$link->$to CONTENT $edge.record; }; CREATE ONLY type::record('edge_support',$edge.support.key) CONTENT $edge.support; }; }; CREATE ONLY type::record('exploration',$exploration.key) CONTENT $exploration; LET $receiptKey=$manifest.data.identity; IF (SELECT VALUE data.deleted FROM ONLY type::record('import_receipt',$receiptKey))=true { LET $archive='retired-'+$importKey; CREATE ONLY type::record('import_receipt',$archive) CONTENT {key:$archive,identityKey:$archive,data:(SELECT VALUE data FROM ONLY type::record('import_receipt',$receiptKey))}; }; UPSERT type::record('import_receipt',$receiptKey) CONTENT $receipt; UPDATE ONLY type::record('migration_manifest',$importKey) SET data.view=$completed; DELETE migration_stage WHERE importKey=$importKey; }; LET $selected=(SELECT * FROM ONLY type::record('migration_manifest',$importKey));";
        self.commit_import(&intent, body, bindings).await
    }
}
fn prepare_publication(legacy: LegacyExploration, manifest: &Manifest) -> Result<Value, AppError> {
    use super::library::add_branch_bindings;
    let exploration = &manifest.planned_exploration_id;
    let mapping = legacy
        .nodes
        .keys()
        .map(|id| {
            (
                id.clone(),
                records::canonical_hash(&json!([manifest.view.id, "appearance", id])),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut contexts = BTreeMap::<String, Vec<Ancestor>>::new();
    contexts.insert(legacy.root_id.clone(), vec![]);
    let mut stack = vec![legacy.root_id.clone()];
    let mut entries = vec![];
    let mut appearances = BTreeMap::new();
    while let Some(id) = stack.pop() {
        let node = &legacy.nodes[&id];
        let context = contexts.remove(&id).ok_or_else(invalid)?;
        let resolved = records::ResolvedQuestion::new(
            &node.question,
            "en",
            None,
            context.clone(),
            "legacy-v1-clarification-unknown",
        )?;
        let (topic, b) =
            KnowledgeStore::topic_bindings(&resolved, &node.question, &legacy.created_at)?;
        let version = node
            .content
            .as_ref()
            .map(|_| {
                AnswerVersionId::new(records::canonical_hash(&json!([
                    manifest.view.id,
                    "answer",
                    id
                ])))
            })
            .transpose()?;
        let appearance = AppearanceView {
            id: AppearanceId::new(mapping[&id].clone())?,
            exploration_id: exploration.clone(),
            parent_id: node
                .parent_id
                .as_ref()
                .map(|id| AppearanceId::new(mapping[id].clone()))
                .transpose()?,
            topic_id: topic.clone(),
            question: node.question.clone(),
            locale: "en".into(),
            clarification: None,
            ancestors: context.clone(),
            interpretation: resolved.interpretation,
            relationship: node.relationship.clone(),
            teaser: node.teaser.clone(),
            reason: None,
            reason_sources: vec![],
            reason_answer_version_id: None,
            answer_version_id: version.clone(),
            child_ids: node
                .child_ids
                .iter()
                .map(|id| AppearanceId::new(mapping[id].clone()))
                .collect::<Result<_, _>>()?,
        };
        let answer=node.content.as_ref().map(|content|json!({"key":version,"topicKey":topic,"revision":0,"data":{"id":version,"topicId":topic,"origin":"legacy","revision":0,"content":content,"createdAt":content.researched_at}}));
        let mut next_context = context;
        next_context.push(Ancestor {
            title: truncate_utf16(
                node.content
                    .as_ref()
                    .map(|c| c.title.as_str())
                    .unwrap_or(&node.question),
                160,
            ),
            summary: truncate_utf16(
                &node
                    .content
                    .as_ref()
                    .map(|c| {
                        c.summary
                            .iter()
                            .map(|p| p.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_else(|| node.teaser.clone()),
                400,
            ),
        });
        if next_context.len() > 15 {
            let mut bounded = vec![next_context[0].clone()];
            bounded.extend_from_slice(&next_context[next_context.len() - 14..]);
            next_context = bounded;
        }
        for child in node.child_ids.iter().rev() {
            contexts.insert(child.clone(), next_context.clone());
            stack.push(child.clone());
        }
        entries.push(json!({"topicKey":topic,"topicRecord":b["topicRecord"],"variantRecord":b["variantRecord"],"appearance":appearance_record(&appearance),"answer":answer}));
        appearances.insert(id, appearance);
    }
    let mut edges = vec![];
    for node in appearances.values() {
        if let Some(parent) = &node.parent_id {
            let from = appearances
                .values()
                .find(|p| p.id == *parent)
                .ok_or_else(invalid)?;
            let mut b = json!({});
            add_branch_bindings(&mut b, &from.topic_id, &node.topic_id, &node.id)?;
            edges.push(json!({"from":from.topic_id,"to":node.topic_id,"record":b["connectionRecord"],"support":b["supportRecord"]}));
        }
    }
    let summary = ExplorationSummary {
        id: exploration.clone(),
        original_prompt: legacy.original_prompt,
        root_id: AppearanceId::new(mapping[&legacy.root_id].clone())?,
        revision: 0,
        created_at: legacy.created_at,
        updated_at: legacy.updated_at,
    };
    let completed = ImportView {
        upload_operation_id: manifest.view.upload_operation_id.clone(),
        id: manifest.view.id.clone(),
        exploration_id: Some(exploration.clone()),
        state: ImportState::Completed,
        mapping,
        server_revision: Some(0),
        error: None,
    };
    Ok(
        json!({"entries":entries,"edges":edges,"exploration":{"key":exploration,"revision":0,"updatedAt":summary.updated_at,"data":summary},"completed":completed,"receipt":{"key":manifest.identity,"identityKey":manifest.identity,"data":{"legacyHash":manifest.legacy_hash,"deleted":false,"explorationId":exploration,"libraryEpoch":manifest.library_epoch,"view":completed}}}),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    async fn setup() -> (tempfile::TempDir, KnowledgeStore) {
        let root = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::open(&StorageConfig {
            data_dir: root.path().join("data"),
            backup_dir: root.path().join("backups"),
            model_dir: root.path().join("models"),
            asset_dir: root.path().join("public"),
        })
        .await
        .unwrap();
        store.install_schema().await.unwrap();
        (root, store)
    }
    fn command<T>(payload: T) -> Command<T> {
        Command {
            operation_id: OperationId::generate(),
            library_epoch: 1,
            expected_revision: None,
            payload,
        }
    }
    async fn start(store: &KnowledgeStore, id: &str) -> (ImportView, String) {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/library/migration.json"
        ))
        .unwrap();
        value["id"] = json!(id);
        let encoded = value.to_string();
        let legacy = LegacyExploration::validate(value).unwrap();
        let view = store
            .begin_import(&command(BeginImport {
                legacy_exploration_id: id.into(),
                content_digest: legacy.digest(),
                expected_bytes: encoded.len() as u64,
                expected_nodes: 3,
                explicit_restore: false,
            }))
            .await
            .unwrap();
        store
            .stage_import(&command(ImportBatch {
                import_id: view.id.clone(),
                batch_index: 0,
                records: vec![json!(encoded)],
            }))
            .await
            .unwrap();
        (view, encoded)
    }
    #[tokio::test]
    async fn expired_staging_cleanup_cannot_delete_published_content() {
        let (_root, store) = setup().await;
        let (a, _) = start(&store, "complete").await;
        let a = store
            .finalize_import(&command(FinalizeImport { import_id: a.id }))
            .await
            .unwrap();
        let (b, _) = start(&store, "expired").await;
        store
            .query(
                "UPDATE migration_manifest SET data.createdAt='2000-01-01T00:00:00Z';",
                json!({}),
            )
            .await
            .unwrap();
        store.cleanup_imports().await.unwrap();
        assert_eq!(
            store.import_status(&a.id).await.unwrap().state,
            ImportState::Completed
        );
        assert_eq!(
            store.import_status(&b.id).await.unwrap().state,
            ImportState::Failed
        );
        assert_eq!(
            store
                .exploration(a.exploration_id.as_ref().unwrap())
                .await
                .unwrap()
                .answers
                .len(),
            3
        );
        assert!(
            store
                .query_value("SELECT * FROM migration_stage LIMIT 1;", json!({}))
                .await
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn legacy_citations_cannot_be_promoted_to_grounded_factual_links() {
        use crate::{
            embeddings::ParagraphLocation,
            knowledge::{
                matching::{MatchCandidate, MatchChannel},
                verification::{
                    ConnectionDecision, ConnectionVerdict, ParagraphReference, VerdictKind,
                },
            },
        };
        let (_root, store) = setup().await;
        let (view, _) = start(&store, "legacy").await;
        let done = store
            .finalize_import(&command(FinalizeImport { import_id: view.id }))
            .await
            .unwrap();
        let snapshot = store
            .exploration(done.exploration_id.as_ref().unwrap())
            .await
            .unwrap();
        let left = &snapshot.answers[0];
        let right = snapshot
            .answers
            .iter()
            .find(|a| a.topic_id != left.topic_id)
            .unwrap();
        let a = store
            .embedding_source(&left.topic_id, Some(&left.id), 1)
            .await
            .unwrap();
        let b = store
            .embedding_source(&right.topic_id, Some(&right.id), 1)
            .await
            .unwrap();
        let location = |answer: &AnswerVersionView| ParagraphLocation {
            paragraph_id: answer.content.summary[0].id.clone(),
            section_index: None,
            part_index: 0,
            token_start: 0,
            token_end: 1,
        };
        let candidate = MatchCandidate::from_sources(
            &a,
            Some(&location(left)),
            &b,
            Some(&location(right)),
            MatchChannel::PassagePassage,
            "fixture",
            0.9,
        )
        .unwrap();
        store
            .save_candidates(std::slice::from_ref(&candidate))
            .await
            .unwrap();
        let attempt = answers::ResearchAttempt {
            id: AttemptId::generate(),
            topic_id: left.topic_id.clone(),
            library_epoch: 1,
            request: crate::contracts::ResearchRequest {
                exploration_id: "fixture".into(),
                node_id: "fixture".into(),
                question: "Question".into(),
                clarification: None,
                ancestors: vec![],
            },
            resolved: records::ResolvedQuestion::new("Question", "en", None, vec![], "literal")
                .unwrap(),
            reuse_version: None,
            state: OperationState::Completed,
        };
        let decision = ConnectionDecision {
            candidate_id: candidate.id.clone(),
            verdict: VerdictKind::Related,
            reason: "Shared cited explanation".into(),
            paragraph_refs: [&candidate.from, &candidate.to]
                .iter()
                .map(|e| {
                    let p = e.passage.as_ref().unwrap();
                    ParagraphReference {
                        answer_version_id: p.location.answer_version_id.clone(),
                        paragraph_id: p.location.paragraph_id.clone(),
                        section_index: p.location.section_index,
                    }
                })
                .collect(),
        };
        for grounded in [false, true] {
            if grounded {
                store
                    .query(
                        "UPDATE answer_version SET data.origin='research';",
                        json!({}),
                    )
                    .await
                    .unwrap();
            }
            store.query("UPSERT type::record('verification_stage',$key) CONTENT {key:$key,attemptKey:$key,data:{state:'started'}};",json!({"key":attempt.id})).await.unwrap();
            store
                .finish_verification(
                    &attempt,
                    std::slice::from_ref(&candidate),
                    &Ok(ConnectionVerdict {
                        decisions: vec![decision.clone()],
                    }),
                )
                .await
                .unwrap();
            let edges = store
                .connections(&left.topic_id, None, 100)
                .await
                .unwrap()
                .items;
            let edge = edges.iter().find(|e| e.id == candidate.id).unwrap();
            assert_eq!(
                edge.status,
                if grounded {
                    ConnectionStatus::Accepted
                } else {
                    ConnectionStatus::Candidate
                }
            );
        }
        store.close().await.unwrap();
    }
}
