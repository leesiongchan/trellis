#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::{
    contracts::*,
    error::{AppError, ErrorCode},
    providers::*,
    research::context::JobContext,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU8, AtomicUsize, Ordering},
};
use tokio::sync::Semaphore;
pub fn content() -> TopicContent {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/results.json"
    ))
    .unwrap();
    let ResearchResult::Topic { content } =
        ResearchResult::from_json(fixtures[0]["input"].clone()).unwrap()
    else {
        unreachable!()
    };
    content
}
pub struct Provider {
    pub searches: AtomicUsize,
    pub generations: AtomicUsize,
    pub verifications: AtomicUsize,
    pub release: Semaphore,
    pub mode: AtomicU8,
    pub requests: Mutex<Vec<ResearchRequest>>,
}
impl Provider {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            searches: AtomicUsize::new(0),
            generations: AtomicUsize::new(0),
            verifications: AtomicUsize::new(0),
            release: Semaphore::new(0),
            mode: AtomicU8::new(0),
            requests: Mutex::new(vec![]),
        })
    }
}
impl ResearchProvider for Provider {
    fn search<'a>(
        &'a self,
        _: &'a str,
        _: &'a JobContext,
    ) -> ProviderFuture<'a, Vec<RetrievedSource>> {
        Box::pin(async move {
            self.searches.fetch_add(1, Ordering::SeqCst);
            self.release.acquire().await.unwrap().forget();
            Ok(vec![RetrievedSource {
                source: content().sources[0].clone(),
                text: "Flash retains data without power. A controller manages flash storage."
                    .into(),
            }])
        })
    }
    fn generate<'a>(
        &'a self,
        input: &'a ResearchRequest,
        _: &'a [RetrievedSource],
        _: &'a JobContext,
    ) -> ProviderFuture<'a, ResearchResult> {
        Box::pin(async move {
            self.generations.fetch_add(1, Ordering::SeqCst);
            self.requests.lock().unwrap().push(input.clone());
            match self.mode.load(Ordering::SeqCst) {
                1 => Ok(ResearchResult::Clarification {
                    question: "Which memory?".into(),
                }),
                3 => Err(AppError::new(ErrorCode::UpstreamFailure)),
                _ => Ok(ResearchResult::Topic { content: content() }),
            }
        })
    }
    fn verify<'a>(
        &'a self,
        _: &'a TopicContent,
        _: &'a [RetrievedSource],
        _: &'a JobContext,
    ) -> ProviderFuture<'a, GroundingVerdict> {
        Box::pin(async move {
            self.verifications.fetch_add(1, Ordering::SeqCst);
            Ok(GroundingVerdict {
                unsupported_paragraph_ids: if self.mode.load(Ordering::SeqCst) == 2 {
                    vec!["p1".into()]
                } else {
                    vec![]
                },
                unsupported_branch_indexes: vec![],
            })
        })
    }
}
