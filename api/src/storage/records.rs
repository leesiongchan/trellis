//! Repository records and versioned, conservative exact identity.
use crate::{
    contracts::{Ancestor, ecma_whitespace},
    error::AppError,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionKey(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedQuestion {
    pub wording: String,
    pub clarification: Option<String>,
    pub locale: String,
    pub ancestors: Vec<Ancestor>,
    pub interpretation: String,
    pub encoding_version: u8,
}
impl ResolvedQuestion {
    pub fn new(
        wording: &str,
        locale: &str,
        clarification: Option<String>,
        ancestors: Vec<Ancestor>,
        interpretation: &str,
    ) -> Result<Self, AppError> {
        let mut request = crate::contracts::ResearchRequest {
            exploration_id: "context".into(),
            node_id: "context".into(),
            question: wording.into(),
            clarification,
            ancestors,
        };
        request = crate::contracts::ResearchRequest::from_json(
            serde_json::to_value(request).map_err(|_| AppError::invalid_question())?,
        )?;
        if locale.is_empty() || locale.len() > 35 || interpretation.len() > 500 {
            return Err(AppError::invalid_question());
        }
        Ok(Self {
            wording: request.question,
            clarification: request.clarification,
            locale: locale.into(),
            ancestors: request.ancestors,
            interpretation: interpretation.into(),
            encoding_version: 1,
        })
    }
}
fn normalized(v: &str) -> String {
    v.nfc()
        .collect::<String>()
        .split(ecma_whitespace)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn question_key(input: &ResolvedQuestion) -> QuestionKey {
    let mut value = serde_json::json!({"mode":"research","wording":normalized(&input.wording),"clarification":input.clarification.as_ref().map(|v|normalized(v)),"locale":input.locale,"ancestors":input.ancestors,"interpretation":input.interpretation,"encodingVersion":input.encoding_version});
    // Ancestor text is the stored bounded research context; its exact contents matter.
    value["keyVersion"] = Value::from(2);
    QuestionKey(canonical_hash(&value))
}
pub fn canonical_hash(input: &Value) -> String {
    fn encode(v: &Value, out: &mut String) {
        match v {
            Value::Object(m) => {
                out.push('{');
                let mut keys = m.keys().collect::<Vec<_>>();
                keys.sort();
                for (i, k) in keys.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&Value::String((*k).clone()).to_string());
                    out.push(':');
                    encode(&m[*k], out);
                }
                out.push('}');
            }
            Value::Array(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    encode(v, out);
                }
                out.push(']');
            }
            _ => out.push_str(&v.to_string()),
        }
    }
    let mut encoded = String::new();
    encode(input, &mut encoded);
    format!("{:x}", Sha256::digest(encoded.as_bytes()))
}
pub(crate) fn now() -> Result<String, AppError> {
    time::OffsetDateTime::now_utc()
        .format(
            &time::format_description::parse_borrowed::<2>(
                "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:9]Z",
            )
            .map_err(|_| AppError::new(crate::error::ErrorCode::InternalFailure))?,
        )
        .map_err(|_| AppError::new(crate::error::ErrorCode::InternalFailure))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TopicRecord {
    pub id: crate::contracts::library::TopicId,
    pub question: String,
    pub current_answer_version_id: Option<crate::contracts::library::AnswerVersionId>,
    pub created_at: String,
    pub updated_at: String,
}
