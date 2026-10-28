//! Bounded source scans, resumable derived state, and streaming exact cosine.
use super::*;
use crate::{contracts::library::*, embeddings::*};
use serde::{Deserialize, Serialize};
use serde_json::json;
pub(super) const RETRIEVAL_FILTER: &str = "modelVersion=$model AND data.sourceKind=$kind AND ($exclude=NULL OR data.topicId!=$exclude) AND ($cursor=NULL OR key>$cursor) AND ($kind='question' OR !$visible OR data.answerVersionId=(SELECT VALUE data.currentAnswerVersionId FROM ONLY type::record('topic',$parent.data.topicId)) OR array::len((SELECT VALUE key FROM appearance WHERE answerVersionKey=$parent.data.answerVersionId LIMIT 1))>0)";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingCursor {
    pub library_epoch: u64,
    pub kind: SourceKind,
    pub after: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VectorRecord {
    source_kind: SourceKind,
    topic_id: TopicId,
    answer_version_id: Option<AnswerVersionId>,
    location: Option<ParagraphLocation>,
    values: Vec<f32>,
    text: String,
}
impl KnowledgeStore {
    pub async fn embedding_sources(
        &self,
        kind: SourceKind,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Page<EmbeddingSource>, AppError> {
        if !(1..=100).contains(&limit) {
            return Err(AppError::invalid_question());
        }
        if let Some(cursor) = cursor {
            TopicId::new(cursor)?;
        }
        let table = match kind {
            SourceKind::Question => "question_variant",
            SourceKind::Passage => "answer_version",
        };
        let sql = format!(
            "RETURN {{epoch:library:main.epoch,items:(SELECT key,topicKey,data FROM {table} WHERE $cursor=NULL OR key>$cursor ORDER BY key ASC LIMIT $limit)}};"
        );
        let value = self
            .query_value(&sql, json!({"cursor":cursor,"limit":limit+1}))
            .await?;
        let epoch = value["epoch"].as_u64().ok_or_else(storage_error)?;
        let rows = value["items"].as_array().ok_or_else(storage_error)?;
        let mut items = vec![];
        for row in rows.iter().take(limit) {
            let key = row["key"].as_str().ok_or_else(storage_error)?;
            let (content, version) = match kind {
                SourceKind::Question => (
                    SourceContent::Question {
                        resolved: serde_json::from_value(row["data"]["resolved"].clone())
                            .map_err(|_| storage_error())?,
                    },
                    None,
                ),
                SourceKind::Passage => (
                    SourceContent::Passage {
                        content: serde_json::from_value(row["data"]["content"].clone())
                            .map_err(|_| storage_error())?,
                    },
                    Some(AnswerVersionId::new(key)?),
                ),
            };
            let input_hash = records::canonical_hash(
                &serde_json::to_value(&content).map_err(|_| storage_error())?,
            );
            items.push(EmbeddingSource {
                key: format!(
                    "{}-{key}",
                    if kind == SourceKind::Question {
                        "q"
                    } else {
                        "a"
                    }
                ),
                library_epoch: epoch,
                topic_id: TopicId::new(row["topicKey"].as_str().ok_or_else(storage_error)?)?,
                answer_version_id: version,
                input_hash,
                content,
            });
        }
        let next_cursor = if rows.len() > limit {
            rows.get(limit - 1)
                .and_then(|r| r["key"].as_str())
                .map(str::to_owned)
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    pub(crate) async fn register_embedding_model(
        &self,
        manifest: &ModelManifest,
    ) -> Result<(), AppError> {
        if manifest.dimensions != 384 {
            return Err(AppError::invalid_question());
        }
        self.query("IF !record::exists(type::record('embedding_model',$key)) { CREATE ONLY type::record('embedding_model',$key) CONTENT {key:$key,data:$manifest}; };",json!({"key":manifest.version(),"manifest":manifest})).await?;
        Ok(())
    }
    pub async fn index_source(
        &self,
        source: &EmbeddingSource,
        manifest: &ModelManifest,
        vectors: &[Vec<f32>],
    ) -> Result<(), AppError> {
        if manifest.dimensions != 384 {
            return Err(AppError::invalid_question());
        }
        let prepared = source.prepare(manifest)?;
        if vectors.len() != prepared.inputs.len() || vectors.len() > 64 {
            return Err(AppError::invalid_question());
        }
        let version = manifest.version();
        let state = state_key(source, &version);
        let mut entries = vec![];
        for (index, values) in vectors.iter().enumerate() {
            let vector = SearchVector::new(version.clone(), values.clone(), manifest.dimensions)
                .map_err(|_| AppError::invalid_question())?;
            let key = records::canonical_hash(&json!([state, index]));
            entries.push(json!({"key":key,"modelVersion":version,"sourceKey":source.key,"inputHash":source.input_hash,"partIndex":index,"data":{"sourceKind":source.kind(),"topicId":source.topic_id,"answerVersionId":source.answer_version_id,"location":prepared.locations[index],"values":vector.values,"text":prepared.inputs[index].text}}));
        }
        let (table, source_id) = if let Some(id) = &source.answer_version_id {
            ("answer_version", id.as_str())
        } else {
            ("question_variant", source.topic_id.as_str())
        };
        self.query("BEGIN TRANSACTION; IF library:main.epoch!=$epoch { THROW 'library_epoch_changed'; }; IF !record::exists(type::record($table,$sourceId)) { THROW 'missing_reference'; }; IF !record::exists(type::record('embedding_model',$model)) { CREATE ONLY type::record('embedding_model',$model) CONTENT {key:$model,data:$manifest}; }; FOR $entry IN $entries { UPSERT type::record('embedding',$entry.key) CONTENT $entry; }; UPSERT type::record('embedding_state',$stateKey) CONTENT {key:$stateKey,modelVersion:$model,sourceKey:$sourceKey,inputHash:$inputHash,data:{state:'completed',partialCoverage:$partial,entries:array::len($entries)}}; COMMIT TRANSACTION;",json!({"epoch":source.library_epoch,"table":table,"sourceId":source_id,"model":version,"manifest":manifest,"entries":entries,"stateKey":state,"sourceKey":source.key,"inputHash":source.input_hash,"partial":prepared.partial_coverage})).await?;
        Ok(())
    }
    pub(crate) async fn embedding_completed(
        &self,
        source: &EmbeddingSource,
        model: &str,
    ) -> Result<bool, AppError> {
        let value = self
            .query_value(
                "SELECT VALUE data.state FROM ONLY type::record('embedding_state',$key);",
                json!({"key":state_key(source,model)}),
            )
            .await?;
        Ok(value == "completed")
    }
    pub(crate) async fn matching_completed(
        &self,
        source: &EmbeddingSource,
        model: &str,
    ) -> Result<bool, AppError> {
        let value = self
            .query_value(
                "SELECT VALUE data.state FROM ONLY type::record('match_state',$key);",
                json!({"key":state_key(source,model)}),
            )
            .await?;
        Ok(value == "completed")
    }
    pub(crate) async fn complete_matching(
        &self,
        source: &EmbeddingSource,
        model: &str,
    ) -> Result<(), AppError> {
        self.query("BEGIN TRANSACTION; IF library:main.epoch=$epoch { UPSERT type::record('match_state',$key) CONTENT {key:$key,data:{state:'completed'}}; }; COMMIT TRANSACTION;", json!({"epoch":source.library_epoch,"key":state_key(source,model)})).await?;
        Ok(())
    }
    pub(crate) async fn embedding_failed(
        &self,
        source: &EmbeddingSource,
        model: &str,
    ) -> Result<(), AppError> {
        self.query("BEGIN TRANSACTION; IF library:main.epoch=$epoch { UPSERT type::record('embedding_state',$key) CONTENT {key:$key,modelVersion:$model,sourceKey:$source,inputHash:$hash,data:{state:'failed',error:'LOCAL_INFERENCE_UNAVAILABLE'}}; }; COMMIT TRANSACTION;",json!({"epoch":source.library_epoch,"key":state_key(source,model),"model":model,"source":source.key,"hash":source.input_hash})).await?;
        Ok(())
    }
    pub(crate) async fn embedding_cursor(&self, model: &str) -> Result<EmbeddingCursor, AppError> {
        let epoch = self.library_info().await?.library_epoch;
        match self
            .record_data::<EmbeddingCursor>("embedding_cursor", model)
            .await
        {
            Ok(c) if c.library_epoch == epoch => Ok(c),
            Ok(_) => Ok(EmbeddingCursor {
                library_epoch: epoch,
                kind: SourceKind::Question,
                after: None,
            }),
            Err(e) if e.kind == ErrorCode::NotFound => Ok(EmbeddingCursor {
                library_epoch: epoch,
                kind: SourceKind::Question,
                after: None,
            }),
            Err(e) => Err(e),
        }
    }
    pub(crate) async fn save_embedding_cursor(
        &self,
        model: &str,
        cursor: &EmbeddingCursor,
    ) -> Result<(), AppError> {
        self.query("BEGIN TRANSACTION; IF library:main.epoch=$epoch { UPSERT type::record('embedding_cursor',$key) CONTENT {key:$key,data:$data}; }; COMMIT TRANSACTION;",json!({"epoch":cursor.library_epoch,"key":model,"data":cursor})).await?;
        Ok(())
    }
    pub(crate) async fn embedding_backfill_complete(&self, model: &str) -> Result<bool, AppError> {
        let value=self.query_value("RETURN {questions:array::len((SELECT VALUE key FROM question_variant WHERE array::len((SELECT VALUE key FROM embedding_state WHERE modelVersion=$model AND sourceKey='q-'+$parent.key AND data.state='completed' LIMIT 1))=0 LIMIT 1)),answers:array::len((SELECT VALUE key FROM answer_version WHERE array::len((SELECT VALUE key FROM embedding_state WHERE modelVersion=$model AND sourceKey='a-'+$parent.key AND data.state='completed' LIMIT 1))=0 LIMIT 1))};",json!({"model":model})).await?;
        Ok(value["questions"] == 0 && value["answers"] == 0)
    }
    pub(crate) async fn set_discovery(
        &self,
        status: DiscoveryStatus,
        model: Option<&str>,
    ) -> Result<(), AppError> {
        self.query("UPDATE library:main SET discoveryStatus=$status; IF $model!=NULL { UPDATE library:main SET activeModelVersion=$model; };",json!({"status":status,"model":model})).await?;
        Ok(())
    }
    pub async fn nearest(
        &self,
        input: &SearchVector,
        filter: &SearchFilter,
        limit: usize,
    ) -> Result<Vec<SearchHit>, AppError> {
        Ok(self
            .nearest_many(std::slice::from_ref(input), filter, limit)
            .await?
            .into_iter()
            .map(|v| v.hit)
            .collect())
    }
    /// Stream target pages once and retain the winning source passage for each hit.
    pub(crate) async fn nearest_many(
        &self,
        inputs: &[SearchVector],
        filter: &SearchFilter,
        limit: usize,
    ) -> Result<Vec<QuerySearchHit>, AppError> {
        if inputs.is_empty() || inputs.len() > 64 || !(1..=10).contains(&limit) {
            return Err(AppError::invalid_question());
        }
        let version = inputs[0].model_version.clone();
        let mut normalized = vec![];
        for input in inputs {
            if input.model_version != version {
                return Err(AppError::invalid_question());
            }
            normalized.push(
                SearchVector::new(version.clone(), input.values.clone(), 384)
                    .map_err(|_| AppError::invalid_question())?,
            );
        }
        let exists = self
            .query_value(
                "RETURN record::exists(type::record('embedding_model',$key));",
                json!({"key":version}),
            )
            .await?;
        if exists != true {
            return Ok(vec![]);
        }
        let normalized = std::sync::Arc::new(normalized);
        tokio::time::timeout(STORAGE_TIMEOUT,async{
            let mut cursor:Option<String>=None;let mut hits=vec![];
            loop{
                let sql=format!("SELECT key,sourceKey,inputHash,data FROM embedding WHERE {RETRIEVAL_FILTER} ORDER BY key ASC LIMIT 512;");
                let value=self.query_value(&sql,json!({"model":version,"kind":filter.kind,"exclude":filter.exclude_topic,"cursor":cursor,"visible":filter.current_and_pinned_only})).await?;
                let rows=value.as_array().ok_or_else(storage_error)?;let mut records=vec![];
                for row in rows{let record:VectorRecord=serde_json::from_value(row["data"].clone()).map_err(|_|storage_error())?;if record.source_kind!=filter.kind{return Err(storage_error());}records.push((record,row["sourceKey"].as_str().ok_or_else(storage_error)?.to_owned(),row["inputHash"].as_str().ok_or_else(storage_error)?.to_owned()));}
                let inputs=normalized.clone();let model=version.clone();
                let page=tokio::task::spawn_blocking(move||{
                    let mut out=vec![];
                    for(record,key,hash)in records{
                        let target=SearchVector::new(model.clone(),record.values,384).map_err(|_|storage_error())?;
                        let(query_index,score)=inputs.iter().enumerate().map(|(index,input)|{let score=input.values.iter().zip(&target.values).map(|(a,b)|*a as f64 * *b as f64).sum::<f64>().clamp(-1.,1.)as f32;(index,score)}).max_by(|a,b|a.1.total_cmp(&b.1).then_with(||b.0.cmp(&a.0))).ok_or_else(storage_error)?;
                        out.push(QuerySearchHit{query_index,hit:SearchHit{source_key:key,topic_id:record.topic_id,answer_version_id:record.answer_version_id,location:record.location,score,model_version:model.clone(),input_hash:hash,text:record.text}});
                    }
                    out.sort_by(hit_order);out.truncate(limit);Ok::<_,AppError>(out)
                }).await.map_err(|_|storage_error())??;
                hits.extend(page);hits.sort_by(hit_order);hits.truncate(limit);
                if rows.len()<512{break;}cursor=Some(rows.last().and_then(|r|r["key"].as_str()).ok_or_else(storage_error)?.into());
            }
            Ok(hits)
        }).await.map_err(|_|AppError::new(ErrorCode::StorageBusy))?
    }
    pub(crate) async fn source_vectors(
        &self,
        source: &EmbeddingSource,
        model: &str,
    ) -> Result<Vec<Vec<f32>>, AppError> {
        let value=self.query_value("SELECT VALUE data.values FROM embedding WHERE modelVersion=$model AND sourceKey=$source AND inputHash=$hash ORDER BY partIndex ASC LIMIT 64;",json!({"model":model,"source":source.key,"hash":source.input_hash})).await?;
        serde_json::from_value(value).map_err(|_| storage_error())
    }
    pub(crate) async fn keyword_hits(
        &self,
        model: &str,
        filter: &SearchFilter,
        terms: &[String],
        limit: usize,
    ) -> Result<Vec<SearchHit>, AppError> {
        if terms.is_empty() {
            return Ok(vec![]);
        }
        if terms.len() > 12 || !(1..=10).contains(&limit) {
            return Err(AppError::invalid_question());
        }
        let mut bindings = json!({"model":model,"kind":filter.kind,"exclude":filter.exclude_topic,"cursor":null,"visible":filter.current_and_pinned_only,"limit":limit});
        let mut expressions = vec![];
        for (index, term) in terms.iter().enumerate() {
            let name = format!("term{index}");
            bindings[&name] = json!(term);
            expressions.push(format!(
                "(IF string::contains(string::lowercase(data.text),${name}) {{ 1 }} ELSE {{ 0 }})"
            ));
        }
        bindings["minimum"] = json!(terms.len().min(2));
        let count = expressions.join("+");
        let sql = format!(
            "SELECT key,sourceKey,inputHash,data,({count}) AS keywordRank FROM embedding WHERE {RETRIEVAL_FILTER} AND ({count})>=$minimum ORDER BY keywordRank DESC,key ASC LIMIT $limit;"
        );
        let value = self.query_value(&sql, bindings).await?;
        let rows = value.as_array().ok_or_else(storage_error)?;
        let mut hits = vec![];
        for row in rows {
            let data: VectorRecord =
                serde_json::from_value(row["data"].clone()).map_err(|_| storage_error())?;
            hits.push(SearchHit {
                source_key: row["sourceKey"].as_str().ok_or_else(storage_error)?.into(),
                topic_id: data.topic_id,
                answer_version_id: data.answer_version_id,
                location: data.location,
                score: 0.,
                model_version: model.into(),
                input_hash: row["inputHash"].as_str().ok_or_else(storage_error)?.into(),
                text: data.text,
            });
        }
        Ok(hits)
    }
    pub(crate) async fn discovery_model_ready(&self, model: &str) -> Result<bool, AppError> {
        let value=self.query_value("RETURN library:main.activeModelVersion=$model AND library:main.discoveryStatus IN ['ready','indexing'];",json!({"model":model})).await?;
        Ok(value == true)
    }
}
fn hit_order(a: &QuerySearchHit, b: &QuerySearchHit) -> std::cmp::Ordering {
    b.hit
        .score
        .total_cmp(&a.hit.score)
        .then_with(|| a.hit.source_key.cmp(&b.hit.source_key))
        .then_with(|| {
            a.hit
                .location
                .as_ref()
                .map(|v| (&v.paragraph_id, v.section_index, v.part_index))
                .cmp(
                    &b.hit
                        .location
                        .as_ref()
                        .map(|v| (&v.paragraph_id, v.section_index, v.part_index)),
                )
        })
}

fn state_key(source: &EmbeddingSource, model: &str) -> String {
    records::canonical_hash(&json!([model, source.key, source.input_hash]))
}
