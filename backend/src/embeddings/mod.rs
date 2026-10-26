//! Optional local discovery. Content persistence never depends on this module.
mod chunking;
mod model;
pub use chunking::{AnswerChunk, ChunkSet, ParagraphLocation, chunk_answer};
pub use model::{LocalEmbedder, ModelManifest};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    Query,
    Document,
}
#[derive(Debug, Clone)]
pub struct EmbeddingInput {
    pub text: String,
    pub kind: InputKind,
}
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("The local discovery model is not installed.")]
    MissingModel,
    #[error("The local discovery model failed integrity verification.")]
    CorruptModel,
    #[error("The local inference runtime is unavailable.")]
    RuntimeUnavailable,
    #[error("The embedding input or vector is invalid.")]
    InvalidInput,
}
pub trait Embedder: Send + Sync {
    fn embed(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
    fn manifest(&self) -> &ModelManifest;
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchVector {
    pub model_version: String,
    pub values: Vec<f32>,
}
impl SearchVector {
    pub fn new(
        model_version: String,
        values: Vec<f32>,
        dimensions: usize,
    ) -> Result<Self, EmbeddingError> {
        if values.len() != dimensions
            || dimensions == 0
            || dimensions > 4096
            || values.iter().any(|v| !v.is_finite())
        {
            return Err(EmbeddingError::InvalidInput);
        }
        let norm = values
            .iter()
            .map(|v| (*v as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        if norm <= f64::EPSILON {
            return Err(EmbeddingError::InvalidInput);
        }
        Ok(Self {
            model_version,
            values: values
                .into_iter()
                .map(|v| (v as f64 / norm) as f32)
                .collect(),
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Question,
    Passage,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SourceContent {
    Question {
        resolved: crate::storage::records::ResolvedQuestion,
    },
    Passage {
        content: crate::contracts::TopicContent,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingSource {
    pub key: String,
    pub library_epoch: u64,
    pub topic_id: crate::contracts::library::TopicId,
    pub answer_version_id: Option<crate::contracts::library::AnswerVersionId>,
    pub input_hash: String,
    pub content: SourceContent,
}
pub struct PreparedSource {
    pub inputs: Vec<EmbeddingInput>,
    pub locations: Vec<Option<ParagraphLocation>>,
    pub partial_coverage: bool,
}
impl EmbeddingSource {
    pub fn kind(&self) -> SourceKind {
        match self.content {
            SourceContent::Question { .. } => SourceKind::Question,
            SourceContent::Passage { .. } => SourceKind::Passage,
        }
    }
    pub fn prepare(
        &self,
        manifest: &ModelManifest,
    ) -> Result<PreparedSource, crate::error::AppError> {
        match &self.content {
            SourceContent::Question { resolved } => {
                let mut parts = vec![resolved.wording.clone()];
                if let Some(c) = &resolved.clarification {
                    parts.push(c.clone());
                }
                for a in &resolved.ancestors {
                    parts.push(format!("{}: {}", a.title, a.summary));
                }
                let text = manifest
                    .truncate_input(&parts.join("\n"), manifest.max_tokens.saturating_sub(2))
                    .map_err(|_| {
                        crate::error::AppError::new(crate::error::ErrorCode::InternalFailure)
                    })?;
                Ok(PreparedSource {
                    inputs: vec![EmbeddingInput {
                        text,
                        kind: InputKind::Query,
                    }],
                    locations: vec![None],
                    partial_coverage: false,
                })
            }
            SourceContent::Passage { content } => {
                let chunks = chunk_answer(content, manifest)?;
                let mut inputs = vec![];
                let mut locations = vec![];
                for c in chunks.chunks {
                    inputs.push(EmbeddingInput {
                        text: c.input,
                        kind: InputKind::Document,
                    });
                    locations.push(Some(c.location));
                }
                Ok(PreparedSource {
                    inputs,
                    locations,
                    partial_coverage: chunks.partial_coverage,
                })
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct SearchFilter {
    pub kind: SourceKind,
    pub exclude_topic: Option<crate::contracts::library::TopicId>,
    pub current_and_pinned_only: bool,
}
impl SearchFilter {
    pub fn questions() -> Self {
        Self {
            kind: SourceKind::Question,
            exclude_topic: None,
            current_and_pinned_only: true,
        }
    }
    pub fn passages() -> Self {
        Self {
            kind: SourceKind::Passage,
            ..Self::questions()
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub source_key: String,
    pub topic_id: crate::contracts::library::TopicId,
    pub answer_version_id: Option<crate::contracts::library::AnswerVersionId>,
    pub location: Option<ParagraphLocation>,
    pub score: f32,
    pub model_version: String,
    pub input_hash: String,
    pub text: String,
}
mod worker;
pub use worker::EmbeddingWorker;
