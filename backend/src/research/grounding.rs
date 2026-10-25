use crate::{
    contracts::{RetrievedSource, TopicContent, ecma_whitespace},
    error::{AppError, ErrorCode},
};
use std::collections::{HashMap, HashSet};
fn normalize(value: &str) -> String {
    value
        .split(ecma_whitespace)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn validate_evidence(
    content: &TopicContent,
    sources: &[RetrievedSource],
) -> Result<(), AppError> {
    let invalid = || AppError::new(ErrorCode::InvalidEvidence);
    let known: HashMap<_, _> = sources.iter().map(|s| (s.source.id.as_str(), s)).collect();
    for source in &content.sources {
        if known
            .get(source.id.as_str())
            .is_none_or(|s| s.source.url != source.url)
        {
            return Err(invalid());
        }
    }
    let public_ids: HashSet<_> = content.sources.iter().map(|s| s.id.as_str()).collect();
    for paragraph in content
        .summary
        .iter()
        .chain(content.sections.iter().flat_map(|s| &s.paragraphs))
        .chain(content.branches.iter().map(|b| &b.reason))
    {
        if paragraph.evidence.is_empty() {
            return Err(invalid());
        }
        for evidence in &paragraph.evidence {
            let source = known.get(evidence.source_id.as_str()).ok_or_else(invalid)?;
            let quote = normalize(&evidence.quote);
            if quote.is_empty()
                || !public_ids.contains(evidence.source_id.as_str())
                || !normalize(&source.text).contains(&quote)
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}
