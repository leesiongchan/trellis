use super::*;
use crate::research::prompts::{EXPLAIN_PROMPT, VERIFY_PROMPT};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
fn sources_json(sources: &[RetrievedSource]) -> Vec<Value> {
    sources.iter().map(|s|json!({"id":s.source.id,"title":s.source.title,"url":s.source.url,"publisher":s.source.publisher,"retrievedAt":s.source.retrieved_at,"text":s.text})).collect()
}
fn output_json(raw: Value, failure: ErrorCode) -> Result<Value, AppError> {
    let invalid = || AppError::new(failure);
    if raw["status"] != "completed" || raw.get("error").is_some_and(|v| !v.is_null()) {
        return Err(invalid());
    }
    let mut texts = Vec::new();
    for item in raw["output"].as_array().ok_or_else(invalid)? {
        if item["type"] == "message" {
            for content in item["content"].as_array().ok_or_else(invalid)? {
                if content["type"] == "refusal" {
                    return Err(invalid());
                }
                if content["type"] == "output_text" {
                    texts.push(content["text"].as_str().ok_or_else(invalid)?);
                }
            }
        }
    }
    if texts.len() != 1 {
        return Err(invalid());
    }
    serde_json::from_str(texts[0]).map_err(|_| invalid())
}
impl HttpProviders {
    async fn response(
        &self,
        prompt: &str,
        data: Value,
        format: (&str, &str),
        tokens: u32,
        ctx: &JobContext,
        failure: ErrorCode,
    ) -> Result<Value, AppError> {
        let (name, schema) = format;
        let schema: Value =
            serde_json::from_str(schema).map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
        let body = json!({"model":self.config.model,"store":false,"reasoning":{"effort":"low"},"max_output_tokens":tokens,"input":[{"role":"system","content":prompt},{"role":"user","content":data.to_string()}],"text":{"format":{"type":"json_schema","name":name,"strict":true,"schema":schema}}});
        output_json(
            self.post(
                &self.endpoints.openai,
                self.config.openai_key.expose(),
                &body,
                ctx,
                ctx.deadline,
                failure,
            )
            .await?,
            failure,
        )
    }
    pub(super) async fn generate_topic(
        &self,
        input: &ResearchRequest,
        sources: &[RetrievedSource],
        ctx: &JobContext,
    ) -> Result<ResearchResult, AppError> {
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
        let data = json!({"date":&now[..10],"topic":input,"sources":sources_json(sources)});
        let draft = self
            .response(
                EXPLAIN_PROMPT,
                data,
                ("topic_explanation", include_str!("schemas/topic.json")),
                8000,
                ctx,
                ErrorCode::InvalidExplanation,
            )
            .await?;
        let mut result = draft
            .get("result")
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::InvalidExplanation))?;
        if result["kind"] == "topic" {
            add_paragraph_ids(&mut result)?;
            let content = result
                .as_object_mut()
                .ok_or_else(|| AppError::new(ErrorCode::InvalidExplanation))?;
            content.remove("kind");
            content.insert(
                "sources".into(),
                serde_json::to_value(sources.iter().map(|s| &s.source).collect::<Vec<_>>())
                    .map_err(|_| AppError::new(ErrorCode::InternalFailure))?,
            );
            content.insert("researchedAt".into(), json!(now));
            result = json!({"kind":"topic","content":result});
        }
        ResearchResult::from_json(result).map_err(|_| AppError::new(ErrorCode::InvalidExplanation))
    }
    pub(super) async fn verify_grounding(
        &self,
        content: &TopicContent,
        sources: &[RetrievedSource],
        ctx: &JobContext,
    ) -> Result<GroundingVerdict, AppError> {
        let raw = self
            .response(
                VERIFY_PROMPT,
                json!({"draft":content,"sources":sources_json(sources)}),
                ("evidence_check", include_str!("schemas/verdict.json")),
                2000,
                ctx,
                ErrorCode::GroundingFailed,
            )
            .await?;
        serde_json::from_value(raw).map_err(|_| AppError::new(ErrorCode::GroundingFailed))
    }
}
fn add_id(value: &mut Value) -> Result<(), AppError> {
    value
        .as_object_mut()
        .ok_or_else(|| AppError::new(ErrorCode::InvalidExplanation))?
        .insert(
            "id".into(),
            json!(uuid::Uuid::new_v4().simple().to_string()),
        );
    Ok(())
}
fn add_paragraph_ids(draft: &mut Value) -> Result<(), AppError> {
    let invalid = || AppError::new(ErrorCode::InvalidExplanation);
    for p in draft["summary"].as_array_mut().ok_or_else(invalid)? {
        add_id(p)?;
    }
    for section in draft["sections"].as_array_mut().ok_or_else(invalid)? {
        for p in section["paragraphs"].as_array_mut().ok_or_else(invalid)? {
            add_id(p)?;
        }
    }
    for b in draft["branches"].as_array_mut().ok_or_else(invalid)? {
        add_id(&mut b["reason"])?;
    }
    Ok(())
}

impl crate::knowledge::verification::ConnectionVerifier for HttpProviders {
    fn model_id(&self) -> &str {
        &self.config.model
    }
    fn verify<'a>(
        &'a self,
        candidates: &'a [crate::knowledge::matching::MatchCandidate],
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, crate::knowledge::verification::ConnectionVerdict> {
        Box::pin(self.observed(
            crate::telemetry::Stage::ConnectionVerification,
            ctx,
            async move {
                let data = crate::knowledge::verification::comparison_data(candidates)?;
                let raw = self
                    .response(
                        crate::knowledge::verification::CONNECTION_PROMPT,
                        data,
                        (
                            "connection_verdict",
                            include_str!("schemas/connections.json"),
                        ),
                        2500,
                        ctx,
                        ErrorCode::InvalidExplanation,
                    )
                    .await?;
                let verdict = crate::knowledge::verification::ConnectionVerdict::from_json(raw)?;
                verdict.validate(candidates)?;
                Ok(verdict)
            },
        ))
    }
}
