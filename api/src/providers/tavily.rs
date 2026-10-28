use super::*;
use serde_json::{Value, json};
use std::collections::HashSet;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
impl HttpProviders {
    pub(super) async fn search_evidence(
        &self,
        query: &str,
        ctx: &JobContext,
    ) -> Result<Vec<RetrievedSource>, AppError> {
        let body = json!({"query":query,"search_depth":"advanced","max_results":5,"include_raw_content":"text","include_answer":false,"include_images":false});
        let raw = self
            .post(
                &self.endpoints.tavily,
                self.config.tavily_key.expose(),
                &body,
                ctx,
                tokio::time::Instant::now() + self.config.limits.search,
                ErrorCode::SearchFailed,
            )
            .await?;
        parse_search_results(raw)
    }
}
pub fn parse_search_results(raw: Value) -> Result<Vec<RetrievedSource>, AppError> {
    let results = raw
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::new(ErrorCode::SearchFailed))?;
    let mut urls = HashSet::new();
    let mut sources = Vec::new();
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
    for item in results {
        let (Some(title), Some(url), Some(text)) = (
            item["title"].as_str(),
            item["url"].as_str(),
            item["raw_content"].as_str(),
        ) else {
            continue;
        };
        if title.is_empty()
            || text.trim_matches(ecma_whitespace).is_empty()
            || !public_source_url(url)
            || urls.contains(url)
            || url.chars().count() > 4096
        {
            continue;
        }
        let Ok(parsed) = url::Url::parse(url) else {
            continue;
        };
        let source = Source {
            id: uuid::Uuid::new_v4().simple().to_string(),
            title: truncate_utf16(title, 500),
            url: url.into(),
            publisher: parsed.host_str().map(str::to_owned),
            retrieved_at: now.clone(),
        };
        urls.insert(url.to_owned());
        sources.push(RetrievedSource {
            source,
            text: truncate_utf16(text, 12000),
        });
        if sources.len() == 5 {
            break;
        }
    }
    Ok(sources)
}
