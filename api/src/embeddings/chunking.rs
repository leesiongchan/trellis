use super::*;
use crate::{
    contracts::TopicContent,
    error::{AppError, ErrorCode},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphLocation {
    pub paragraph_id: String,
    pub section_index: Option<usize>,
    pub part_index: usize,
    pub token_start: usize,
    pub token_end: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerChunk {
    pub location: ParagraphLocation,
    pub text: String,
    pub input: String,
    pub passage_tokens: usize,
    pub input_tokens: usize,
}
#[derive(Debug, Clone)]
pub struct ChunkSet {
    pub chunks: Vec<AnswerChunk>,
    pub partial_coverage: bool,
}
pub fn chunk_answer(
    content: &TopicContent,
    manifest: &ModelManifest,
) -> Result<ChunkSet, AppError> {
    fn error(_: EmbeddingError) -> AppError {
        AppError::new(ErrorCode::InternalFailure)
    }
    if manifest.max_tokens < 352 {
        return Err(AppError::new(ErrorCode::InternalFailure));
    }
    let paragraphs = content.summary.iter().map(|p| (None, "Summary", p)).chain(
        content.sections.iter().enumerate().flat_map(|(index, s)| {
            s.paragraphs
                .iter()
                .map(move |p| (Some(index), s.heading.as_str(), p))
        }),
    );
    let mut chunks = vec![];
    let mut partial = false;
    'paragraphs: for (section, heading, p) in paragraphs {
        let encoded = manifest.tokens(&p.text, false).map_err(error)?;
        let offsets = encoded.get_offsets();
        for (start, part) in offsets.chunks(350).enumerate() {
            if chunks.len() == 64 {
                partial = true;
                break 'paragraphs;
            }
            let Some((first, _)) = part.first() else {
                continue;
            };
            let Some((_, last)) = part.last() else {
                continue;
            };
            let text = p
                .text
                .get(*first..*last)
                .ok_or_else(|| AppError::new(ErrorCode::InternalFailure))?
                .to_owned();
            let context = format!("Topic: {}\nHeading: {heading}", content.title);
            let mut context_budget = 128.min(manifest.max_tokens - 352);
            let (mut input, mut input_tokens);
            loop {
                let prefix = manifest
                    .truncate_input(&context, context_budget)
                    .map_err(error)?;
                input = if prefix.is_empty() {
                    text.clone()
                } else {
                    format!("{prefix}\n{text}")
                };
                input_tokens = manifest.token_count(&input, true).map_err(error)?;
                if input_tokens <= manifest.max_tokens {
                    break;
                }
                if context_budget == 0 {
                    return Err(AppError::new(ErrorCode::InternalFailure));
                }
                context_budget -= 1;
            }
            chunks.push(AnswerChunk {
                location: ParagraphLocation {
                    paragraph_id: p.id.clone(),
                    section_index: section,
                    part_index: start,
                    token_start: start * 350,
                    token_end: start * 350 + part.len(),
                },
                text,
                input,
                passage_tokens: part.len(),
                input_tokens,
            });
        }
    }
    Ok(ChunkSet {
        chunks,
        partial_coverage: partial,
    })
}
