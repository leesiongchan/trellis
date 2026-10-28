#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::{
    contracts::*,
    providers::*,
    research::{context::JobContext, grounding::validate_evidence, research_topic},
};
use serde_json::{Value, json};
use std::{sync::Mutex, time::Duration};
fn content() -> TopicContent {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/contracts/results.json")).unwrap();
    let ResearchResult::Topic { content } =
        ResearchResult::from_json(cases[0]["input"].clone()).unwrap()
    else {
        unreachable!()
    };
    content
}
fn sources() -> Vec<RetrievedSource> {
    vec![RetrievedSource {
        source: content().sources[0].clone(),
        text: " Flash\n retains   data without power. A controller manages flash storage.".into(),
    }]
}
fn context() -> JobContext {
    JobContext {
        deadline: tokio::time::Instant::now() + Duration::from_secs(2),
        cancel: tokio_util::sync::CancellationToken::new(),
        job_id: "pipeline-test".into(),
        telemetry: None,
    }
}
fn input() -> ResearchRequest {
    ResearchRequest::from_json(json!({"explorationId":"e","nodeId":"n","question":"microSD","clarification":null,"ancestors":[]})).unwrap()
}
struct Provider {
    empty: bool,
    clarify: bool,
    unsupported: bool,
    calls: Mutex<Vec<&'static str>>,
}
impl Provider {
    fn new() -> Self {
        Self {
            empty: false,
            clarify: false,
            unsupported: false,
            calls: Mutex::new(vec![]),
        }
    }
}
impl ResearchProvider for Provider {
    fn search<'a>(
        &'a self,
        _: &'a str,
        _: &'a JobContext,
    ) -> ProviderFuture<'a, Vec<RetrievedSource>> {
        Box::pin(async {
            self.calls.lock().unwrap().push("search");
            Ok(if self.empty { vec![] } else { sources() })
        })
    }
    fn generate<'a>(
        &'a self,
        _: &'a ResearchRequest,
        _: &'a [RetrievedSource],
        _: &'a JobContext,
    ) -> ProviderFuture<'a, ResearchResult> {
        Box::pin(async {
            self.calls.lock().unwrap().push("generate");
            Ok(if self.clarify {
                ResearchResult::Clarification {
                    question: "Which?".into(),
                }
            } else {
                ResearchResult::Topic { content: content() }
            })
        })
    }
    fn verify<'a>(
        &'a self,
        _: &'a TopicContent,
        _: &'a [RetrievedSource],
        _: &'a JobContext,
    ) -> ProviderFuture<'a, GroundingVerdict> {
        Box::pin(async {
            self.calls.lock().unwrap().push("verify");
            Ok(GroundingVerdict {
                unsupported_paragraph_ids: if self.unsupported {
                    vec!["p1".into()]
                } else {
                    vec![]
                },
                unsupported_branch_indexes: vec![],
            })
        })
    }
}
#[tokio::test]
async fn pipeline_orders_and_short_circuits() {
    let mut provider = Provider::new();
    provider.empty = true;
    assert!(matches!(
        research_topic(&input(), &context(), &provider)
            .await
            .unwrap(),
        ResearchResult::InsufficientEvidence { .. }
    ));
    assert_eq!(*provider.calls.lock().unwrap(), vec!["search"]);
    let mut provider = Provider::new();
    provider.clarify = true;
    assert!(matches!(
        research_topic(&input(), &context(), &provider)
            .await
            .unwrap(),
        ResearchResult::Clarification { .. }
    ));
    assert_eq!(*provider.calls.lock().unwrap(), vec!["search", "generate"]);
    let provider = Provider::new();
    assert!(matches!(
        research_topic(&input(), &context(), &provider)
            .await
            .unwrap(),
        ResearchResult::Topic { .. }
    ));
    assert_eq!(
        *provider.calls.lock().unwrap(),
        vec!["search", "generate", "verify"]
    );
}
#[test]
fn grounding_rejects_invented_sources_quotes_and_missing_evidence() {
    assert!(validate_evidence(&content(), &sources()).is_ok());
    for variant in 0..5 {
        let mut content = content();
        match variant {
            0 => content.sources[0].url = "https://invented.example/".into(),
            1 => content.summary[0].evidence[0].source_id = "missing".into(),
            2 => content.summary[0].evidence[0].quote = "Invented quote".into(),
            3 => content.summary[0].evidence.clear(),
            _ => content.branches[0].reason.evidence.clear(),
        };
        assert!(
            validate_evidence(&content, &sources()).is_err(),
            "variant {variant}"
        );
    }
}
#[tokio::test]
async fn unsupported_verdict_is_not_a_researched_topic() {
    let mut provider = Provider::new();
    provider.unsupported = true;
    assert!(matches!(
        research_topic(&input(), &context(), &provider)
            .await
            .unwrap(),
        ResearchResult::InsufficientEvidence { .. }
    ));
}
#[tokio::test]
async fn cancelled_context_makes_no_provider_requests() {
    let provider = Provider::new();
    let ctx = context();
    ctx.cancel.cancel();
    assert_eq!(
        research_topic(&input(), &ctx, &provider)
            .await
            .unwrap_err()
            .code(),
        "RESEARCH_TIMEOUT"
    );
    assert!(provider.calls.lock().unwrap().is_empty());
}
