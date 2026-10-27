//! Evidence-backed research orchestration.
pub mod context;
pub mod grounding;
pub mod prompts;
use crate::{
    contracts::*,
    error::{AppError, ErrorCode},
    providers::{GeneratedResearch, ResearchProvider},
};
use context::JobContext;
pub async fn research_topic(
    input: &ResearchRequest,
    ctx: &JobContext,
    provider: &dyn ResearchProvider,
) -> Result<ResearchResult, AppError> {
    Ok(research_topic_recorded(input, ctx, provider).await?.result)
}
pub async fn research_topic_recorded(
    input: &ResearchRequest,
    ctx: &JobContext,
    provider: &dyn ResearchProvider,
) -> Result<GeneratedResearch, AppError> {
    ctx.check()?;
    let context = input
        .ancestors
        .iter()
        .map(|a| a.title.as_str())
        .collect::<Vec<_>>()
        .join(" → ");
    let mut parts = vec![input.question.clone()];
    if let Some(clarification) = &input.clarification {
        parts.push(clarification.clone());
    }
    if !context.is_empty() {
        parts.push(format!("Context: {context}"));
    }
    let sources = provider.search(&parts.join(" — "), ctx).await?;
    ctx.check()?;
    if sources.is_empty() {
        return Ok(ResearchResult::InsufficientEvidence{message:"I could not retrieve enough reliable source text for this topic. Try a more specific question.".into()}.into());
    }
    let generated = provider.generate_recorded(input, &sources, ctx).await?;
    ctx.check()?;
    let result = ResearchResult::from_json(
        serde_json::to_value(generated.result)
            .map_err(|_| AppError::new(ErrorCode::InvalidExplanation))?,
    )
    .map_err(|_| AppError::new(ErrorCode::InvalidExplanation))?;
    let ResearchResult::Topic { content } = &result else {
        return Ok(GeneratedResearch {
            result,
            generation: generated.generation,
        });
    };
    let timer = ctx
        .telemetry
        .as_ref()
        .map(|t| t.start(crate::telemetry::Stage::Grounding, &ctx.job_id));
    let evidence = grounding::validate_evidence(content, &sources);
    if let Some(timer) = timer {
        timer.finish(&evidence);
    }
    if evidence.is_err() {
        return Ok(ResearchResult::InsufficientEvidence {
            message:
                "The explanation did not pass its source check. Try again or narrow the topic."
                    .into(),
        }
        .into());
    }
    ctx.check()?;
    let verdict = provider.verify(content, &sources, ctx).await?;
    ctx.check()?;
    if !verdict.unsupported_paragraph_ids.is_empty()
        || !verdict.unsupported_branch_indexes.is_empty()
    {
        return Ok(ResearchResult::InsufficientEvidence{message:"Some claims were not sufficiently supported by the retrieved sources. Please retry or make the question more specific.".into()}.into());
    }
    Ok(GeneratedResearch {
        result,
        generation: generated.generation,
    })
}
