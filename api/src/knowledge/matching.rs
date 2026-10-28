//! Local candidate discovery; this service has no paid-provider capability.
use crate::{
    contracts::{library::*, utf16_len},
    embeddings::*,
    error::{AppError, ErrorCode},
    storage::{KnowledgeStore, records::ResolvedQuestion},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Debug, Clone)]
pub struct MatchSource {
    pub topic_id: TopicId,
    pub answer_version_id: Option<AnswerVersionId>,
    pub library_epoch: u64,
}
impl MatchSource {
    pub fn question(topic_id: TopicId, library_epoch: u64) -> Self {
        Self {
            topic_id,
            answer_version_id: None,
            library_epoch,
        }
    }
    pub fn answer(
        topic_id: TopicId,
        answer_version_id: AnswerVersionId,
        library_epoch: u64,
    ) -> Self {
        Self {
            topic_id,
            answer_version_id: Some(answer_version_id),
            library_epoch,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MatchChannel {
    QuestionQuestion,
    QuestionPassage,
    PassagePassage,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchProvenance {
    pub channel: MatchChannel,
    pub vector_score: Option<f32>,
    pub vector_rank: Option<usize>,
    pub keyword_rank: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchedPassage {
    pub location: PassageLocation,
    pub text: String,
    pub citations: Vec<ConnectionCitation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchEndpoint {
    pub topic_id: TopicId,
    pub source_key: String,
    pub input_hash: String,
    pub question: String,
    pub context: Option<ResolvedQuestion>,
    pub passage: Option<MatchedPassage>,
}
impl MatchEndpoint {
    pub fn from_source(
        source: &EmbeddingSource,
        location: Option<&ParagraphLocation>,
    ) -> Result<Self, AppError> {
        let (question, context, passage) = match &source.content {
            SourceContent::Question { resolved } => {
                (resolved.wording.clone(), Some(resolved.clone()), None)
            }
            SourceContent::Passage { content } => {
                let version = source
                    .answer_version_id
                    .clone()
                    .ok_or_else(AppError::invalid_question)?;
                let location = location.ok_or_else(AppError::invalid_question)?;
                let paragraphs = match location.section_index {
                    None => &content.summary,
                    Some(index) => {
                        &content
                            .sections
                            .get(index)
                            .ok_or_else(AppError::invalid_question)?
                            .paragraphs
                    }
                };
                let paragraph = paragraphs
                    .iter()
                    .find(|p| p.id == location.paragraph_id)
                    .ok_or_else(AppError::invalid_question)?;
                let mut citations = vec![];
                for evidence in &paragraph.evidence {
                    let cited = content
                        .sources
                        .iter()
                        .find(|s| s.id == evidence.source_id)
                        .ok_or_else(AppError::invalid_question)?;
                    citations.push(ConnectionCitation {
                        topic_id: source.topic_id.clone(),
                        answer_version_id: version.clone(),
                        paragraph_id: paragraph.id.clone(),
                        source: cited.clone(),
                        quote: evidence.quote.clone(),
                    });
                }
                (
                    content.title.clone(),
                    None,
                    Some(MatchedPassage {
                        location: PassageLocation {
                            answer_version_id: version,
                            paragraph_id: paragraph.id.clone(),
                            section_index: location.section_index,
                        },
                        text: paragraph.text.clone(),
                        citations,
                    }),
                )
            }
        };
        Ok(Self {
            topic_id: source.topic_id.clone(),
            source_key: source.key.clone(),
            input_hash: source.input_hash.clone(),
            question,
            context,
            passage,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchCandidate {
    pub id: ConnectionId,
    pub library_epoch: u64,
    pub model_version: String,
    pub from: MatchEndpoint,
    pub to: MatchEndpoint,
    pub provenance: Vec<MatchProvenance>,
    pub order_rank: f64,
}
impl MatchCandidate {
    pub fn questions(
        a: &EmbeddingSource,
        b: &EmbeddingSource,
        model: &str,
        score: f32,
    ) -> Result<Self, AppError> {
        Self::from_sources(
            a,
            None,
            b,
            None,
            MatchChannel::QuestionQuestion,
            model,
            score,
        )
    }
    pub fn from_sources(
        a: &EmbeddingSource,
        al: Option<&ParagraphLocation>,
        b: &EmbeddingSource,
        bl: Option<&ParagraphLocation>,
        channel: MatchChannel,
        model: &str,
        score: f32,
    ) -> Result<Self, AppError> {
        if a.topic_id == b.topic_id
            || a.library_epoch != b.library_epoch
            || !score.is_finite()
            || !(-1.0..=1.0).contains(&score)
            || model.is_empty()
            || utf16_len(model) > 200
        {
            return Err(AppError::invalid_question());
        }
        let mut from = MatchEndpoint::from_source(a, al)?;
        let mut to = MatchEndpoint::from_source(b, bl)?;
        if (&from.source_key, &from.input_hash) > (&to.source_key, &to.input_hash) {
            std::mem::swap(&mut from, &mut to);
        }
        let id = ConnectionId::new(crate::storage::records::canonical_hash(
            &serde_json::json!({"model":model,"from":[from.source_key,from.input_hash],"to":[to.source_key,to.input_hash]}),
        ))?;
        Ok(Self {
            id,
            library_epoch: a.library_epoch,
            model_version: model.into(),
            from,
            to,
            provenance: vec![MatchProvenance {
                channel,
                vector_score: Some(score),
                vector_rank: Some(1),
                keyword_rank: None,
            }],
            order_rank: 1.0 / 61.0,
        })
    }
    pub fn channel(&self) -> MatchChannel {
        match (self.from.passage.is_some(), self.to.passage.is_some()) {
            (false, false) => MatchChannel::QuestionQuestion,
            (true, true) => MatchChannel::PassagePassage,
            _ => MatchChannel::QuestionPassage,
        }
    }
    pub fn view(&self) -> ConnectionView {
        let mut a = self.from.topic_id.clone();
        let mut b = self.to.topic_id.clone();
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        let (kind, reason) = match self.channel() {
            MatchChannel::PassagePassage => (
                ConnectionKind::OverlappingAnswer,
                "These saved passages may overlap.",
            ),
            MatchChannel::QuestionQuestion => (
                ConnectionKind::SimilarQuestion,
                "These saved questions may be similar.",
            ),
            MatchChannel::QuestionPassage => (
                ConnectionKind::SimilarQuestion,
                "A saved explanation may help with this question.",
            ),
        };
        let passages = [&self.from, &self.to]
            .into_iter()
            .filter_map(|e| e.passage.as_ref().map(|p| p.location.clone()))
            .collect();
        let citations = [&self.from, &self.to]
            .into_iter()
            .filter_map(|e| e.passage.as_ref())
            .flat_map(|p| p.citations.clone())
            .collect::<Vec<_>>();
        ConnectionView {
            from_question: if a == self.from.topic_id {
                self.from.question.clone()
            } else {
                self.to.question.clone()
            },
            to_question: if b == self.to.topic_id {
                self.to.question.clone()
            } else {
                self.from.question.clone()
            },
            id: self.id.clone(),
            from_topic_id: a,
            to_topic_id: b,
            kind,
            status: ConnectionStatus::Candidate,
            reason: reason.into(),
            method: "local similarity".into(),
            model_version: Some(self.model_version.clone()),
            score: self
                .provenance
                .iter()
                .filter_map(|p| p.vector_score)
                .max_by(f32::total_cmp),
            passages,
            evidence: citations
                .iter()
                .map(|c| crate::contracts::EvidenceRef {
                    source_id: c.source.id.clone(),
                    quote: c.quote.clone(),
                })
                .collect(),
            citation_sources: citations,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphBounds {
    pub hops: u8,
    pub nodes: usize,
    pub edges: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: TopicId,
    pub question: String,
    pub current_answer_version_id: Option<AnswerVersionId>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNeighborhood {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<ConnectionView>,
    pub truncated: bool,
    pub bounds: GraphBounds,
}
#[derive(Debug, Default)]
pub struct MatchSummary {
    pub candidates: Vec<MatchCandidate>,
    pub links: usize,
    pub skipped: bool,
}
pub struct MatchingService {
    pub(super) store: Arc<KnowledgeStore>,
    pub(super) embedder: Arc<dyn Embedder>,
}
impl MatchingService {
    pub fn new(store: Arc<KnowledgeStore>, embedder: Arc<dyn Embedder>) -> Self {
        Self { store, embedder }
    }
}

impl MatchingService {
    pub async fn discover(&self, source: &MatchSource) -> Result<MatchSummary, AppError> {
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.discover_inner(source),
        )
        .await
        .map_err(|_| AppError::new(ErrorCode::StorageBusy))?
    }
    async fn discover_inner(&self, request: &MatchSource) -> Result<MatchSummary, AppError> {
        let manifest = self.embedder.manifest();
        let model = manifest.version();
        if !self.store.discovery_model_ready(&model).await? {
            return Ok(MatchSummary {
                skipped: true,
                ..MatchSummary::default()
            });
        }
        let source = self
            .store
            .embedding_source(
                &request.topic_id,
                request.answer_version_id.as_ref(),
                request.library_epoch,
            )
            .await?;
        let prepared = source.prepare(manifest)?;
        if !self.store.embedding_completed(&source, &model).await? {
            let vectors = self.infer(&prepared.inputs).await?;
            self.store.index_source(&source, manifest, &vectors).await?;
        }
        let vectors = self.store.source_vectors(&source, &model).await?;
        if vectors.len() != prepared.inputs.len() {
            return Err(AppError::new(ErrorCode::StorageUnavailable));
        }
        if vectors.is_empty() {
            return Ok(MatchSummary::default());
        }
        let vectors = vectors
            .into_iter()
            .map(|v| {
                SearchVector::new(model.clone(), v, 384)
                    .map_err(|_| AppError::new(ErrorCode::InternalFailure))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut candidates = std::collections::BTreeMap::<ConnectionId, MatchCandidate>::new();
        let channels = if source.kind() == SourceKind::Question {
            vec![
                (MatchChannel::QuestionQuestion, SearchFilter::questions()),
                (MatchChannel::QuestionPassage, SearchFilter::passages()),
            ]
        } else {
            vec![
                (MatchChannel::PassagePassage, SearchFilter::passages()),
                (MatchChannel::QuestionPassage, SearchFilter::questions()),
            ]
        };
        let query_text = match &source.content {
            SourceContent::Question { resolved } => resolved.wording.as_str(),
            SourceContent::Passage { content } => content.title.as_str(),
        };
        let terms = lexical_terms(query_text);
        for (channel, mut filter) in channels {
            filter.exclude_topic = Some(source.topic_id.clone());
            let hits = self.store.nearest_many(&vectors, &filter, 10).await?;
            let mut channel_candidates =
                std::collections::BTreeMap::<ConnectionId, MatchCandidate>::new();
            for (rank, found) in hits.into_iter().enumerate() {
                // Per-channel retrieval floors gate suggestions, never equivalence acceptance.
                let floor = match channel {
                    MatchChannel::QuestionQuestion => 0.65,
                    MatchChannel::QuestionPassage => 0.65,
                    MatchChannel::PassagePassage => 0.72,
                };
                if found.hit.score < floor {
                    continue;
                }
                let target = self
                    .store
                    .embedding_source(
                        &found.hit.topic_id,
                        found.hit.answer_version_id.as_ref(),
                        request.library_epoch,
                    )
                    .await?;
                let source_location = prepared
                    .locations
                    .get(found.query_index)
                    .and_then(|v| v.as_ref());
                let mut candidate = MatchCandidate::from_sources(
                    &source,
                    source_location,
                    &target,
                    found.hit.location.as_ref(),
                    channel,
                    &model,
                    found.hit.score,
                )?;
                candidate.provenance[0].vector_rank = Some(rank + 1);
                candidate.order_rank = 1.0 / (60.0 + rank as f64 + 1.0);
                match channel_candidates.get(&candidate.id) {
                    Some(previous) if previous.order_rank >= candidate.order_rank => {}
                    _ => {
                        channel_candidates.insert(candidate.id.clone(), candidate);
                    }
                }
            }
            let keywords = self.store.keyword_hits(&model, &filter, &terms, 10).await?;
            for (rank, hit) in keywords.into_iter().enumerate() {
                let target = self
                    .store
                    .embedding_source(
                        &hit.topic_id,
                        hit.answer_version_id.as_ref(),
                        request.library_epoch,
                    )
                    .await?;
                let source_location = prepared.locations.first().and_then(|v| v.as_ref());
                let mut candidate = MatchCandidate::from_sources(
                    &source,
                    source_location,
                    &target,
                    hit.location.as_ref(),
                    channel,
                    &model,
                    0.,
                )?;
                if let Some(previous) = channel_candidates.get_mut(&candidate.id) {
                    if previous.provenance[0].keyword_rank.is_none() {
                        previous.provenance[0].keyword_rank = Some(rank + 1);
                        previous.order_rank += 1.0 / (60.0 + rank as f64 + 1.0);
                    }
                } else {
                    candidate.provenance[0].vector_score = None;
                    candidate.provenance[0].vector_rank = None;
                    candidate.provenance[0].keyword_rank = Some(rank + 1);
                    candidate.order_rank = 1.0 / (60.0 + rank as f64 + 1.0);
                    channel_candidates.insert(candidate.id.clone(), candidate);
                }
            }
            let mut ranked = channel_candidates.into_values().collect::<Vec<_>>();
            ranked.sort_by(candidate_order);
            ranked.truncate(10);
            for candidate in ranked {
                candidates.insert(candidate.id.clone(), candidate);
            }
        }
        let mut ranked = candidates.into_values().collect::<Vec<_>>();
        ranked.sort_by(candidate_order);
        ranked.truncate(5);
        let stored = self.store.save_candidates(&ranked).await?;
        let links = stored
            .iter()
            .filter(|v| v.status == ConnectionStatus::Accepted)
            .count();
        let eligible = stored
            .iter()
            .filter(|v| v.status == ConnectionStatus::Candidate)
            .map(|v| v.id.clone())
            .collect::<std::collections::HashSet<_>>();
        ranked.retain(|c| eligible.contains(&c.id));
        Ok(MatchSummary {
            candidates: ranked,
            links,
            skipped: false,
        })
    }
    async fn infer(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, AppError> {
        let mut vectors = vec![];
        for batch in inputs.chunks(32) {
            let embedder = self.embedder.clone();
            let batch = batch.to_vec();
            let mut result = tokio::task::spawn_blocking(move || embedder.embed(&batch))
                .await
                .map_err(|_| AppError::new(ErrorCode::InternalFailure))?
                .map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
            vectors.append(&mut result);
        }
        Ok(vectors)
    }
}
fn candidate_order(a: &MatchCandidate, b: &MatchCandidate) -> std::cmp::Ordering {
    b.order_rank
        .total_cmp(&a.order_rank)
        .then_with(|| a.id.cmp(&b.id))
}
fn lexical_terms(text: &str) -> Vec<String> {
    use unicode_normalization::UnicodeNormalization;
    const STOP: &[&str] = &[
        "how", "what", "why", "when", "where", "which", "does", "the", "are", "can", "for", "with",
        "from", "that", "this", "into", "and", "not",
    ];
    let normalized = text.nfc().collect::<String>().to_lowercase();
    let mut terms = normalized
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.chars().count() >= 3 && !STOP.contains(s))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    terms.sort_by_key(|t| std::cmp::Reverse(t.len()));
    terms.truncate(12);
    terms
}
