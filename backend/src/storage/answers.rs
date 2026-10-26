//! Durable research attempts, followers and immutable answer commits.
use super::{records::ResolvedQuestion, *};
use crate::contracts::{ResearchRequest, library::*};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResearchAttempt {
    pub id: AttemptId,
    pub topic_id: TopicId,
    pub library_epoch: u64,
    pub request: ResearchRequest,
    pub resolved: ResolvedQuestion,
    pub reuse_version: Option<AnswerVersionId>,
    pub state: OperationState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResearchFollower {
    pub operation_id: OperationId,
    pub appearance: AppearanceView,
    pub pin_revision: u64,
}
pub(crate) struct ResearchReservation {
    pub operation: OperationView,
    pub attempt: Option<ResearchAttempt>,
    pub leader: bool,
}
impl KnowledgeStore {
    pub(crate) async fn reserve_research(
        &self,
        command: &Command<ResearchInput>,
    ) -> Result<ResearchReservation, AppError> {
        let (command, intent) = Self::intent(command, "research")?;
        if let Some(operation) = self.replay_existing(&intent, false).await? {
            return Ok(ResearchReservation {
                operation,
                attempt: None,
                leader: false,
            });
        }
        let appearance: AppearanceView = self
            .record_data("appearance", command.payload.appearance_id.as_str())
            .await?;
        let expected_revision = command
            .expected_revision
            .ok_or_else(|| AppError::new(ErrorCode::RevisionConflict))?;
        let clarification = command
            .payload
            .clarification
            .or(appearance.clarification.clone());
        let resolved = ResolvedQuestion::new(
            &appearance.question,
            &appearance.locale,
            clarification.clone(),
            appearance.ancestors.clone(),
            &appearance.interpretation,
        )?;
        let (topic, mut bindings) =
            Self::topic_bindings(&resolved, &appearance.question, &intent.created_at)?;
        let current: Option<records::TopicRecord> =
            match self.record_data("topic", topic.as_str()).await {
                Ok(v) => Some(v),
                Err(e) if e.kind == ErrorCode::NotFound => None,
                Err(e) => return Err(e),
            };
        let existing_pin = if topic == appearance.topic_id {
            appearance.answer_version_id.clone()
        } else {
            None
        };
        let reuse_version = if command.payload.refresh {
            None
        } else {
            existing_pin.or_else(|| current.and_then(|v| v.current_answer_version_id))
        };
        let reuse = reuse_version.is_some();
        let candidate = AttemptId::generate();
        let attempt = ResearchAttempt {
            id: candidate.clone(),
            topic_id: topic.clone(),
            library_epoch: command.library_epoch,
            request: ResearchRequest {
                exploration_id: appearance.exploration_id.to_string(),
                node_id: appearance.id.to_string(),
                question: appearance.question.clone(),
                clarification,
                ancestors: appearance.ancestors.clone(),
            },
            resolved,
            reuse_version,
            state: OperationState::Pending,
        };
        let pending = OperationView {
            id: intent.id.clone(),
            state: OperationState::Pending,
            result: None,
            error: None,
            created_at: intent.created_at.clone(),
            updated_at: intent.created_at.clone(),
        };
        bindings["epoch"] = json!(command.library_epoch);
        bindings["reuse"] = json!(reuse);
        bindings["operationKey"] = json!(intent.id);
        bindings["requestHash"] = json!(intent.request_hash);
        bindings["target"] = json!(intent.target);
        bindings["appearanceKey"] = json!(appearance.id);
        bindings["explorationKey"] = json!(appearance.exploration_id);
        bindings["expectedRevision"] = json!(expected_revision);
        bindings["candidateKey"] = json!(candidate);
        bindings["operationRecord"] = json!({"key":intent.id,"requestHash":intent.request_hash,"libraryEpoch":intent.library_epoch,"target":intent.target,"data":pending});
        bindings["attemptRecord"] =
            json!({"key":candidate,"topicKey":topic,"operationKey":intent.id,"data":attempt});
        bindings["activeRecord"] =
            json!({"key":topic,"topicKey":topic,"attemptKey":candidate,"data":{}});
        let pin_revision = self
            .query_value(
                "SELECT VALUE pinRevision FROM ONLY type::record('appearance',$key);",
                json!({"key":appearance.id}),
            )
            .await?
            .as_u64()
            .unwrap_or(0);
        bindings["follower"] = json!(ResearchFollower {
            operation_id: intent.id.clone(),
            appearance,
            pin_revision,
        });
        let sql = format!(
            "BEGIN TRANSACTION; IF library:main.epoch!=$epoch {{ THROW 'library_epoch_changed'; }}; LET $existing=(SELECT * FROM ONLY type::record('operation',$operationKey)); IF $existing!=NONE {{ IF $existing.requestHash!=$requestHash OR $existing.target!=$target {{ THROW 'intent_conflict'; }}; }} ELSE {{ IF !record::exists(type::record('appearance',$appearanceKey)) {{ THROW 'missing_reference'; }}; IF (SELECT VALUE revision FROM ONLY type::record('exploration',$explorationKey))!=$expectedRevision {{ THROW 'revision_conflict'; }}; {} LET $active=IF $reuse {{ NONE }} ELSE {{ (SELECT VALUE attemptKey FROM ONLY type::record('active_attempt',$topicKey)) }}; IF $active!=NONE AND (SELECT VALUE data.state FROM ONLY type::record('research_attempt',$active))!='pending' {{ THROW 'research_limit'; }}; LET $attemptKey=IF $active=NONE {{ $candidateKey }} ELSE {{ $active }}; IF $active=NONE {{ CREATE ONLY type::record('research_attempt',$candidateKey) CONTENT $attemptRecord; IF !$reuse {{ CREATE ONLY type::record('active_attempt',$topicKey) CONTENT $activeRecord; }}; }}; IF array::len((SELECT VALUE key FROM research_follower WHERE attemptKey=$attemptKey LIMIT 33))>=32 {{ THROW 'research_limit'; }}; CREATE ONLY type::record('operation',$operationKey) CONTENT object::extend($operationRecord,{{attemptKey:$attemptKey}}); CREATE ONLY type::record('research_follower',$operationKey) CONTENT {{key:$operationKey,attemptKey:$attemptKey,appearanceKey:$appearanceKey,data:$follower}}; }}; COMMIT TRANSACTION;",
            super::library::ENSURE_TOPIC
        );
        if let Err(error) = self.query(&sql, bindings).await {
            if let Some(operation) = self.replay_existing(&intent, false).await? {
                return Ok(ResearchReservation {
                    operation,
                    attempt: None,
                    leader: false,
                });
            }
            return Err(error);
        }
        let operation = self.operation(&intent.id).await?;
        let key = self
            .query_value(
                "SELECT VALUE attemptKey FROM ONLY type::record('operation',$key);",
                json!({"key":intent.id}),
            )
            .await?;
        let id = AttemptId::new(key.as_str().ok_or_else(storage_error)?)?;
        let attempt = self.record_data("research_attempt", id.as_str()).await?;
        Ok(ResearchReservation {
            operation,
            attempt: Some(attempt),
            leader: id == candidate,
        })
    }
    pub(crate) async fn fail_attempt(
        &self,
        attempt: &ResearchAttempt,
        error: AppError,
    ) -> Result<(), AppError> {
        let state = if matches!(
            error.kind,
            ErrorCode::ResearchTimeout | ErrorCode::WorkInterrupted
        ) {
            OperationState::Interrupted
        } else {
            OperationState::Failed
        };
        self.query("BEGIN TRANSACTION; IF library:main.epoch=$epoch AND (SELECT VALUE data.state FROM ONLY type::record('research_attempt',$key))='pending' { LET $followers=(SELECT VALUE data FROM research_follower WHERE attemptKey=$key LIMIT 33); FOR $follower IN $followers { UPDATE ONLY type::record('operation',$follower.operationId) SET data.state=$state,data.error=$error,data.updatedAt=$now WHERE data.state='pending'; }; UPDATE ONLY type::record('research_attempt',$key) SET data.state=$state; DELETE active_attempt WHERE attemptKey=$key; DELETE research_follower WHERE attemptKey=$key; }; COMMIT TRANSACTION;",json!({"epoch":attempt.library_epoch,"key":attempt.id,"state":state,"error":OperationError::from(error),"now":records::now()?})).await?;
        Ok(())
    }
    pub async fn choose_answer(
        &self,
        command: &Command<ChooseAnswer>,
    ) -> Result<OperationView, AppError> {
        let (command, intent) = Self::intent(command, "choose-answer")?;
        if let Some(view) = self.replay_existing(&intent, false).await? {
            return Ok(view);
        }
        let appearance: AppearanceView = self
            .record_data("appearance", command.payload.appearance_id.as_str())
            .await?;
        let answer: AnswerVersionView = self
            .record_data("answer_version", command.payload.answer_version_id.as_str())
            .await?;
        if answer.topic_id != appearance.topic_id {
            return Err(AppError::new(ErrorCode::NotFound));
        }
        let revision = command
            .expected_revision
            .ok_or_else(|| AppError::new(ErrorCode::RevisionConflict))?;
        let body = "IF (SELECT VALUE revision FROM ONLY type::record('exploration',$explorationKey))!=$revision { THROW 'revision_conflict'; }; IF (SELECT VALUE topicKey FROM ONLY type::record('answer_version',$version))!=(SELECT VALUE topicKey FROM ONLY type::record('appearance',$appearanceKey)) { THROW 'missing_reference'; }; UPDATE ONLY type::record('appearance',$appearanceKey) SET answerVersionKey=$version,pinRevision=(pinRevision ?? 0)+1,data.answerVersionId=$version; UPDATE ONLY type::record('exploration',$explorationKey) SET revision=$revision+1,updatedAt=$now,data.revision=$revision+1,data.updatedAt=$now;";
        self.commit_mutation(&intent,body,json!({"explorationKey":appearance.exploration_id,"appearanceKey":appearance.id,"version":answer.id,"revision":revision,"now":intent.created_at}),OperationResult::Exploration{exploration_id:appearance.exploration_id,appearance_id:appearance.id,revision:revision+1}).await
    }
}

impl KnowledgeStore {
    pub(crate) async fn complete_attempt(
        &self,
        attempt: &ResearchAttempt,
        outcome: &crate::contracts::ResearchResult,
        keep_active: bool,
    ) -> Result<(), AppError> {
        use crate::contracts::{Ancestor, ResearchResult, truncate_utf16};
        let now = records::now()?;
        let content = match outcome {
            ResearchResult::Topic { content } => Some(content),
            _ => None,
        };
        let version = if content.is_some() {
            Some(
                attempt
                    .reuse_version
                    .clone()
                    .unwrap_or_else(AnswerVersionId::generate),
            )
        } else {
            None
        };
        let mut bindings = json!({"attemptKey":attempt.id,"epoch":attempt.library_epoch,"topicKey":attempt.topic_id,"now":now,"outcome":outcome,"version":version,"hasAnswer":content.is_some(),"newVersion":attempt.reuse_version.is_none(),"clarification":attempt.resolved.clarification,"interruptedError":OperationError::from(AppError::new(ErrorCode::WorkInterrupted)),"keepActive":keep_active});
        let mut branches = vec![];
        if let Some(content) = content {
            bindings["answerRecord"] = json!({"key":version,"topicKey":attempt.topic_id,"revision":0,"data":{"id":version,"topicId":attempt.topic_id,"revision":0,"content":content,"createdAt":now}});
            let mut context = attempt.resolved.ancestors.clone();
            context.push(Ancestor {
                title: truncate_utf16(&content.title, 160),
                summary: truncate_utf16(
                    &content
                        .summary
                        .iter()
                        .map(|p| p.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    400,
                ),
            });
            if context.len() > 15 {
                let mut bounded = vec![context[0].clone()];
                bounded.extend_from_slice(&context[context.len() - 14..]);
                context = bounded;
            }
            for branch in content.branches.iter().take(6) {
                let resolved = ResolvedQuestion::new(
                    &branch.question,
                    &attempt.resolved.locale,
                    None,
                    context.clone(),
                    "literal",
                )?;
                let (topic, mut b) = Self::topic_bindings(&resolved, &branch.question, &now)?;
                b["appearanceTemplate"] = json!({"id":"placeholder","explorationId":"placeholder","parentId":null,"topicId":topic,"question":branch.question,"locale":attempt.resolved.locale,"clarification":null,"ancestors":resolved.ancestors,"interpretation":resolved.interpretation,"relationship":branch.relationship,"reason":branch.reason,"reasonSources":content.sources.iter().filter(|source|branch.reason.evidence.iter().any(|e|e.source_id==source.id)).collect::<Vec<_>>(),"reasonAnswerVersionId":version,"answerVersionId":null,"childIds":[]});
                branches.push(b);
            }
        }
        bindings["branches"] = json!(branches);
        let pin_sql = format!(
            "UPDATE ONLY type::record('appearance',$appearance.id) SET topicKey=$topicKey,answerVersionKey=$version,pinRevision=(pinRevision ?? 0)+IF $appearance.topicId!=$topicKey OR $appearance.answerVersionId!=$version {{ 1 }} ELSE {{ 0 }},data.topicId=$topicKey,data.answerVersionId=$version,data.clarification=$clarification; FOR $branch IN $branches {{ LET $childTopic=$branch.topicKey; IF !record::exists(type::record('topic',$childTopic)) {{ CREATE ONLY type::record('topic',$childTopic) CONTENT $branch.topicRecord; }}; IF !record::exists(type::record('question_variant',$childTopic)) {{ CREATE ONLY type::record('question_variant',$childTopic) CONTENT $branch.variantRecord; }}; LET $existingChild=(SELECT VALUE key FROM appearance WHERE parentKey=$appearance.id AND topicKey=$childTopic LIMIT 1); IF array::len($existingChild)=0 AND array::len((SELECT VALUE key FROM appearance WHERE explorationKey=$appearance.explorationId LIMIT 2001))<2000 {{ LET $childKey=crypto::sha256($appearance.id+'|'+$childTopic); LET $childPin=(SELECT VALUE data.currentAnswerVersionId FROM ONLY type::record('topic',$childTopic)); LET $childData=object::extend($branch.appearanceTemplate,{{id:$childKey,explorationId:$appearance.explorationId,parentId:$appearance.id,answerVersionId:$childPin}}); CREATE ONLY type::record('appearance',$childKey) CONTENT {{key:$childKey,explorationKey:$appearance.explorationId,topicKey:$childTopic,parentKey:$appearance.id,answerVersionKey:$childPin,pinRevision:0,data:$childData}}; UPDATE ONLY type::record('appearance',$appearance.id) SET data.childIds=array::append(data.childIds,$childKey); }}; }}; IF $appearance.parentId!=NULL {{ LET $linkFrom=(SELECT VALUE topicKey FROM ONLY type::record('appearance',$appearance.parentId)); LET $linkTo=$topicKey; LET $linkAppearance=$appearance.id; {} }}; FOR $child IN (SELECT VALUE data FROM appearance WHERE parentKey=$appearance.id LIMIT 2000) {{ LET $linkFrom=$topicKey; LET $linkTo=$child.topicId; LET $linkAppearance=$child.id; {} }}; UPDATE ONLY type::record('exploration',$appearance.explorationId) SET revision=revision+1,updatedAt=$now,data.revision=data.revision+1,data.updatedAt=$now;",
            relink_occurrence_sql(),
            relink_occurrence_sql()
        );
        let sql = format!(
            "BEGIN TRANSACTION; IF library:main.epoch!=$epoch {{ THROW 'library_epoch_changed'; }}; IF (SELECT VALUE data.state FROM ONLY type::record('research_attempt',$attemptKey))!='pending' {{ THROW 'work_interrupted'; }}; LET $followers=(SELECT VALUE data FROM research_follower WHERE attemptKey=$attemptKey LIMIT 33); LET $live=(SELECT VALUE key FROM research_follower WHERE attemptKey=$attemptKey AND record::exists(type::record('appearance',appearanceKey)) LIMIT 33); IF array::len($live)=0 {{ THROW 'work_interrupted'; }}; IF $hasAnswer AND $newVersion {{ LET $revision=((SELECT VALUE data.answerRevision FROM ONLY type::record('topic',$topicKey)) ?? 0)+1; CREATE ONLY type::record('answer_version',$version) CONTENT object::extend($answerRecord,{{revision:$revision,data:object::extend($answerRecord.data,{{revision:$revision}})}}); UPDATE ONLY type::record('topic',$topicKey) SET data.currentAnswerVersionId=$version,data.answerRevision=$revision,data.updatedAt=$now; }}; FOR $follower IN $followers {{ LET $appearance=$follower.appearance; IF record::exists(type::record('appearance',$appearance.id)) AND record::exists(type::record('exploration',$appearance.explorationId)) {{ IF $hasAnswer AND ((SELECT VALUE pinRevision FROM ONLY type::record('appearance',$appearance.id)) ?? 0)=$follower.pinRevision AND (SELECT VALUE topicKey FROM ONLY type::record('appearance',$appearance.id))=$appearance.topicId AND (SELECT VALUE answerVersionKey FROM ONLY type::record('appearance',$appearance.id))=$appearance.answerVersionId {{ {pin_sql} }}; LET $revision=(SELECT VALUE revision FROM ONLY type::record('exploration',$appearance.explorationId)); UPDATE ONLY type::record('operation',$follower.operationId) SET data.state='completed',data.result={{kind:'research',explorationId:$appearance.explorationId,appearanceId:$appearance.id,answerVersionId:$version,revision:$revision,outcome:$outcome}},data.error=NULL,data.updatedAt=$now WHERE data.state='pending'; }} ELSE {{ UPDATE ONLY type::record('operation',$follower.operationId) SET data.state='interrupted',data.error=$interruptedError,data.updatedAt=$now WHERE data.state='pending'; }}; }}; UPDATE ONLY type::record('research_attempt',$attemptKey) SET data.state='completed'; IF !$keepActive {{ DELETE active_attempt WHERE attemptKey=$attemptKey; }}; DELETE research_follower WHERE attemptKey=$attemptKey; COMMIT TRANSACTION;"
        );
        self.query(&sql, bindings).await?;
        Ok(())
    }
}
fn relink_occurrence_sql() -> &'static str {
    "LET $old=(SELECT VALUE connectionKey FROM edge_support WHERE appearanceKey=$linkAppearance LIMIT 2); DELETE edge_support WHERE appearanceKey=$linkAppearance; FOR $oldKey IN $old { IF array::len((SELECT VALUE key FROM edge_support WHERE connectionKey=$oldKey LIMIT 1))=0 { DELETE connection WHERE key=$oldKey AND data.kind='branches_to'; }; }; IF $linkFrom!=NONE AND $linkFrom!=$linkTo { LET $edgeKey=crypto::sha256(encoding::json::encode(['branches_to',$linkFrom,$linkTo])); LET $from=type::record('topic',$linkFrom); LET $to=type::record('topic',$linkTo); LET $edge=type::record('connection',$edgeKey); IF !record::exists($edge) { RELATE $from->$edge->$to CONTENT {key:$edgeKey,identityKey:$edgeKey,data:{id:$edgeKey,fromTopicId:$linkFrom,toTopicId:$linkTo,kind:'branches_to',status:'accepted',reason:'Appears as a branch in an exploration',method:'occurrence',modelVersion:NULL,score:NULL,passages:[],evidence:[]}}; }; CREATE ONLY type::record('edge_support',$linkAppearance) CONTENT {key:$linkAppearance,connectionKey:$edgeKey,appearanceKey:$linkAppearance,data:{}}; };"
}
