#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
#[path = "common/providers.rs"]
mod stubs;
use branch_backend::{
    access::issue_cookie,
    app::{AppState, api_router},
    providers::{HttpProviders, ProviderEndpoints},
    telemetry::Telemetry,
};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use stubs::{Script, StubProviders};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
async fn setup() -> (common::TestServer, Arc<AppState>, StubProviders) {
    setup_with_keys(true).await
}
async fn setup_with_keys(configured: bool) -> (common::TestServer, Arc<AppState>, StubProviders) {
    let stub = StubProviders::start(vec![Script::json(json!({"results":[]}))]).await;
    let (config, data) = common::config("http://127.0.0.1:3000");
    let mut config = (*config).clone();
    config.limits.body_read = Duration::from_millis(30);
    if !configured {
        config.openai_key = Default::default();
        config.tavily_key = Default::default();
    }
    let config = Arc::new(config);
    let provider = Arc::new(
        HttpProviders::with_endpoints(
            config.clone(),
            ProviderEndpoints {
                openai: stub.url(),
                tavily: stub.url(),
            },
        )
        .unwrap(),
    );
    let telemetry = Arc::new(Telemetry::new());
    let state = AppState::start(config, provider.clone(), Some(provider), telemetry)
        .await
        .unwrap();
    let server = common::spawn_router(api_router(state.clone()))
        .await
        .keep_data_directory(data);
    (server, state, stub)
}

#[tokio::test]
async fn ready_without_provider_keys_still_serves_read_only_mode() {
    let (server, _, stub) = setup_with_keys(false).await;
    let response = reqwest::get(format!("{}/health/ready", server.origin()))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.json::<Value>().await.unwrap()["researchConfigured"],
        false
    );
    assert_error(
        request(&server).json(&body()).send().await.unwrap(),
        503,
        "SETUP_REQUIRED",
    )
    .await;
    assert_eq!(stub.count(), 0);
}
fn body() -> Value {
    json!({"explorationId":"e","nodeId":"n","question":"private-question","clarification":null,"ancestors":[]})
}
fn request(server: &common::TestServer) -> reqwest::RequestBuilder {
    reqwest::Client::new()
        .post(format!("{}/api/research", server.origin()))
        .header("host", "127.0.0.1:3000")
        .header("origin", "http://127.0.0.1:3000")
        .header("cookie", cookie())
        .header("x-request-id", "known-request")
}
fn cookie() -> String {
    format!(
        "branch_access={}",
        issue_cookie(&"a".repeat(64), 4102444800000).unwrap()
    )
}
async fn with_header(
    builder: reqwest::RequestBuilder,
    name: &str,
    value: &str,
) -> reqwest::Response {
    let mut request = builder.build().unwrap();
    request.headers_mut().insert(
        reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
        value.parse().unwrap(),
    );
    reqwest::Client::new().execute(request).await.unwrap()
}
async fn assert_error(response: reqwest::Response, status: u16, code: &str) {
    assert_eq!(response.status(), status);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(response.headers().contains_key("x-request-id"));
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], code);
    assert!(body["error"]["requestId"].is_string());
}
#[tokio::test]
async fn access_and_origin_checks_precede_provider_work() {
    let (server, _, stub) = setup().await;
    assert_error(
        with_header(request(&server).json(&body()), "cookie", "").await,
        401,
        "ACCESS_REQUIRED",
    )
    .await;
    for (header, value) in [
        ("origin", "https://evil.example"),
        ("host", "evil.example"),
        ("origin", ""),
    ] {
        assert_error(
            with_header(
                request(&server)
                    .header("x-forwarded-host", "127.0.0.1:3000")
                    .json(&body()),
                header,
                value,
            )
            .await,
            403,
            "ORIGIN_REJECTED",
        )
        .await;
    }
    assert_eq!(stub.count(), 0);
    let response = request(&server).json(&body()).send().await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["x-request-id"], "known-request");
    assert_eq!(
        response.json::<Value>().await.unwrap()["kind"],
        "insufficient-evidence"
    );
}

