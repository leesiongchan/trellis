#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use base64::{Engine, engine::general_purpose::STANDARD};
use branch_backend::{
    app::{AppState, router},
    http::static_files::StaticAssets,
    providers::HttpProviders,
    telemetry::Telemetry,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::Arc};
fn fixture() -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().unwrap();
    let public = dir.path().join("public");
    fs::create_dir_all(public.join("_app/immutable")).unwrap();
    let script = "window.fixture=true;";
    let html =
        format!("<!doctype html><html><head></head><body><script>{script}</script></body></html>");
    let hash = format!("sha256-{}", STANDARD.encode(Sha256::digest(script)));
    let files = [
        ("index.html", html.as_str(), false),
        ("_app/immutable/app.abc.css", "body{}", true),
        ("_app/version.json", "{}", false),
    ];
    let mut entries = vec![];
    for (path, body, immutable) in files {
        fs::write(public.join(path), body).unwrap();
        entries.push(
            json!({"path":path,"sha256":hex::encode(Sha256::digest(body)),"immutable":immutable}),
        );
    }
    fs::write(
        dir.path().join("frontend-manifest.json"),
        json!({"version":1,"files":entries,"inlineScriptHashes":[hash]}).to_string(),
    )
    .unwrap();
    fs::write(public.join(".env"), "private-release-secret").unwrap();
    (dir, public, hash)
}
async fn server(public: PathBuf) -> common::TestServer {
    let (config, data) = common::config("https://private.example:8443");
    let mut config = (*config).clone();
    config.asset_dir = public.clone();
    let config = Arc::new(config);
    let provider = Arc::new(HttpProviders::production(config.clone()).unwrap());
    let state = AppState::start(
        config,
        provider.clone(),
        Some(provider),
        Arc::new(Telemetry::new()),
    )
    .await
    .unwrap();
    let assets = Arc::new(StaticAssets::load(&public).unwrap());
    common::spawn_router(router(state, assets))
        .await
        .keep_data_directory(data)
}
#[tokio::test]
async fn release_serves_only_verified_assets() {
    let (_dir, public, hash) = fixture();
    let server = server(public).await;
    let client = reqwest::Client::new();
    let root = client.get(server.origin()).send().await.unwrap();
    assert_eq!(root.status(), 200);
    assert!(
        root.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains(&hash)
    );
    assert_eq!(
        root.headers()["strict-transport-security"],
        "max-age=31536000"
    );
    assert_eq!(root.headers()["cache-control"], "no-cache");
    let css = client
        .get(format!("{}/_app/immutable/app.abc.css", server.origin()))
        .send()
        .await
        .unwrap();
    assert_eq!(css.status(), 200);
    assert_eq!(
        css.headers()["cache-control"],
        "public, max-age=31536000, immutable"
    );
    assert!(
        css.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/css")
    );
    let etag = css.headers()["etag"].clone();
    assert_eq!(
        client
            .get(format!("{}/_app/immutable/app.abc.css", server.origin()))
            .header("if-none-match", etag)
            .send()
            .await
            .unwrap()
            .status(),
        304
    );
    for path in [
        "/.env",
        "/frontend-manifest.json",
        "/_app/",
        "/_app/immutable/missing.js",
        "/unknown",
        "/%2e%2e/.env",
        "/api/missing",
    ] {
        let response = client
            .get(format!("{}{path}", server.origin()))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 404, "{path}");
        assert!(
            !response
                .text()
                .await
                .unwrap()
                .contains("private-release-secret")
        );
    }
}
#[test]
fn tampered_missing_or_escaped_assets_are_rejected_at_load() {
    let (dir, public, _) = fixture();
    assert!(StaticAssets::load(&public).is_ok());
    fs::write(public.join("index.html"), "tampered").unwrap();
    assert!(StaticAssets::load(&public).is_err());
    let (_dir, public, _) = fixture();
    fs::remove_file(public.join("index.html")).unwrap();
    assert!(StaticAssets::load(&public).is_err());
    let (_dir2, public2, _) = fixture();
    fs::remove_file(public2.join("index.html")).unwrap();
    std::os::unix::fs::symlink(
        dir.path().join("frontend-manifest.json"),
        public2.join("index.html"),
    )
    .unwrap();
    assert!(StaticAssets::load(&public2).is_err());
}
#[test]
fn asset_path_is_relative_to_executable_not_working_directory() {
    let data = tempfile::tempdir().unwrap();
    let config = branch_backend::config::Config::from_values(
        &std::collections::HashMap::from([
            ("APP_ORIGIN".into(), "http://127.0.0.1:3000".into()),
            (
                "APP_DATA_DIR".into(),
                data.path().join("data").display().to_string(),
            ),
        ]),
        std::path::Path::new("/different/release/branch-backend"),
    )
    .unwrap();
    assert_eq!(config.asset_dir, PathBuf::from("/different/release/public"));
}
