//! Schema-v1 browser snapshots. These are preserved imports, not grounded research.
use super::*;
use crate::error::ErrorCode;
use std::collections::{BTreeMap, HashSet};
pub const MAX_IMPORT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_IMPORT_NODES: usize = 2000;
fn invalid() -> AppError {
    AppError::new(ErrorCode::InvalidImport)
}
fn research_mode() -> String {
    "research".into()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub question: String,
    pub relationship: String,
    pub teaser: String,
    pub status: String,
    pub content: Option<TopicContent>,
    pub child_ids: Vec<String>,
    pub last_error: Option<Value>,
    pub is_refreshing: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyExploration {
    pub schema_version: u8,
    #[serde(default = "research_mode")]
    pub mode: String,
    pub id: String,
    pub original_prompt: String,
    pub root_id: String,
    pub nodes: BTreeMap<String, LegacyNode>,
    pub view: Value,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}
impl LegacyExploration {
    pub fn validate(value: Value) -> Result<Self, AppError> {
        if value.to_string().len() > MAX_IMPORT_BYTES {
            return Err(invalid());
        }
        validation::required(
            &value,
            &[
                "schemaVersion",
                "id",
                "originalPrompt",
                "rootId",
                "nodes",
                "view",
                "revision",
                "createdAt",
                "updatedAt",
            ],
        )
        .map_err(|_| invalid())?;
        let mut parsed: Self = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
        if parsed.schema_version != 1
            || parsed.mode != "research"
            || parsed.nodes.is_empty()
            || parsed.nodes.len() > MAX_IMPORT_NODES
            || parsed.revision > library::MAX_SAFE_INTEGER
        {
            return Err(invalid());
        }
        for id in [&parsed.id, &parsed.root_id] {
            validation::length(id, 1, 200).map_err(|_| invalid())?;
        }
        validation::question(&mut parsed.original_prompt).map_err(|_| invalid())?;
        for timestamp in [&parsed.created_at, &parsed.updated_at] {
            if !timestamp.ends_with('Z')
                || time::OffsetDateTime::parse(
                    timestamp,
                    &time::format_description::well_known::Rfc3339,
                )
                .is_err()
            {
                return Err(invalid());
            }
        }
        for (key, node) in &mut parsed.nodes {
            let raw = &value["nodes"][key];
            validation::required(raw, &["parentId", "content", "lastError", "isRefreshing"])
                .map_err(|_| invalid())?;
            if node.id != *key
                || !["suggested", "researching", "ready", "failed"].contains(&node.status.as_str())
                || (node.status == "ready" && node.content.is_none())
            {
                return Err(invalid());
            }
            validation::length(key, 1, 200).map_err(|_| invalid())?;
            validation::question(&mut node.question).map_err(|_| invalid())?;
            validation::length(&node.relationship, 0, 100).map_err(|_| invalid())?;
            validation::length(&node.teaser, 0, 6000).map_err(|_| invalid())?;
            if let Some(parent) = &node.parent_id {
                validation::length(parent, 1, 200).map_err(|_| invalid())?;
            }
            for child in &node.child_ids {
                validation::length(child, 1, 200).map_err(|_| invalid())?;
            }
            if node.content.is_some() {
                let ResearchResult::Topic { content } = ResearchResult::from_json(
                    serde_json::json!({"kind":"topic","content":raw["content"]}),
                )
                .map_err(|_| invalid())?
                else {
                    return Err(invalid());
                };
                node.content = Some(content);
            }
            if let Some(error) = &node.last_error
                && (!["code", "message", "requestId"]
                    .iter()
                    .all(|k| error[*k].is_string())
                    || !error["retryable"].is_boolean())
            {
                return Err(invalid());
            }
        }
        if parsed
            .nodes
            .get(&parsed.root_id)
            .is_none_or(|n| n.parent_id.is_some())
        {
            return Err(invalid());
        }
        let mut stack = vec![parsed.root_id.as_str()];
        let mut seen = HashSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                return Err(invalid());
            }
            let node = parsed.nodes.get(id).ok_or_else(invalid)?;
            for child in &node.child_ids {
                if parsed
                    .nodes
                    .get(child)
                    .is_none_or(|n| n.parent_id.as_deref() != Some(id))
                {
                    return Err(invalid());
                }
                stack.push(child);
            }
        }
        if seen.len() != parsed.nodes.len() {
            return Err(invalid());
        }
        let view = &parsed.view;
        let selected = view["selectedNodeId"].as_str().ok_or_else(invalid)?;
        let expanded = view["expandedNodeIds"].as_array().ok_or_else(invalid)?;
        if !parsed.nodes.contains_key(selected)
            || expanded
                .iter()
                .any(|v| v.as_str().is_none_or(|id| !parsed.nodes.contains_key(id)))
            || !["map", "reader"].contains(&view["mobilePane"].as_str().ok_or_else(invalid)?)
        {
            return Err(invalid());
        }
        for axis in ["x", "y", "zoom"] {
            let number = view["viewport"][axis].as_f64().ok_or_else(invalid)?;
            if !number.is_finite() || (axis == "zoom" && !(0.1..=3.).contains(&number)) {
                return Err(invalid());
            }
        }
        Ok(parsed)
    }
    /// Only durable content determines identity. View changes update updatedAt in v1.
    pub fn content_identity(&self) -> Value {
        let nodes=self.nodes.iter().map(|(key,n)|(key.clone(),serde_json::json!({"id":n.id,"parentId":n.parent_id,"question":n.question,"relationship":n.relationship,"teaser":n.teaser,"content":n.content,"childIds":n.child_ids}))).collect::<serde_json::Map<_,_>>();
        serde_json::json!({"digestVersion":1,"id":self.id,"originalPrompt":self.original_prompt,"rootId":self.root_id,"nodes":nodes,"createdAt":self.created_at})
    }
    pub fn digest(&self) -> String {
        crate::storage::records::canonical_hash(&self.content_identity())
    }
}
