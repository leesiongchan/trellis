//! Validated wire contracts shared with the browser.
pub mod legacy;
pub mod library;
mod validation;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use validation::{ecma_whitespace, public_source_url, truncate_utf16, utf16_len};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchRequest {
    pub exploration_id: String,
    pub node_id: String,
    pub question: String,
    pub clarification: Option<String>,
    pub ancestors: Vec<Ancestor>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ancestor {
    pub title: String,
    pub summary: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub title: String,
    pub url: String,
    pub publisher: Option<String>,
    pub retrieved_at: String,
}
#[derive(Debug, Clone)]
pub struct RetrievedSource {
    pub source: Source,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRef {
    pub source_id: String,
    pub quote: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    pub id: String,
    pub text: String,
    pub evidence: Vec<EvidenceRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub heading: String,
    pub paragraphs: Vec<Paragraph>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub title: String,
    pub question: String,
    pub relationship: String,
    pub reason: Paragraph,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicContent {
    pub title: String,
    pub summary: Vec<Paragraph>,
    pub sections: Vec<Section>,
    pub analogy: Option<String>,
    pub limitations: Option<String>,
    pub sources: Vec<Source>,
    pub branches: Vec<Branch>,
    pub researched_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ResearchResult {
    #[serde(rename = "topic")]
    Topic { content: TopicContent },
    #[serde(rename = "clarification")]
    Clarification { question: String },
    #[serde(rename = "insufficient-evidence")]
    InsufficientEvidence { message: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingVerdict {
    pub unsupported_paragraph_ids: Vec<String>,
    pub unsupported_branch_indexes: Vec<usize>,
}
impl ResearchRequest {
    pub fn from_json(value: Value) -> Result<Self, AppError> {
        validation::required(&value, &["clarification"])?;
        let mut input: Self =
            serde_json::from_value(value).map_err(|_| AppError::invalid_question())?;
        validation::request(&mut input)?;
        Ok(input)
    }
}
impl ResearchResult {
    pub fn from_json(value: Value) -> Result<Self, AppError> {
        if value["kind"] == "topic" {
            validation::required(&value["content"], &["analogy", "limitations"])?;
            if let Some(sources) = value["content"]["sources"].as_array() {
                for source in sources {
                    validation::required(source, &["publisher"])?;
                }
            }
        }
        let mut result: Self =
            serde_json::from_value(value).map_err(|_| AppError::invalid_question())?;
        match &mut result {
            Self::Topic { content } => validation::content(content)?,
            Self::Clarification { question } => validation::question(question)?,
            Self::InsufficientEvidence { message } => validation::length(message, 1, 2000)?,
        }
        Ok(result)
    }
}
