//! Explicitly ignored fixture harness; it is never linked into the release executable.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use axum::{Json, Router, http::StatusCode, routing::post};
use branch_backend::{
    app::{AppState, router},
    config::Config,
    http::static_files::StaticAssets,
    jobs::Coordinator,
    providers::{HttpProviders, ProviderEndpoints},
    server::Server,
    telemetry::{Telemetry, init_logging, install_redacted_panic_hook},
};
use serde_json::{Value, json};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;
const EVIDENCE: &str = "Flash retains data without power. A controller manages flash storage.";
async fn search(Json(input): Json<Value>) -> Result<Json<Value>, StatusCode> {
    if input["search_depth"] != "advanced" || input["max_results"] != 5 {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Json(
        json!({"results":[{"title":"Fixture memory reference","url":"https://example.org/memory","raw_content":EVIDENCE}]}),
    ))
}
async fn respond(Json(input): Json<Value>) -> Result<Json<Value>, StatusCode> {
    if input["store"] != false
        || input.get("tools").is_some()
        || input["text"]["format"]["strict"] != true
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let data: Value = serde_json::from_str(
        input["input"][1]["content"]
            .as_str()
            .ok_or(StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;
    let output = if input["text"]["format"]["name"] == "evidence_check" {
        json!({"unsupportedParagraphIds":[],"unsupportedBranchIndexes":[]})
    } else {
        let title = data["topic"]["question"]
            .as_str()
            .ok_or(StatusCode::BAD_REQUEST)?;
        let source = data["sources"][0]["id"]
            .as_str()
            .ok_or(StatusCode::BAD_REQUEST)?;
        let paragraph =
            |text: &str| json!({"text":text,"evidence":[{"sourceId":source,"quote":text}]});
        let child = match title {
            "microSD cards" => Some("NAND flash"),
            "NAND flash" => Some("Memory cells"),
            "Memory cells" => Some("Quantum tunneling"),
            _ => None,
        };
        let branches=child.map(|title|vec![json!({"title":title,"question":title,"relationship":"Component","reason":paragraph("Flash retains data without power.")})]).unwrap_or_default();
        json!({"result":{"kind":"topic","title":title,"summary":[paragraph("Flash retains data without power.")],"sections":[{"heading":"Mechanism","paragraphs":[paragraph("A controller manages flash storage.")]}],"analogy":null,"limitations":null,"branches":branches}})
    };
    Ok(Json(
        json!({"id":"resp_fixture","object":"response","status":"completed","error":null,"incomplete_details":null,"output":[{"id":"msg_fixture","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":output.to_string(),"annotations":[]}]}]}),
    ))
}
#[tokio::test]
#[ignore = "Only launched by scripts/test-server.ts for local browser acceptance"]
async fn serve_browser_fixtures() {
    install_redacted_panic_hook();
    init_logging("warn").unwrap();
    let upstream = common::spawn_router(
        Router::new()
            .route("/search", post(search))
            .route("/responses", post(respond)),
    )
    .await;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let assets = Arc::new(StaticAssets::load(&root.join("release/public")).unwrap());
    let shutdown = CancellationToken::new();
    let mut tasks = tokio::task::JoinSet::new();
    for (port, origin) in [
        (5184, "http://127.0.0.1:5184"),
        (5186, "https://127.0.0.1:5185"),
    ] {
        let values = HashMap::from([
            ("APP_ORIGIN".into(), origin.into()),
            ("PORT".into(), port.to_string()),
            ("APP_ACCESS_KEY".into(), "fixture-access-key-".repeat(4)),
            ("OPENAI_API_KEY".into(), "fixture-openai".into()),
            ("TAVILY_API_KEY".into(), "fixture-tavily".into()),
        ]);
        let config =
            Arc::new(Config::from_values(&values, &root.join("release/branch-backend")).unwrap());
        let telemetry = Arc::new(Telemetry::new());
        let provider = Arc::new(
            HttpProviders::with_endpoints(
                config.clone(),
                ProviderEndpoints {
                    openai: url::Url::parse(&format!("{}/responses", upstream.origin())).unwrap(),
                    tavily: url::Url::parse(&format!("{}/search", upstream.origin())).unwrap(),
                },
            )
            .unwrap()
            .with_telemetry(telemetry.clone()),
        );
        let jobs = Coordinator::instrumented(provider, config.limits.clone(), telemetry.clone());
        let draining = CancellationToken::new();
        let state = Arc::new(AppState {
            config: config.clone(),
            jobs: jobs.clone(),
            telemetry,
            draining: draining.clone(),
        });
        let server = Server::bind(
            router(state, assets.clone()),
            config.bind,
            jobs,
            draining,
            config.limits.clone(),
        )
        .await
        .unwrap();
        let signal = shutdown.clone();
        tasks.spawn(server.run(signal.cancelled_owned()));
    }
    let mut terminate =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
    tokio::select! {_=tokio::signal::ctrl_c()=>{},_=terminate.recv()=>{}}
    shutdown.cancel();
    while let Some(result) = tasks.join_next().await {
        result.unwrap().unwrap();
    }
}
