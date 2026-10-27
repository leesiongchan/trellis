//! Reversible model/version-scoped edges and bounded native graph walks.
use super::*;
use crate::{contracts::library::*, embeddings::*, knowledge::matching::*};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashSet};
#[derive(Serialize, Deserialize)]
struct Cursor {
    rank: f64,
    key: ConnectionId,
}
const VISIBLE: &str = "data.status!='dismissed' AND (IF data.kind='branches_to' { true } ELSE { (library:main.activeModelVersion=NONE OR data.modelVersion=library:main.activeModelVersion) AND (data.candidate.from.passage=NULL OR data.candidate.from.passage.location.answerVersionId=(SELECT VALUE data.currentAnswerVersionId FROM ONLY type::record('topic',$parent.data.candidate.from.topicId)) OR array::len((SELECT VALUE key FROM appearance WHERE answerVersionKey=$parent.data.candidate.from.passage.location.answerVersionId LIMIT 1))>0) AND (data.candidate.to.passage=NULL OR data.candidate.to.passage.location.answerVersionId=(SELECT VALUE data.currentAnswerVersionId FROM ONLY type::record('topic',$parent.data.candidate.to.topicId)) OR array::len((SELECT VALUE key FROM appearance WHERE answerVersionKey=$parent.data.candidate.to.passage.location.answerVersionId LIMIT 1))>0) })";
impl KnowledgeStore {
    pub(crate) async fn embedding_source(
        &self,
        topic: &TopicId,
        version: Option<&AnswerVersionId>,
        epoch: u64,
    ) -> Result<EmbeddingSource, AppError> {
        if self.library_info().await?.library_epoch != epoch {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        let (content, key) = if let Some(version) = version {
            let answer: AnswerVersionView =
                self.record_data("answer_version", version.as_str()).await?;
            if answer.topic_id != *topic {
                return Err(AppError::new(ErrorCode::NotFound));
            }
            (
                SourceContent::Passage {
                    content: answer.content,
                },
                format!("a-{version}"),
            )
        } else {
            let value: Value = self.record_data("question_variant", topic.as_str()).await?;
            let resolved =
                serde_json::from_value(value["resolved"].clone()).map_err(|_| storage_error())?;
            (SourceContent::Question { resolved }, format!("q-{topic}"))
        };
        let input_hash =
            records::canonical_hash(&serde_json::to_value(&content).map_err(|_| storage_error())?);
        Ok(EmbeddingSource {
            key,
            library_epoch: epoch,
            topic_id: topic.clone(),
            answer_version_id: version.cloned(),
            input_hash,
            content,
        })
    }
    async fn validate_endpoint(
        &self,
        endpoint: &MatchEndpoint,
        epoch: u64,
    ) -> Result<(), AppError> {
        let version = endpoint
            .passage
            .as_ref()
            .map(|p| &p.location.answer_version_id);
        let source = self
            .embedding_source(&endpoint.topic_id, version, epoch)
            .await?;
        let location = endpoint.passage.as_ref().map(|p| ParagraphLocation {
            paragraph_id: p.location.paragraph_id.clone(),
            section_index: p.location.section_index,
            part_index: 0,
            token_start: 0,
            token_end: 0,
        });
        let expected = MatchEndpoint::from_source(&source, location.as_ref())?;
        if serde_json::to_value(expected).map_err(|_| storage_error())?
            != serde_json::to_value(endpoint).map_err(|_| storage_error())?
        {
            return Err(AppError::invalid_question());
        }
        Ok(())
    }
    pub async fn save_candidates(
        &self,
        candidates: &[MatchCandidate],
    ) -> Result<Vec<ConnectionView>, AppError> {
        if candidates.len() > 10 {
            return Err(AppError::invalid_question());
        }
        if candidates.is_empty() {
            return Ok(vec![]);
        }
        let epoch = candidates[0].library_epoch;
        let mut entries = vec![];
        for c in candidates {
            if c.library_epoch != epoch
                || c.from.topic_id == c.to.topic_id
                || !c.order_rank.is_finite()
                || !(0.0..=1.0).contains(&c.order_rank)
                || c.provenance.len() > 3
            {
                return Err(AppError::invalid_question());
            }
            self.validate_endpoint(&c.from, epoch).await?;
            self.validate_endpoint(&c.to, epoch).await?;
            let expected = records::canonical_hash(
                &json!({"model":c.model_version,"from":[c.from.source_key,c.from.input_hash],"to":[c.to.source_key,c.to.input_hash]}),
            );
            if c.id.as_str() != expected || c.from.source_key > c.to.source_key {
                return Err(AppError::invalid_question());
            }
            for p in &c.provenance {
                if p.vector_score
                    .is_some_and(|v| !v.is_finite() || !(-1.0..=1.0).contains(&v))
                    || p.vector_rank.is_some_and(|v| !(1..=10).contains(&v))
                    || p.keyword_rank.is_some_and(|v| !(1..=10).contains(&v))
                {
                    return Err(AppError::invalid_question());
                }
            }
            let view = c.view();
            let mut data = serde_json::to_value(&view).map_err(|_| storage_error())?;
            data["candidate"] = json!(c);
            data["orderRank"] = json!(c.order_rank);
            entries.push(json!({"key":c.id,"from":view.from_topic_id,"to":view.to_topic_id,"record":{"key":c.id,"identityKey":c.id,"data":data}}));
        }
        self.query("BEGIN TRANSACTION; IF library:main.epoch!=$epoch { THROW 'library_epoch_changed'; }; FOR $entry IN $entries { IF !record::exists(type::record('topic',$entry.from)) OR !record::exists(type::record('topic',$entry.to)) { THROW 'missing_reference'; }; LET $edge=type::record('connection',$entry.key); IF !record::exists($edge) { LET $from=type::record('topic',$entry.from); LET $to=type::record('topic',$entry.to); RELATE $from->$edge->$to CONTENT $entry.record; } ELSE { IF (SELECT VALUE data.status FROM ONLY $edge)='candidate' AND (SELECT VALUE data.score FROM ONLY $edge)<$entry.record.data.score { UPDATE ONLY $edge SET data=$entry.record.data; }; }; }; COMMIT TRANSACTION;",json!({"epoch":epoch,"entries":entries})).await?;
        let values = self
            .query_value(
                "SELECT VALUE data FROM connection WHERE key IN $keys LIMIT 10;",
                json!({"keys":candidates.iter().map(|c|&c.id).collect::<Vec<_>>()}),
            )
            .await?;
        serde_json::from_value(values).map_err(|_| storage_error())
    }
    pub async fn connections(
        &self,
        topic: &TopicId,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Page<ConnectionView>, AppError> {
        if !(1..=100).contains(&limit) {
            return Err(AppError::invalid_question());
        }
        let _: records::TopicRecord = self.record_data("topic", topic.as_str()).await?;
        let cursor = cursor
            .map(|s| {
                if s.len() > 1000 {
                    return Err(AppError::invalid_question());
                }
                let bytes = URL_SAFE_NO_PAD
                    .decode(s)
                    .map_err(|_| AppError::invalid_question())?;
                let c: Cursor =
                    serde_json::from_slice(&bytes).map_err(|_| AppError::invalid_question())?;
                if !c.rank.is_finite() || !(0.0..=1.0).contains(&c.rank) {
                    return Err(AppError::invalid_question());
                }
                Ok(c)
            })
            .transpose()?;
        let sql = format!(
            "SELECT key,data,(data.orderRank ?? 0) AS orderRank FROM connection WHERE (data.fromTopicId=$topic OR data.toTopicId=$topic) AND {VISIBLE} AND ($cursor=NULL OR (data.orderRank ?? 0)<$rank OR ((data.orderRank ?? 0)=$rank AND key>$cursor)) ORDER BY orderRank DESC,key ASC LIMIT $limit;"
        );
        let value=self.query_value(&sql,json!({"topic":topic,"cursor":cursor.as_ref().map(|c|&c.key),"rank":cursor.as_ref().map(|c|c.rank),"limit":limit+1})).await?;
        let rows = value.as_array().ok_or_else(storage_error)?;
        let mut items = vec![];
        for row in rows.iter().take(limit) {
            items.push(serde_json::from_value(row["data"].clone()).map_err(|_| storage_error())?);
        }
        let keys: BTreeSet<TopicId> = items
            .iter()
            .flat_map(|item: &ConnectionView| {
                [item.from_topic_id.clone(), item.to_topic_id.clone()]
            })
            .collect();
        let topics: Vec<records::TopicRecord> = serde_json::from_value(
            self.query_value(
                "SELECT VALUE data FROM topic WHERE key IN $keys LIMIT 200;",
                json!({"keys":keys}),
            )
            .await?,
        )
        .map_err(|_| storage_error())?;
        let labels: std::collections::BTreeMap<_, _> = topics
            .into_iter()
            .map(|topic| (topic.id, topic.question))
            .collect();
        for item in &mut items {
            if let Some(label) = labels.get(&item.from_topic_id) {
                item.from_question = label.clone();
            }
            if let Some(label) = labels.get(&item.to_topic_id) {
                item.to_question = label.clone();
            }
        }
        let next_cursor = if rows.len() > limit {
            let last = &rows[limit - 1];
            let c = Cursor {
                rank: last["orderRank"].as_f64().ok_or_else(storage_error)?,
                key: ConnectionId::new(last["key"].as_str().ok_or_else(storage_error)?)?,
            };
            Some(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&c).map_err(|_| storage_error())?))
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    pub async fn dismiss_connection(
        &self,
        command: &Command<DismissConnection>,
    ) -> Result<OperationView, AppError> {
        let (command, intent) = Self::intent(command, "dismiss-connection")?;
        if let Some(view) = self.replay_existing(&intent, false).await? {
            return Ok(view);
        }
        if command.expected_revision.is_some() {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
        let view: ConnectionView = self
            .record_data("connection", command.payload.connection_id.as_str())
            .await?;
        if view.kind == ConnectionKind::BranchesTo {
            return Err(AppError::invalid_question());
        }
        self.commit_mutation(&intent,"IF !record::exists(type::record('connection',$key)) { THROW 'missing_reference'; }; UPDATE ONLY type::record('connection',$key) SET data.status='dismissed';",json!({"key":command.payload.connection_id}),OperationResult::Connection{connection_id:command.payload.connection_id}).await
    }
    pub async fn neighborhood(
        &self,
        topic: &TopicId,
        bounds: GraphBounds,
    ) -> Result<GraphNeighborhood, AppError> {
        if bounds.hops > 3
            || !(1..=100).contains(&bounds.nodes)
            || !(1..=200).contains(&bounds.edges)
        {
            return Err(AppError::invalid_question());
        }
        tokio::time::timeout(std::time::Duration::from_secs(2),async{
   let root_value=self.query_value("SELECT VALUE data FROM ONLY type::record('topic',$key) TIMEOUT 2s;",json!({"key":topic})).await?;if root_value.is_null(){return Err(AppError::new(ErrorCode::NotFound));}let root:records::TopicRecord=serde_json::from_value(root_value).map_err(|_|storage_error())?;let mut nodes=vec![GraphNode{id:root.id.clone(),question:root.question,current_answer_version_id:root.current_answer_version_id}];let mut seen=BTreeSet::from([root.id.clone()]);let mut edge_ids=HashSet::new();let mut frontier=vec![root.id];let mut edges=vec![];let mut truncated=false;
   for _depth in 0..bounds.hops{
    if frontier.is_empty(){break;}let sql=format!("SELECT VALUE data FROM connection WHERE (data.fromTopicId IN $frontier OR data.toTopicId IN $frontier) AND {VISIBLE} AND key NOT IN $seenEdges ORDER BY key ASC LIMIT $limit TIMEOUT 2s;");
    let value=self.query_value(&sql,json!({"frontier":frontier,"seenEdges":edge_ids.iter().collect::<Vec<_>>(),"limit":bounds.edges.saturating_sub(edges.len())+1})).await?;
    let found:Vec<ConnectionView>=serde_json::from_value(value).map_err(|_|storage_error())?;let mut next=BTreeSet::new();
    for edge in found{
     if edges.len()==bounds.edges{truncated=true;break;}
     let mut missing=vec![];for id in [&edge.from_topic_id,&edge.to_topic_id]{if !seen.contains(id){missing.push(id.clone());}}
     if nodes.len()+missing.len()>bounds.nodes{truncated=true;continue;}
     for id in missing{let value=self.query_value("SELECT VALUE data FROM ONLY type::record('topic',$key) TIMEOUT 2s;",json!({"key":id})).await?;let t:records::TopicRecord=serde_json::from_value(value).map_err(|_|storage_error())?;seen.insert(id.clone());next.insert(id);nodes.push(GraphNode{id:t.id,question:t.question,current_answer_version_id:t.current_answer_version_id});}
     edge_ids.insert(edge.id.clone());edges.push(edge);
    }
    frontier=next.into_iter().collect();
   }
   if !frontier.is_empty() && !truncated {
    let sql=format!("SELECT VALUE key FROM connection WHERE (data.fromTopicId IN $frontier OR data.toTopicId IN $frontier) AND {VISIBLE} AND key NOT IN $seenEdges LIMIT 1 TIMEOUT 2s;");
    let more=self.query_value(&sql,json!({"frontier":frontier,"seenEdges":edge_ids.iter().collect::<Vec<_>>()})).await?;
    truncated=more.as_array().is_some_and(|v|!v.is_empty());
   }
   Ok(GraphNeighborhood{nodes,edges,truncated,bounds})
  }).await.map_err(|_|AppError::new(ErrorCode::StorageBusy))?
    }
}

impl KnowledgeStore {
    pub(crate) async fn claim_verification(
        &self,
        attempt: &super::answers::ResearchAttempt,
        candidates: &[MatchCandidate],
        provider_model: &str,
    ) -> Result<bool, AppError> {
        if candidates.is_empty() || candidates.len() > 3 {
            return Ok(false);
        }
        if attempt.reuse_version.is_some() {
            return Ok(false);
        }
        if provider_model.is_empty()
            || provider_model.len() > 200
            || candidates.iter().any(|c| {
                c.library_epoch != attempt.library_epoch
                    || (c.from.topic_id != attempt.topic_id && c.to.topic_id != attempt.topic_id)
            })
        {
            return Err(AppError::invalid_question());
        }
        let nonce = uuid::Uuid::new_v4().to_string();
        self.query("BEGIN TRANSACTION; IF library:main.epoch!=$epoch { THROW 'library_epoch_changed'; }; IF (SELECT VALUE data.state FROM ONLY type::record('research_attempt',$key))!='completed' { THROW 'work_interrupted'; }; IF (SELECT VALUE attemptKey FROM ONLY type::record('active_attempt',$topic))!=$key { THROW 'work_interrupted'; }; IF !record::exists(type::record('verification_stage',$key)) { CREATE ONLY type::record('verification_stage',$key) CONTENT {key:$key,attemptKey:$key,data:{state:'started',owner:$owner,candidates:$candidates,startedAt:$now,providerModel:$providerModel}}; }; COMMIT TRANSACTION;",json!({"epoch":attempt.library_epoch,"key":attempt.id,"owner":nonce,"candidates":candidates,"now":records::now()?,"providerModel":provider_model,"topic":attempt.topic_id})).await?;
        let value = self
            .query_value(
                "SELECT VALUE data.owner FROM ONLY type::record('verification_stage',$key);",
                json!({"key":attempt.id}),
            )
            .await?;
        Ok(value == nonce)
    }
    pub(crate) async fn finish_verification(
        &self,
        attempt: &super::answers::ResearchAttempt,
        candidates: &[MatchCandidate],
        result: &Result<crate::knowledge::verification::ConnectionVerdict, AppError>,
    ) -> Result<(), AppError> {
        use crate::knowledge::verification::{VerdictKind, equivalent_compatible};
        if let Ok(verdict) = result {
            verdict.validate(candidates)?;
        }
        let mut updates = vec![];
        if let Ok(verdict) = result {
            for decision in &verdict.decisions {
                let c = candidates
                    .iter()
                    .find(|c| c.id == decision.candidate_id)
                    .ok_or_else(AppError::invalid_question)?;
                let mut view = c.view();
                match decision.verdict {
                    VerdictKind::Equivalent if equivalent_compatible(c) => {
                        view.kind = ConnectionKind::EquivalentQuestion;
                        view.status = ConnectionStatus::Accepted;
                        view.reason = decision.reason.clone();
                        view.method = "checked question equivalence".into();
                    }
                    VerdictKind::Related => {
                        let supported = [&c.from, &c.to].iter().all(|e| {
                            e.passage.as_ref().is_some_and(|p| {
                                !p.citations.is_empty()
                                    && decision.paragraph_refs.iter().any(|r| {
                                        r.answer_version_id == p.location.answer_version_id
                                            && r.paragraph_id == p.location.paragraph_id
                                            && r.section_index == p.location.section_index
                                    })
                            })
                        });
                        let mut grounded = supported;
                        if grounded {
                            for endpoint in [&c.from, &c.to] {
                                if let Some(passage) = &endpoint.passage {
                                    let answer: AnswerVersionView = self
                                        .record_data(
                                            "answer_version",
                                            passage.location.answer_version_id.as_str(),
                                        )
                                        .await?;
                                    grounded &= answer.origin == AnswerOrigin::Research;
                                }
                            }
                        }
                        if grounded {
                            view.kind = ConnectionKind::RelatedTopic;
                            view.status = ConnectionStatus::Accepted;
                            view.reason = decision.reason.clone();
                            view.method = "checked cited relationship".into();
                        }
                    }
                    VerdictKind::Different => {
                        view.status = ConnectionStatus::Dismissed;
                        view.reason = decision.reason.clone();
                        view.method = "checked different questions".into();
                    }
                    _ => {}
                }
                let mut data = serde_json::to_value(view).map_err(|_| storage_error())?;
                data["candidate"] = json!(c);
                data["verification"] = json!(decision);
                data["orderRank"] = json!(c.order_rank);
                updates.push(json!({"key":c.id,"data":data}));
            }
        }
        self.query("BEGIN TRANSACTION; IF library:main.epoch=$epoch AND (SELECT VALUE data.state FROM ONLY type::record('verification_stage',$key))='started' { FOR $entry IN $updates { UPDATE ONLY type::record('connection',$entry.key) SET data=$entry.data WHERE data.status='candidate'; }; UPDATE ONLY type::record('verification_stage',$key) SET data.state=$state,data.error=$error,data.finishedAt=$now; }; COMMIT TRANSACTION;",json!({"epoch":attempt.library_epoch,"key":attempt.id,"updates":updates,"state":if result.is_ok(){"completed"}else{"failed"},"error":result.as_ref().err().map(|e|e.code()),"now":records::now()?})).await?;
        Ok(())
    }
    pub(crate) async fn release_research_attempt(
        &self,
        attempt: &super::answers::ResearchAttempt,
        interrupted: bool,
    ) -> Result<(), AppError> {
        self.query("BEGIN TRANSACTION; IF library:main.epoch=$epoch { DELETE active_attempt WHERE attemptKey=$key; IF $interrupted AND (SELECT VALUE data.state FROM ONLY type::record('verification_stage',$key))='started' { UPDATE ONLY type::record('verification_stage',$key) SET data.state='interrupted',data.finishedAt=$now; }; }; COMMIT TRANSACTION;",json!({"epoch":attempt.library_epoch,"key":attempt.id,"interrupted":interrupted,"now":records::now()?})).await?;
        Ok(())
    }
}
