//! Bounded library locations and explicit roots from existing resolved topics.
use super::{library::appearance_record, records::ResolvedQuestion, *};
use crate::contracts::{library::*, utf16_len};
use serde_json::{Value, json};
impl KnowledgeStore {
    pub(crate) async fn appearance(&self, id: &AppearanceId) -> Result<AppearanceView, AppError> {
        self.record_data("appearance", id.as_str()).await
    }
    pub async fn topics(
        &self,
        cursor: Option<&str>,
        limit: usize,
        search: Option<&str>,
    ) -> Result<Page<TopicSummary>, AppError> {
        if !(1..=100).contains(&limit) || search.is_some_and(|s| utf16_len(s) > 500) {
            return Err(AppError::invalid_question());
        }
        if let Some(cursor) = cursor {
            TopicId::new(cursor)?;
        }
        let value=self.query_value("SELECT key,data,((SELECT count() AS total FROM answer_version WHERE topicKey=$parent.key GROUP ALL)[0].total ?? 0) AS answerVersionCount FROM topic WHERE ($cursor=NULL OR key>$cursor) AND ($search=NULL OR string::contains(string::lowercase(data.question),string::lowercase($search))) ORDER BY key ASC LIMIT $limit;",json!({"cursor":cursor,"search":search,"limit":limit+1})).await?;
        let rows = value.as_array().ok_or_else(storage_error)?;
        let mut items = vec![];
        for row in rows.iter().take(limit) {
            let topic: records::TopicRecord =
                serde_json::from_value(row["data"].clone()).map_err(|_| storage_error())?;
            items.push(TopicSummary {
                id: topic.id,
                question: topic.question,
                current_answer_version_id: topic.current_answer_version_id,
                updated_at: topic.updated_at,
                answer_version_count: row["answerVersionCount"]
                    .as_u64()
                    .ok_or_else(storage_error)?,
            });
        }
        let next_cursor = if rows.len() > limit {
            items.last().map(|v| v.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    pub async fn topic_appearances(
        &self,
        topic: &TopicId,
        cursor: Option<&str>,
        limit: usize,
        exploration: Option<&ExplorationId>,
        version: Option<&AnswerVersionId>,
    ) -> Result<Page<AppearanceLocation>, AppError> {
        if !(1..=100).contains(&limit) {
            return Err(AppError::invalid_question());
        }
        if let Some(cursor) = cursor {
            AppearanceId::new(cursor)?;
        }
        let _: records::TopicRecord = self.record_data("topic", topic.as_str()).await?;
        let rows=self.query_value("SELECT key,data FROM appearance WHERE topicKey=$topic AND ($cursor=NULL OR key>$cursor) AND ($exploration=NULL OR explorationKey=$exploration) AND ($version=NULL OR answerVersionKey=$version) ORDER BY key ASC LIMIT $limit;",json!({"topic":topic,"cursor":cursor,"exploration":exploration,"version":version,"limit":limit+1})).await?;
        let rows = rows.as_array().ok_or_else(storage_error)?;
        let mut items = vec![];
        for row in rows.iter().take(limit) {
            let appearance: AppearanceView =
                serde_json::from_value(row["data"].clone()).map_err(|_| storage_error())?;
            let summary: ExplorationSummary = match self
                .record_data("exploration", appearance.exploration_id.as_str())
                .await
            {
                Ok(summary) => summary,
                Err(e) if e.kind == ErrorCode::NotFound => continue,
                Err(e) => return Err(e),
            };
            items.push(AppearanceLocation {
                exploration_id: appearance.exploration_id,
                appearance_id: appearance.id,
                answer_version_id: appearance.answer_version_id,
                question: appearance.question,
                original_prompt: summary.original_prompt,
                revision: summary.revision,
            });
        }
        let next_cursor = if rows.len() > limit {
            Some(
                rows[limit - 1]["key"]
                    .as_str()
                    .ok_or_else(storage_error)?
                    .to_owned(),
            )
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    pub async fn start_from_topic(
        &self,
        command: &Command<StartFromTopic>,
    ) -> Result<OperationView, AppError> {
        let (command, intent) = Self::intent(command, "start-from-topic")?;
        if let Some(view) = self.replay_existing(&intent, false).await? {
            return Ok(view);
        }
        if command.expected_revision.is_some() {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        let topic: records::TopicRecord = self
            .record_data("topic", command.payload.topic_id.as_str())
            .await?;
        let variant: Value = self
            .record_data("question_variant", topic.id.as_str())
            .await?;
        let resolved: ResolvedQuestion =
            serde_json::from_value(variant["resolved"].clone()).map_err(|_| storage_error())?;
        let pin = command
            .payload
            .answer_version_id
            .or(topic.current_answer_version_id);
        if let Some(pin) = &pin {
            let answer: AnswerVersionView =
                self.record_data("answer_version", pin.as_str()).await?;
            if answer.topic_id != topic.id {
                return Err(AppError::new(ErrorCode::NotFound));
            }
        }
        let exploration_id = ExplorationId::generate();
        let appearance_id = AppearanceId::generate();
        let summary = ExplorationSummary {
            id: exploration_id.clone(),
            original_prompt: topic.question.clone(),
            root_id: appearance_id.clone(),
            revision: 0,
            created_at: intent.created_at.clone(),
            updated_at: intent.created_at.clone(),
        };
        let appearance = AppearanceView {
            id: appearance_id.clone(),
            exploration_id: exploration_id.clone(),
            parent_id: None,
            topic_id: topic.id.clone(),
            question: topic.question,
            locale: resolved.locale,
            clarification: resolved.clarification,
            ancestors: resolved.ancestors,
            interpretation: resolved.interpretation,
            relationship: String::new(),
            teaser: String::new(),
            reason: None,
            reason_sources: vec![],
            reason_answer_version_id: None,
            answer_version_id: pin.clone(),
            child_ids: vec![],
        };
        let body = "IF !record::exists(type::record('topic',$topic)) { THROW 'missing_reference'; }; IF $pin!=NULL AND (SELECT VALUE topicKey FROM ONLY type::record('answer_version',$pin))!=$topic { THROW 'missing_reference'; }; CREATE ONLY type::record('exploration',$explorationKey) CONTENT $exploration; CREATE ONLY type::record('appearance',$appearanceKey) CONTENT $appearance;";
        self.commit_mutation(&intent,body,json!({"topic":topic.id,"pin":pin,"explorationKey":exploration_id,"appearanceKey":appearance_id,"exploration":{"key":exploration_id,"revision":0,"updatedAt":intent.created_at,"data":summary},"appearance":appearance_record(&appearance)}),OperationResult::Exploration{exploration_id,appearance_id,revision:0}).await
    }
}
