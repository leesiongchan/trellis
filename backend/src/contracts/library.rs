//! Protocol 2. Public IDs are opaque strings, never database record identifiers.
use super::{Ancestor, Paragraph, TopicContent, ecma_whitespace, utf16_len};
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

macro_rules! ids {
 ($($name:ident),+) => {$ (
 #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
 #[serde(try_from="String", into="String")]
 pub struct $name(String);
 impl $name {
  pub fn new(value: impl Into<String>) -> Result<Self, AppError> {
   let value=value.into();
   if value.is_empty() || value.len()>200 || !value.bytes().all(|b|b.is_ascii_alphanumeric() || b==b'_' || b==b'-') {return Err(AppError::invalid_question());}
   Ok(Self(value))
  }
  pub fn as_str(&self)->&str {&self.0}
  pub fn generate()->Self {Self(uuid::Uuid::new_v4().to_string())}
 }
 impl TryFrom<String> for $name {type Error=AppError;fn try_from(value:String)->Result<Self,AppError>{Self::new(value)}}
 impl From<$name> for String {fn from(value:$name)->Self{value.0}}
 impl std::fmt::Display for $name {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{self.0.fmt(f)}}
 )+};
}
ids!(
    OperationId,
    LibraryId,
    ExplorationId,
    AppearanceId,
    TopicId,
    AnswerVersionId,
    ConnectionId,
    ImportId,
    AttemptId,
    VariantId
);

