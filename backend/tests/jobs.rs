#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use branch_backend::{
    config::Limits,
    contracts::*,
    error::{AppError, ErrorCode},
    jobs::Coordinator,
    providers::*,
    research::context::JobContext,
};
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::Semaphore;
struct Provider {
    calls: AtomicUsize,
    release: Semaphore,
    mode: u8,
}
impl Provider {
    fn new(mode: u8) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            release: Semaphore::new(0),
            mode,
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
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if self.mode == 2 && n == 0 {
                panic!("private-provider-panic");
            }
            if self.mode == 3 && n == 0 {
                return Err(AppError::new(ErrorCode::SearchFailed));
            }
            if self.mode == 0 {
                let permit = self.release.acquire().await.unwrap();
                permit.forget();
            }
            Ok(vec![])
        })
    }
    fn generate<'a>(
        &'a self,
        _: &'a ResearchRequest,
        _: &'a [RetrievedSource],
        _: &'a JobContext,
    ) -> ProviderFuture<'a, ResearchResult> {
        Box::pin(async { Err(AppError::new(ErrorCode::InternalFailure)) })
    }
    fn verify<'a>(
        &'a self,
        _: &'a TopicContent,
        _: &'a [RetrievedSource],
        _: &'a JobContext,
    ) -> ProviderFuture<'a, GroundingVerdict> {
        Box::pin(async { Err(AppError::new(ErrorCode::InternalFailure)) })
    }
}
fn input(node: &str) -> ResearchRequest {
    ResearchRequest::from_json(json!({"explorationId":"e","nodeId":node,"question":"q","clarification":null,"ancestors":[]})).unwrap()
}
#[tokio::test(start_paused = true)]
async fn duplicate_requests_share_work_and_survive_one_disconnect() {
    let p = Provider::new(0);
    let c = Coordinator::new(p.clone(), Limits::default());
    let a = c.admit(input("n")).unwrap();
    let mut b = c.admit(input("n")).unwrap();
    tokio::task::yield_now().await;
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    assert_eq!(c.snapshot().waiters, 2);
    drop(a);
    assert_eq!(c.snapshot().waiters, 1);
    p.release.add_permits(1);
    assert!(matches!(
        *b.result().await.unwrap(),
        ResearchResult::InsufficientEvidence { .. }
    ));
    drop(b);
    c.idle().await;
    assert_eq!(c.snapshot().active, 0);
    assert_eq!(c.snapshot().waiters, 0);
}
#[tokio::test(start_paused = true)]
async fn concurrent_jobs_and_waiters_are_bounded() {
    let p = Provider::new(0);
    let c = Coordinator::new(p, Limits::default());
    let a = c.admit(input("a")).unwrap();
    let b = c.admit(input("b")).unwrap();
    assert_eq!(c.admit(input("c")).err().unwrap().code(), "RESEARCH_LIMIT");
    let mut waiters = vec![a, b];
    for _ in 0..30 {
        waiters.push(c.admit(input("a")).unwrap());
    }
    assert_eq!(c.snapshot().waiters, 32);
    assert_eq!(c.admit(input("a")).err().unwrap().code(), "RESEARCH_LIMIT");
    drop(waiters);
    c.idle().await;
    assert_eq!(c.snapshot().active, 0);
}
#[tokio::test(start_paused = true)]
async fn rolling_budget_expires_without_retaining_completed_jobs() {
    let c = Coordinator::new(Provider::new(1), Limits::default());
    for n in 0..10 {
        let mut waiter = c.admit(input(&n.to_string())).unwrap();
        waiter.result().await.unwrap();
        drop(waiter);
    }
    c.idle().await;
    assert_eq!(
        c.admit(input("eleven")).err().unwrap().code(),
        "RESEARCH_LIMIT"
    );
    tokio::time::advance(Duration::from_secs(60)).await;
    let mut w = c.admit(input("eleven")).unwrap();
    assert!(w.result().await.is_ok());
}
#[tokio::test(start_paused = true)]
async fn timeout_error_and_panic_release_job_capacity() {
    for mode in [0, 2, 3] {
        let c = Coordinator::new(Provider::new(mode), Limits::default());
        let mut w = c.admit(input("n")).unwrap();
        let code = w.result().await.err().unwrap().code();
        assert_eq!(
            code,
            match mode {
                0 => "RESEARCH_TIMEOUT",
                2 => "INTERNAL_FAILURE",
                _ => "SEARCH_FAILED",
            }
        );
        drop(w);
        c.idle().await;
        assert_eq!(c.snapshot().active, 0);
        assert_eq!(c.snapshot().waiters, 0);
        assert!(c.admit(input("again")).is_ok());
        c.cancel_all();
        c.idle().await;
    }
}
#[tokio::test(start_paused = true)]
async fn old_cleanup_cannot_remove_replacement() {
    let p = Provider::new(0);
    let c = Coordinator::new(p.clone(), Limits::default());
    let old = c.admit(input("same")).unwrap();
    tokio::task::yield_now().await;
    drop(old);
    let mut replacement = c.admit(input("same")).unwrap();
    tokio::task::yield_now().await;
    assert_eq!(c.snapshot().active, 1);
    assert_eq!(c.snapshot().waiters, 1);
    p.release.add_permits(1);
    assert!(replacement.result().await.is_ok());
    drop(replacement);
    c.idle().await;
}
#[tokio::test(start_paused = true)]
async fn drain_rejects_admission_but_finishes_existing_work() {
    let p = Provider::new(0);
    let c = Coordinator::new(p.clone(), Limits::default());
    let mut w = c.admit(input("old")).unwrap();
    c.begin_drain();
    assert_eq!(
        c.admit(input("new")).err().unwrap().code(),
        "SERVER_DRAINING"
    );
    p.release.add_permits(1);
    assert!(w.result().await.is_ok());
    drop(w);
    c.idle().await;
}
