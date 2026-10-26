//! Atomic content commands and bounded read projections.
use super::{
    records::{ResolvedQuestion, TopicRecord, question_key},
    *,
};
use crate::contracts::{Ancestor, library::*, truncate_utf16};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
const ENSURE_TOPIC: &str = "IF !record::exists(type::record('topic',$topicKey)) { CREATE ONLY type::record('topic',$topicKey) CONTENT $topicRecord; }; IF !record::exists(type::record('question_variant',$topicKey)) { CREATE ONLY type::record('question_variant',$topicKey) CONTENT $variantRecord; };";
impl KnowledgeStore {
    pub(crate) async fn query_value(&self, sql: &str, bindings: Value) -> Result<Value, AppError> {
        let mut response = self.query(sql, bindings).await?;
        Ok(response
            .take::<surrealdb::types::Value>(0)
            .map_err(|_| storage_error())?
            .into_json_value())
    }
    pub(crate) async fn record_data<T: DeserializeOwned>(
        &self,
        table: &str,
        key: &str,
    ) -> Result<T, AppError> {
        let value = self
            .query_value(
                "SELECT VALUE data FROM ONLY type::record($table,$key);",
                json!({"table":table,"key":key}),
            )
            .await?;
        if value.is_null() {
            return Err(AppError::new(ErrorCode::NotFound));
        }
        serde_json::from_value(value).map_err(|_| storage_error())
    }
    fn topic_bindings(
        input: &ResolvedQuestion,
        question: &str,
        now: &str,
    ) -> Result<(TopicId, Value), AppError> {
        let key = question_key(input);
        let id = TopicId::new(key.0.clone())?;
        let topic = TopicRecord {
            id: id.clone(),
            question: question.into(),
            current_answer_version_id: None,
            created_at: now.into(),
            updated_at: now.into(),
        };
        Ok((
            id.clone(),
            json!({"topicKey":id,"topicRecord":{"key":id,"data":topic},"variantRecord":{"key":id,"exactKey":key.0,"topicKey":id,"data":{"id":id,"resolved":input,"originalQuestion":question}}}),
        ))
    }
    pub async fn create_exploration(
        &self,
        command: &Command<CreateExploration>,
    ) -> Result<OperationView, AppError> {
        let (command, intent) = Self::intent(command, "create-exploration")?;
        if let Some(view) = self.replay_existing(&intent, false).await? {
            return Ok(view);
        }
        if command.expected_revision.is_some() {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        let resolved = ResolvedQuestion::new(
            &command.payload.question,
            &command.payload.locale,
            None,
            vec![],
            "literal",
        )?;
        let (topic_id, mut bindings) =
            Self::topic_bindings(&resolved, &command.payload.question, &intent.created_at)?;
        let exploration_id = ExplorationId::generate();
        let appearance_id = AppearanceId::generate();
        let summary = ExplorationSummary {
            id: exploration_id.clone(),
            original_prompt: command.payload.question.clone(),
            root_id: appearance_id.clone(),
            revision: 0,
            created_at: intent.created_at.clone(),
            updated_at: intent.created_at.clone(),
        };
        let appearance = AppearanceView {
            id: appearance_id.clone(),
            exploration_id: exploration_id.clone(),
            parent_id: None,
            topic_id: topic_id.clone(),
            question: command.payload.question,
            locale: command.payload.locale,
            clarification: None,
            ancestors: vec![],
            interpretation: "literal".into(),
            relationship: String::new(),
            reason: None,
            answer_version_id: None,
            child_ids: vec![],
        };
        bindings["explorationKey"] = json!(exploration_id);
        bindings["appearanceKey"] = json!(appearance_id);
        bindings["explorationRecord"] =
            json!({"key":exploration_id,"revision":0,"updatedAt":intent.created_at,"data":summary});
        bindings["appearanceRecord"] = appearance_record(&appearance);
        let body = format!(
            "{ENSURE_TOPIC} LET $pin=(SELECT VALUE data.currentAnswerVersionId FROM ONLY type::record('topic',$topicKey)); CREATE ONLY type::record('exploration',$explorationKey) CONTENT $explorationRecord; CREATE ONLY type::record('appearance',$appearanceKey) CONTENT object::extend($appearanceRecord,{{answerVersionKey:$pin,data:object::extend($appearanceRecord.data,{{answerVersionId:$pin}})}});"
        );
        self.commit_mutation(
            &intent,
            &body,
            bindings,
            OperationResult::Exploration {
                exploration_id,
                appearance_id,
                revision: 0,
            },
        )
        .await
    }
    pub async fn add_branch(
        &self,
        command: &Command<AddBranch>,
    ) -> Result<OperationView, AppError> {
        let (command, intent) = Self::intent(command, "add-branch")?;
        if let Some(view) = self.replay_existing(&intent, false).await? {
            return Ok(view);
        }
        let mut parent: AppearanceView = self
            .record_data("appearance", command.payload.parent_appearance_id.as_str())
            .await?;
        let revision = command
            .expected_revision
            .ok_or_else(|| AppError::new(ErrorCode::RevisionConflict))?;
        let mut context = parent.ancestors.clone();
        let (title, text) = if let Some(pin) = &parent.answer_version_id {
            let answer: AnswerVersionView =
                self.record_data("answer_version", pin.as_str()).await?;
            (
                answer.content.title,
                answer
                    .content
                    .summary
                    .iter()
                    .map(|p| p.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        } else {
            (
                parent.question.clone(),
                parent
                    .reason
                    .as_ref()
                    .map(|p| p.text.clone())
                    .unwrap_or_default(),
            )
        };
        context.push(Ancestor {
            title: truncate_utf16(&title, 160),
            summary: truncate_utf16(&text, 400),
        });
        if context.len() > 15 {
            let mut bounded = vec![context[0].clone()];
            bounded.extend_from_slice(&context[context.len() - 14..]);
            context = bounded;
        }
        let resolved = ResolvedQuestion::new(
            &command.payload.question,
            &parent.locale,
            None,
            context,
            "literal",
        )?;
        let (topic_id, mut bindings) =
            Self::topic_bindings(&resolved, &command.payload.question, &intent.created_at)?;
        let id = AppearanceId::generate();
        let appearance = AppearanceView {
            id: id.clone(),
            exploration_id: parent.exploration_id.clone(),
            parent_id: Some(parent.id.clone()),
            topic_id: topic_id.clone(),
            question: command.payload.question,
            locale: parent.locale.clone(),
            clarification: None,
            ancestors: resolved.ancestors,
            interpretation: resolved.interpretation,
            relationship: "branch".into(),
            reason: None,
            answer_version_id: None,
            child_ids: vec![],
        };
        parent.child_ids.push(id.clone());
        bindings["appearanceKey"] = json!(id);
        bindings["appearanceRecord"] = appearance_record(&appearance);
        bindings["parentKey"] = json!(parent.id);
        bindings["parentData"] = json!(parent);
        bindings["explorationKey"] = json!(parent.exploration_id);
        bindings["expectedRevision"] = json!(revision);
        bindings["updatedAt"] = json!(intent.created_at);
        add_branch_bindings(&mut bindings, &parent.topic_id, &topic_id, &id)?;
        let body = format!(
            "IF !record::exists(type::record('appearance',$parentKey)) OR (SELECT VALUE explorationKey FROM ONLY type::record('appearance',$parentKey)) != $explorationKey {{ THROW 'missing_reference'; }}; IF (SELECT VALUE revision FROM ONLY type::record('exploration',$explorationKey)) != $expectedRevision {{ THROW 'revision_conflict'; }}; IF array::len((SELECT VALUE key FROM appearance WHERE explorationKey=$explorationKey LIMIT 2001)) >= 2000 {{ THROW 'content_limit'; }}; {ENSURE_TOPIC} LET $pin=(SELECT VALUE data.currentAnswerVersionId FROM ONLY type::record('topic',$topicKey)); CREATE ONLY type::record('appearance',$appearanceKey) CONTENT object::extend($appearanceRecord,{{answerVersionKey:$pin,data:object::extend($appearanceRecord.data,{{answerVersionId:$pin}})}}); UPDATE ONLY type::record('appearance',$parentKey) SET data=$parentData; UPDATE ONLY type::record('exploration',$explorationKey) SET revision=$expectedRevision+1,updatedAt=$updatedAt,data.revision=$expectedRevision+1,data.updatedAt=$updatedAt; {}",
            branch_support_sql()
        );
        self.commit_mutation(
            &intent,
            &body,
            bindings,
            OperationResult::Exploration {
                exploration_id: parent.exploration_id,
                appearance_id: id,
                revision: revision + 1,
            },
        )
        .await
    }
    pub async fn exploration(&self, id: &ExplorationId) -> Result<ExplorationSnapshot, AppError> {
        let value=self.query_value("RETURN {epoch:library:main.epoch,exploration:(SELECT VALUE data FROM ONLY type::record('exploration',$key)),appearances:(SELECT VALUE data FROM appearance WHERE explorationKey=$key LIMIT 2001),answers:(SELECT VALUE data FROM answer_version WHERE key IN (SELECT VALUE answerVersionKey FROM appearance WHERE explorationKey=$key LIMIT 2001) LIMIT 2001)};",json!({"key":id})).await?;
        if value["exploration"].is_null() {
            return Err(AppError::new(ErrorCode::NotFound));
        }
        let appearances: Vec<AppearanceView> = decode(value["appearances"].clone())?;
        if appearances.len() > 2000 {
            return Err(storage_error());
        }
        Ok(ExplorationSnapshot {
            schema_version: 2,
            library_epoch: value["epoch"].as_u64().ok_or_else(storage_error)?,
            exploration: decode(value["exploration"].clone())?,
            appearances,
            answers: decode(value["answers"].clone())?,
        })
    }
    pub async fn explorations(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Page<ExplorationSummary>, AppError> {
        if !(1..=100).contains(&limit) {
            return Err(AppError::invalid_question());
        }
        let parsed = cursor.map(decode_cursor).transpose()?;
        let value=self.query_value("SELECT VALUE data FROM exploration WHERE $cursorKey=NULL OR updatedAt<$updatedAt OR (updatedAt=$updatedAt AND key<$cursorKey) ORDER BY updatedAt DESC,key DESC LIMIT $limit;",json!({"cursorKey":parsed.as_ref().map(|v|v.1.as_str()),"updatedAt":parsed.as_ref().map(|v|&v.0),"limit":limit+1})).await?;
        let mut items: Vec<ExplorationSummary> = decode(value)?;
        let more = items.len() > limit;
        items.truncate(limit);
        let next_cursor = if more {
            items
                .last()
                .map(|v| encode_cursor(&v.updated_at, v.id.as_str()))
                .transpose()?
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    pub async fn topic(
        &self,
        id: &TopicId,
        version: Option<&AnswerVersionId>,
    ) -> Result<TopicView, AppError> {
        let topic: TopicRecord = self.record_data("topic", id.as_str()).await?;
        let selected = version.or(topic.current_answer_version_id.as_ref());
        let answer = if let Some(selected) = selected {
            let answer: AnswerVersionView = self
                .record_data("answer_version", selected.as_str())
                .await?;
            if answer.topic_id != *id {
                return Err(AppError::new(ErrorCode::NotFound));
            }
            Some(answer)
        } else {
            None
        };
        let versions:Vec<AnswerVersionSummary>=decode(self.query_value("SELECT data.id AS id,data.revision AS revision,data.createdAt AS createdAt FROM answer_version WHERE topicKey=$key ORDER BY revision DESC LIMIT 100;",json!({"key":id})).await?)?;
        Ok(TopicView {
            id: id.clone(),
            question: topic.question,
            current_answer_version_id: topic.current_answer_version_id,
            selected_answer: answer,
            versions,
        })
    }
    pub async fn delete_exploration(
        &self,
        command: &Command<DeleteExploration>,
    ) -> Result<OperationView, AppError> {
        let (command, intent) = Self::intent(command, "delete-exploration")?;
        if let Some(view) = self.replay_existing(&intent, false).await? {
            return Ok(view);
        }
        let revision = command
            .expected_revision
            .ok_or_else(|| AppError::new(ErrorCode::RevisionConflict))?;
        let key = records::canonical_hash(&json!(["deleted", command.payload.exploration_id]));
        let body = "IF !record::exists(type::record('exploration',$explorationKey)) { THROW 'missing_reference'; }; IF (SELECT VALUE revision FROM ONLY type::record('exploration',$explorationKey)) != $expectedRevision { THROW 'revision_conflict'; }; LET $appearances=(SELECT VALUE key FROM appearance WHERE explorationKey=$explorationKey LIMIT 2001); LET $edges=(SELECT VALUE connectionKey FROM edge_support WHERE appearanceKey IN $appearances LIMIT 2001); DELETE edge_support WHERE appearanceKey IN $appearances; FOR $edge IN $edges { IF array::len((SELECT VALUE key FROM edge_support WHERE connectionKey=$edge LIMIT 1))=0 { DELETE connection WHERE key=$edge AND data.kind='branches_to'; }; }; DELETE appearance WHERE explorationKey=$explorationKey; DELETE ONLY type::record('exploration',$explorationKey); UPDATE import_receipt SET data.deleted=true WHERE data.explorationId=$explorationKey; UPSERT type::record('import_receipt',$receiptKey) CONTENT $receipt;";
        self.commit_mutation(&intent,body,json!({"explorationKey":command.payload.exploration_id,"expectedRevision":revision,"receiptKey":key,"receipt":{"key":key,"identityKey":format!("deleted:{}",command.payload.exploration_id),"data":{"explorationId":command.payload.exploration_id,"deleted":true}}}),OperationResult::Deletion{exploration_id:command.payload.exploration_id}).await
    }
    pub async fn purge_library(
        &self,
        command: &Command<PurgeLibrary>,
    ) -> Result<OperationView, AppError> {
        let (command, intent) = Self::intent(command, "purge")?;
        if let Some(view) = self.replay_existing(&intent, true).await? {
            return Ok(view);
        }
        if command.expected_revision.is_some() {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        let epoch = command
            .library_epoch
            .checked_add(1)
            .filter(|v| *v <= MAX_SAFE_INTEGER)
            .ok_or_else(storage_error)?;
        let body = "DELETE embedding; DELETE edge_support; DELETE connection; DELETE active_attempt; DELETE research_attempt; DELETE appearance; DELETE exploration; DELETE question_variant; DELETE answer_version; DELETE topic; DELETE migration_stage; DELETE migration_manifest; DELETE operation WHERE target!='purge'; UPDATE import_receipt SET data.deleted=true; UPDATE library:main SET epoch=$nextEpoch;";
        self.commit_mutation(
            &intent,
            body,
            json!({"nextEpoch":epoch}),
            OperationResult::Purge {
                library_epoch: epoch,
            },
        )
        .await
    }
}
fn decode<T: DeserializeOwned>(value: Value) -> Result<T, AppError> {
    serde_json::from_value(value).map_err(|_| storage_error())
}
pub(crate) fn appearance_record(a: &AppearanceView) -> Value {
    json!({"key":a.id,"explorationKey":a.exploration_id,"topicKey":a.topic_id,"parentKey":a.parent_id,"answerVersionKey":a.answer_version_id,"data":a})
}
pub(crate) fn add_branch_bindings(
    bindings: &mut Value,
    from: &TopicId,
    to: &TopicId,
    appearance: &AppearanceId,
) -> Result<(), AppError> {
    let pair = records::canonical_hash(&json!(["branches_to", from, to]));
    let id = ConnectionId::new(pair.clone())?;
    let connection = ConnectionView {
        id: id.clone(),
        from_topic_id: from.clone(),
        to_topic_id: to.clone(),
        kind: ConnectionKind::BranchesTo,
        status: ConnectionStatus::Accepted,
        reason: "Appears as a branch in an exploration".into(),
        method: "occurrence".into(),
        model_version: None,
        score: None,
        passages: vec![],
        evidence: vec![],
    };
    bindings["fromTopicKey"] = json!(from);
    bindings["connectionKey"] = json!(id);
    bindings["connectionRecord"] = json!({"key":id,"identityKey":pair,"data":connection});
    bindings["supportRecord"] =
        json!({"key":appearance,"connectionKey":id,"appearanceKey":appearance,"data":{}});
    Ok(())
}
pub(crate) fn branch_support_sql() -> &'static str {
    "IF $fromTopicKey != $topicKey { LET $from=type::record('topic',$fromTopicKey); LET $to=type::record('topic',$topicKey); LET $edge=type::record('connection',$connectionKey); IF !record::exists($edge) { RELATE $from->$edge->$to CONTENT $connectionRecord; }; CREATE ONLY type::record('edge_support',$appearanceKey) CONTENT $supportRecord; };"
}
fn encode_cursor(time: &str, id: &str) -> Result<String, AppError> {
    Ok(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&(time, id)).map_err(|_| storage_error())?))
}
fn decode_cursor(value: &str) -> Result<(String, ExplorationId), AppError> {
    if value.len() > 1000 {
        return Err(AppError::invalid_question());
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| AppError::invalid_question())?;
    let value: (String, ExplorationId) =
        serde_json::from_slice(&bytes).map_err(|_| AppError::invalid_question())?;
    if time::OffsetDateTime::parse(&value.0, &time::format_description::well_known::Rfc3339)
        .is_err()
    {
        return Err(AppError::invalid_question());
    }
    Ok(value)
}