#[tokio::test]
async fn duplicate_origin_is_rejected_before_provider_work() {
    let (server, _, stub) = setup().await;
    assert_error(
        request(&server)
            .header("origin", "https://evil.example")
            .json(&body())
            .send()
            .await
            .unwrap(),
        403,
        "ORIGIN_REJECTED",
    )
    .await;
    assert_eq!(stub.count(), 0);
}

#[tokio::test]
async fn malformed_request_id_is_replaced_consistently() {
    let (server, _, _) = setup().await;
    let response = with_header(
        request(&server)
            .header("content-type", "application/json")
            .body("{bad"),
        "x-request-id",
        "untrusted id with spaces",
    )
    .await;
    assert_eq!(response.status(), 400);
    let id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    assert_ne!(id, "untrusted id with spaces");
    assert_eq!(
        response.json::<Value>().await.unwrap()["error"]["requestId"],
        id
    );
}
#[tokio::test]
async fn invalid_requests_use_one_safe_envelope() {
    let (server, _, stub) = setup().await;
    assert_error(
        request(&server).body("{}").send().await.unwrap(),
        400,
        "INVALID_CONTENT_TYPE",
    )
    .await;
    assert_error(
        request(&server)
            .header("content-type", "application/json")
            .body("{bad")
            .send()
            .await
            .unwrap(),
        400,
        "INVALID_JSON",
    )
    .await;
    assert_error(
        request(&server).json(&json!({})).send().await.unwrap(),
        422,
        "INVALID_QUESTION",
    )
    .await;
    assert_error(
        request(&server)
            .header("content-type", "application/json")
            .body("x".repeat(65537))
            .send()
            .await
            .unwrap(),
        413,
        "BODY_TOO_LARGE",
    )
    .await;
    assert_error(
        reqwest::get(format!("{}/api/missing", server.origin()))
            .await
            .unwrap(),
        404,
        "NOT_FOUND",
    )
    .await;
    let wrong = reqwest::get(format!("{}/api/research", server.origin()))
        .await
        .unwrap();
    assert!(wrong.headers().contains_key("allow"));
    assert_error(wrong, 405, "METHOD_NOT_ALLOWED").await;
    assert_eq!(stub.count(), 0);
}
#[tokio::test]
async fn slow_chunked_body_has_a_total_deadline() {
    let (server, _, _) = setup().await;
    let mut socket = tokio::net::TcpStream::connect(server.address)
        .await
        .unwrap();
    socket.write_all(format!("POST /api/research HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nOrigin: http://127.0.0.1:3000\r\nCookie: {}\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",cookie()).as_bytes()).await.unwrap();
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(1), socket.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(response.starts_with("HTTP/1.1 408"), "{response}");
    assert!(response.contains("BODY_TIMEOUT"));
}

#[tokio::test]
async fn chunked_body_is_limited_without_content_length() {
    let (server, _, stub) = setup().await;
    let mut socket = tokio::net::TcpStream::connect(server.address)
        .await
        .unwrap();
    let body = "x".repeat(65537);
    socket.write_all(format!("POST /api/research HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nOrigin: http://127.0.0.1:3000\r\nCookie: {}\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{body}\r\n0\r\n\r\n",cookie(),body.len()).as_bytes()).await.unwrap();
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(1), socket.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(response.starts_with("HTTP/1.1 413"), "{response}");
    assert!(response.contains("BODY_TOO_LARGE"));
    assert_eq!(stub.count(), 0);
}
#[tokio::test]
async fn readiness_and_private_metrics_are_explicit() {
    let (server, state, _) = setup().await;
    let client = reqwest::Client::new();
    let ready: Value = client
        .get(format!("{}/health/ready", server.origin()))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ready["researchConfigured"], true);
    assert_eq!(
        client
            .get(format!("{}/metrics", server.origin()))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let metrics = client
        .get(format!("{}/metrics", server.origin()))
        .header("cookie", cookie())
        .send()
        .await
        .unwrap();
    assert_eq!(metrics.status(), 200);
    state.draining.cancel();
    assert_eq!(
        client
            .get(format!("{}/health/ready", server.origin()))
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
}
