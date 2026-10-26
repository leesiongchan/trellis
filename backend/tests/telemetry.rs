#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::telemetry::Telemetry;
mod common;
#[path = "common/providers.rs"]
mod stubs;
use branch_backend::{
    access::issue_cookie,
    app::{AppState, api_router},
    contracts::library::*,
    providers::{HttpProviders, ProviderEndpoints},
};
use serde_json::json;
use std::sync::Arc;
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

struct LoggingProcess(std::process::Child);
impl Drop for LoggingProcess {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[test]
fn failed_research_is_correlated_and_redacted_in_actual_logs() {
    use std::{fs, process::Command, time::Duration};
    for (filter, info, debug, trace) in [
        ("debug", true, true, false),
        ("trace", true, true, true),
        (
            "off,branch_backend=debug,reqwest=trace,hyper_util=trace",
            true,
            true,
            false,
        ),
        (
            "off,reqwest=trace,hyper_util=trace,branch_backend_untrusted=trace",
            false,
            false,
            false,
        ),
    ] {
        let output = tempfile::tempdir().unwrap();
        let stdout = output.path().join("stdout.log");
        let stderr = output.path().join("stderr.log");
        // Global subscriber state must not leak between filter configurations.
        let mut child = LoggingProcess(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "production_logging_fixture",
                    "--ignored",
                    "--nocapture",
                    "--quiet",
                ])
                .env_clear()
                .env("APP_ENV_FILE", "")
                .env("RUST_LOG", filter)
                .stdin(std::process::Stdio::null())
                .stdout(fs::File::create(&stdout).unwrap())
                .stderr(fs::File::create(&stderr).unwrap())
                .spawn()
                .unwrap(),
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "logging fixture stalled"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        let logs = fs::read_to_string(stdout).unwrap();
        let errors = fs::read_to_string(stderr).unwrap();
        assert!(status.success(), "fixture failed for {filter}: {errors}");
        for secret in [
            "private-user-question",
            "private-ancestor-title",
            "private-ancestor-summary",
            "private-provider-body",
            "private-source-text",
            "private-query-key",
            "private-cookie",
            "private-dependency-error",
            "private-dependency-span",
            "private-prefix-spoof",
            "fake-tavily",
            "fake-openai",
            &"a".repeat(64),
            &issue_cookie(&"a".repeat(64), 4102444800000).unwrap(),
        ] {
            assert!(!logs.contains(secret), "logged {secret} with {filter}");
            assert!(
                !errors.contains(secret),
                "stderr logged {secret} with {filter}"
            );
        }
        let records: Vec<serde_json::Value> = logs
            .lines()
            .filter(|line| line.starts_with('{'))
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        for record in &records {
            let target = record["target"].as_str().unwrap();
            assert!(
                target == "branch_backend" || target.starts_with("branch_backend::"),
                "dependency target {target} bypassed {filter}"
            );
            assert_ne!(record["span"]["name"], "dependency_span");
            if let Some(spans) = record["spans"].as_array() {
                assert!(spans.iter().all(|span| span["name"] != "dependency_span"));
            }
        }
        let has_event = |name: &str| records.iter().any(|r| r["fields"]["event"] == name);
        assert_eq!(has_event("binary_logging_info"), info, "{filter}");
        assert_eq!(has_event("logging_debug"), debug, "{filter}");
        assert_eq!(has_event("logging_trace"), trace, "{filter}");
        assert_eq!(has_event("inside_dependency_span"), info, "{filter}");
        if info {
            for expected in [
                "diagnostic-request",
                "jobId",
                "search",
                "SEARCH_FAILED",
                "durationMs",
            ] {
                assert!(logs.contains(expected), "missing {expected} with {filter}");
            }
            let completion = records
                .iter()
                .find(|r| r["fields"]["event"] == "research_completed")
                .expect("research completion event");
            assert_eq!(completion["fields"]["code"], "SEARCH_FAILED");
            assert!(
                completion["spans"].as_array().is_some_and(|spans| spans
                    .iter()
                    .any(|span| span["requestId"] == "diagnostic-request")),
                "background completion lost originating request"
            );
        } else {
            assert!(records.is_empty(), "disabled app targets produced events");
        }
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "Subprocess fixture for production subscriber tests"]
async fn production_logging_fixture() {
    let telemetry = Arc::new(Telemetry::new());
    let _logging =
        branch_backend::telemetry::init_logging(&std::env::var("RUST_LOG").unwrap(), &telemetry)
            .unwrap();
    tracing::info!(target: "branch_backend", event = "binary_logging_info");
    tracing::debug!(target: "branch_backend::logging_fixture", event = "logging_debug");
    tracing::trace!(target: "branch_backend::logging_fixture", event = "logging_trace");
    tracing::error!(target: "reqwest", error = "private-dependency-error");
    tracing::trace!(target: "hyper_util::client", body = "private-provider-body");
    tracing::warn!(target: "branch_backend_untrusted", key = "private-prefix-spoof");
    {
        let application = tracing::info_span!(target: "branch_backend::logging_fixture", "application_scope", requestId = "diagnostic-request");
        let _application = application.enter();
        let dependency = tracing::info_span!(target: "reqwest::client", "dependency_span", authorization = "private-dependency-span");
        let _dependency = dependency.enter();
        tracing::info!(target: "branch_backend::logging_fixture", event = "inside_dependency_span");
    }
    let upstream = stubs::StubProviders::start(vec![stubs::Script {
        status: axum::http::StatusCode::BAD_GATEWAY,
        ..stubs::Script::json(
            json!({"private":"private-provider-body","source":"private-source-text"}),
        )
    }])
    .await;
    let (config, _data) = common::config("http://127.0.0.1:3000");
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
    let state = AppState::start(
        config.clone(),
        provider.clone(),
        Some(provider),
        telemetry.clone(),
    )
    .await
    .unwrap();
    let created = state
        .store
        .create_exploration(&Command {
            operation_id: OperationId::generate(),
            library_epoch: 1,
            expected_revision: None,
            payload: CreateExploration {
                question: "private-user-question".into(),
                locale: "en".into(),
            },
        })
        .await
        .unwrap();
    let Some(OperationResult::Exploration { appearance_id, .. }) = created.result else {
        unreachable!()
    };
    let operation_id = OperationId::generate();
    let server = common::spawn_router(api_router(state.clone())).await;
    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "{}/api/appearances/{appearance_id}/research",
            server.origin()
        ))
        .header("host", "127.0.0.1:3000")
        .header("origin", "http://127.0.0.1:3000")
        .header("x-request-id", "diagnostic-request")
        .header("x-branch-protocol", "2")
        .header("x-branch-library", state.library_id.as_str())
        .header(
            "cookie",
            format!(
                "branch_access={}",
                issue_cookie(config.access_key.expose(), 4102444800000).unwrap()
            ),
        )
        .json(&Command {
            operation_id: operation_id.clone(),
            library_epoch: 1,
            expected_revision: Some(0),
            payload: ResearchInput {
                appearance_id,
                refresh: false,
                clarification: None,
            },
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 202);
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
    state.knowledge.drain().await.unwrap();
    let operation = state.store.operation(&operation_id).await.unwrap();
    assert_eq!(operation.state, OperationState::Failed);
    assert_eq!(operation.error.unwrap().code, "SEARCH_FAILED");
    let metrics = telemetry.render().unwrap();
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
        assert!(!metrics.contains(secret));
    }
    assert!(metrics.contains("branch_research_stage_duration_seconds_bucket"));
    assert!(metrics.contains("SEARCH_FAILED"));
    assert!(metrics.contains("branch_active_jobs 0"));
    assert!(metrics.contains("branch_waiters 0"));
}
