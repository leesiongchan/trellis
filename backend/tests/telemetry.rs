#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::telemetry::Telemetry;
mod common;
#[path = "common/providers.rs"]
mod stubs;
use branch_backend::{
    access::issue_cookie,
    app::{AppState, api_router},
    jobs::Coordinator,
    providers::{HttpProviders, ProviderEndpoints},
};
use serde_json::json;
use std::{
    io::Write,
    sync::{Arc, Mutex},
};
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}
#[test]
fn metrics_expose_aggregate_instruments_without_user_labels() {
    let telemetry = Telemetry::new();
    telemetry.http(
        "/health/live",
        &axum::http::Method::GET,
        axum::http::StatusCode::OK,
        std::time::Duration::from_millis(1),
    );
    let text = telemetry.render().unwrap();
    assert!(text.contains("branch_http_requests"));
    assert!(text.contains("branch_active_jobs"));
    for forbidden in ["requestId", "nodeId", "question", "cookie", "authorization"] {
        assert!(!text.contains(forbidden));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn failed_research_is_correlated_and_redacted_in_actual_logs() {
    let capture = Capture::default();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_writer(capture.clone())
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let upstream = stubs::StubProviders::start(vec![stubs::Script {
        status: axum::http::StatusCode::BAD_GATEWAY,
        ..stubs::Script::json(
            json!({"private":"private-provider-body","source":"private-source-text"}),
        )
    }])
    .await;
    let config = common::config("http://127.0.0.1:3000");
    let telemetry = Arc::new(Telemetry::new());
    let provider = Arc::new(
        HttpProviders::with_endpoints(
            config.clone(),
            ProviderEndpoints {
                openai: upstream.url(),
                tavily: upstream.url(),
            },
        )
        .unwrap()
        .with_telemetry(telemetry.clone()),
    );
    let jobs = Coordinator::instrumented(provider, config.limits.clone(), telemetry.clone());
    let state = Arc::new(AppState {
        config: config.clone(),
        jobs: jobs.clone(),
        telemetry: telemetry.clone(),
        draining: tokio_util::sync::CancellationToken::new(),
    });
    let server = common::spawn_router(api_router(state)).await;
    let client = reqwest::Client::new();
    let response=client.post(format!("{}/api/research",server.origin())).header("host","127.0.0.1:3000").header("origin","http://127.0.0.1:3000").header("x-request-id","diagnostic-request").header("cookie",format!("branch_access={}",issue_cookie(config.access_key.expose(),4102444800000).unwrap())).json(&json!({"explorationId":"e","nodeId":"n","question":"private-user-question","clarification":null,"ancestors":[{"title":"private-ancestor-title","summary":"private-ancestor-summary"}]})).send().await.unwrap();
    assert_eq!(response.status(), 502);
    let _ = client
        .get(format!("{}/access?key=private-query-key", server.origin()))
        .send()
        .await
        .unwrap();
    let _ = client
        .get(format!("{}/metrics", server.origin()))
        .header("cookie", "branch_access=private-cookie")
        .send()
        .await
        .unwrap();
    jobs.idle().await;
    let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    let metrics = telemetry.render().unwrap();
    for expected in [
        "diagnostic-request",
        "jobId",
        "search",
        "SEARCH_FAILED",
        "durationMs",
    ] {
        assert!(logs.contains(expected), "missing {expected}: {logs}");
    }
    for secret in [
        "private-user-question",
        "private-ancestor-title",
        "private-ancestor-summary",
        "private-provider-body",
        "private-source-text",
        "private-query-key",
        "private-cookie",
        "fake-tavily",
        config.access_key.expose(),
    ] {
        assert!(!logs.contains(secret), "logged {secret}");
        assert!(!metrics.contains(secret));
    }
    for line in logs.lines() {
        assert!(serde_json::from_str::<serde_json::Value>(line).is_ok());
    }
    assert!(metrics.contains("branch_research_stage_duration_seconds_bucket"));
    assert!(metrics.contains("SEARCH_FAILED"));
    assert!(metrics.contains("branch_active_jobs 0"));
    assert!(metrics.contains("branch_waiters 0"));
}