pub const PROTOCOL_VERSION: u8 = 2;
pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub trait ValidatedInput {
    fn validate(&mut self) -> Result<(), AppError>;
    fn required_fields() -> &'static [&'static str] {
        &[]
    }
}
fn required(value: &Value, names: &[&str]) -> Result<(), AppError> {
    if names.iter().all(|n| value.get(n).is_some()) {
        Ok(())
    } else {
        Err(AppError::invalid_question())
    }
}
pub fn question(value: &mut String) -> Result<(), AppError> {
    *value = value.trim_matches(ecma_whitespace).into();
    if (1..=500).contains(&utf16_len(value)) {
        Ok(())
    } else {
        Err(AppError::invalid_question())
    }
}
fn locale(value: &str) -> Result<(), AppError> {
    if !value.is_empty()
        && value.len() <= 35
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        Ok(())
    } else {
        Err(AppError::invalid_question())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command<T> {
    pub operation_id: OperationId,
    pub library_epoch: u64,
    pub expected_revision: Option<u64>,
    pub payload: T,
}
impl<T: ValidatedInput + serde::de::DeserializeOwned> Command<T> {
    pub fn from_json(value: Value) -> Result<Self, AppError> {
        required(&value, &["expectedRevision"])?;
        required(&value["payload"], T::required_fields())?;
        let mut result: Self =
            serde_json::from_value(value).map_err(|_| AppError::invalid_question())?;
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&mut self) -> Result<(), AppError> {
        if self.library_epoch == 0
            || self.library_epoch > MAX_SAFE_INTEGER
            || self.expected_revision.is_some_and(|v| v > MAX_SAFE_INTEGER)
        {
            return Err(AppError::invalid_question());
        }
        self.payload.validate()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExploration {
    pub question: String,
    pub locale: String,
}
impl ValidatedInput for CreateExploration {
    fn validate(&mut self) -> Result<(), AppError> {
        question(&mut self.question)?;
        locale(&self.locale)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddBranch {
    pub parent_appearance_id: AppearanceId,
    pub question: String,
}
impl ValidatedInput for AddBranch {
    fn validate(&mut self) -> Result<(), AppError> {
        question(&mut self.question)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchInput {
    pub appearance_id: AppearanceId,
    pub refresh: bool,
    pub clarification: Option<String>,
}
impl ValidatedInput for ResearchInput {
    fn validate(&mut self) -> Result<(), AppError> {
        if let Some(v) = &mut self.clarification {
            question(v)?;
        }
        Ok(())
    }
    fn required_fields() -> &'static [&'static str] {
        &["clarification"]
    }
}
macro_rules! simple_input {($name:ident{$($field:ident:$ty:ty),+})=>{
 #[derive(Debug,Clone,Serialize,Deserialize)] #[serde(rename_all="camelCase")] pub struct $name{$(pub $field:$ty),+}
 impl ValidatedInput for $name{fn validate(&mut self)->Result<(),AppError>{Ok(())}}
}}
simple_input!(ChooseAnswer {
    appearance_id: AppearanceId,
    answer_version_id: AnswerVersionId
});
simple_input!(DismissConnection {
    connection_id: ConnectionId
});
simple_input!(DeleteExploration {
    exploration_id: ExplorationId
});
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurgeLibrary {
    pub confirmation: String,
}
pub const PURGE_CONFIRMATION: &str = "Delete all server library content; offline copies and retained backups require separate clearing";
impl ValidatedInput for PurgeLibrary {
    fn validate(&mut self) -> Result<(), AppError> {
        if self.confirmation == PURGE_CONFIRMATION {
            Ok(())
        } else {
            Err(AppError::invalid_question())
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveryStatus {
    Ready,
    Unavailable,
    Indexing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub protocol_version: u8,
    pub schema_version: u8,
    pub library_id: LibraryId,
    pub library_epoch: u64,
    pub discovery: DiscoveryStatus,
}
impl LibraryInfo {
    pub fn from_json(value: Value) -> Result<Self, AppError> {
        let v: Self = serde_json::from_value(value).map_err(|_| AppError::invalid_question())?;
        if v.protocol_version != 2
            || v.schema_version != 2
            || v.library_epoch == 0
            || v.library_epoch > MAX_SAFE_INTEGER
        {
            return Err(AppError::invalid_question());
        }
        Ok(v)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplorationSummary {
    pub id: ExplorationId,
    pub original_prompt: String,
    pub root_id: AppearanceId,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceView {
    pub id: AppearanceId,
    pub exploration_id: ExplorationId,
    pub parent_id: Option<AppearanceId>,
    pub topic_id: TopicId,
    pub question: String,
    pub locale: String,
    pub clarification: Option<String>,
    pub ancestors: Vec<Ancestor>,
    pub interpretation: String,
    pub relationship: String,
    #[serde(default)]
    pub teaser: String,
    pub reason: Option<Paragraph>,
    pub reason_sources: Vec<super::Source>,
    pub reason_answer_version_id: Option<AnswerVersionId>,
    pub answer_version_id: Option<AnswerVersionId>,
    pub child_ids: Vec<AppearanceId>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplorationSnapshot {
    pub schema_version: u8,
    pub library_epoch: u64,
    pub exploration: ExplorationSummary,
    pub appearances: Vec<AppearanceView>,
    pub answers: Vec<AnswerVersionView>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnswerOrigin {
    #[default]
    Research,
    Legacy,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerVersionView {
    #[serde(default)]
    pub origin: AnswerOrigin,
    pub id: AnswerVersionId,
    pub topic_id: TopicId,
    pub revision: u64,
    pub content: TopicContent,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicView {
    pub id: TopicId,
    pub question: String,
    pub current_answer_version_id: Option<AnswerVersionId>,
    pub selected_answer: Option<AnswerVersionView>,
    pub versions: Vec<AnswerVersionSummary>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerVersionSummary {
    pub id: AnswerVersionId,
    pub revision: u64,
    pub created_at: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    BranchesTo,
    EquivalentQuestion,
    SimilarQuestion,
    OverlappingAnswer,
    RelatedTopic,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Candidate,
    Accepted,
    Dismissed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassageLocation {
    pub answer_version_id: AnswerVersionId,
    pub paragraph_id: String,
    pub section_index: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCitation {
    pub topic_id: TopicId,
    pub answer_version_id: AnswerVersionId,
    pub paragraph_id: String,
    pub source: super::Source,
    pub quote: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub id: ConnectionId,
    pub from_topic_id: TopicId,
    pub to_topic_id: TopicId,
    pub kind: ConnectionKind,
    pub status: ConnectionStatus,
    pub reason: String,
    pub method: String,
    pub model_version: Option<String>,
    pub score: Option<f32>,
    pub passages: Vec<PassageLocation>,
    pub evidence: Vec<super::EvidenceRef>,
    #[serde(default)]
    pub citation_sources: Vec<ConnectionCitation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OperationState {
    Pending,
    Completed,
    Failed,
    Interrupted,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum OperationResult {
    Exploration {
        exploration_id: ExplorationId,
        appearance_id: AppearanceId,
        revision: u64,
    },
    Research {
        exploration_id: ExplorationId,
        appearance_id: AppearanceId,
        answer_version_id: Option<AnswerVersionId>,
        revision: u64,
        outcome: Box<super::ResearchResult>,
    },
    Connection {
        connection_id: ConnectionId,
    },
    Deletion {
        exploration_id: ExplorationId,
    },
    Purge {
        library_epoch: u64,
    },
    Import {
        import_id: ImportId,
        exploration_id: ExplorationId,
        revision: u64,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}
impl From<AppError> for OperationError {
    fn from(e: AppError) -> Self {
        Self {
            code: e.code().into(),
            message: e.message().into(),
            retryable: e.retryable(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationView {
    pub id: OperationId,
    pub state: OperationState,
    pub result: Option<OperationResult>,
    pub error: Option<OperationError>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginImport {
    pub legacy_exploration_id: String,
    pub content_digest: String,
    pub expected_bytes: u64,
    pub expected_nodes: u64,
    #[serde(default)]
    pub explicit_restore: bool,
}
impl ValidatedInput for BeginImport {
    fn validate(&mut self) -> Result<(), AppError> {
        self.content_digest.make_ascii_lowercase();
        if self.legacy_exploration_id.is_empty()
            || self.legacy_exploration_id.chars().count() > 200
            || self.content_digest.len() != 64
            || !self.content_digest.bytes().all(|b| b.is_ascii_hexdigit())
            || self.expected_bytes == 0
            || self.expected_bytes > 32 * 1024 * 1024
            || self.expected_nodes == 0
            || self.expected_nodes > 2000
        {
            return Err(crate::error::AppError::new(
                crate::error::ErrorCode::InvalidImport,
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatch {
    pub import_id: ImportId,
    pub batch_index: u64,
    pub records: Vec<Value>,
}
impl ValidatedInput for ImportBatch {
    fn validate(&mut self) -> Result<(), AppError> {
        if self.batch_index > 2000
            || self.records.is_empty()
            || self.records.len() > 2000
            || serde_json::to_vec(&self.records)
                .map_err(|_| AppError::invalid_question())?
                .len()
                > 2 * 1024 * 1024
        {
            return Err(crate::error::AppError::new(
                crate::error::ErrorCode::InvalidImport,
            ));
        }
        Ok(())
    }
}
simple_input!(FinalizeImport {
    import_id: ImportId
});
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImportState {
    Staging,
    Completed,
    Failed,
    Suppressed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportView {
    pub upload_operation_id: OperationId,
    pub exploration_id: Option<ExplorationId>,
    pub id: ImportId,
    pub state: ImportState,
    pub mapping: std::collections::BTreeMap<String, String>,
    pub server_revision: Option<u64>,
    pub error: Option<OperationError>,
}
