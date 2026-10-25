#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use branch_backend::{
    access::{authorize_cookie, issue_cookie, routes},
    config::Config,
};
use serde_json::Value;
use std::{collections::HashMap, path::Path};

#[test]
fn legacy_cookie_roundtrips() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/contracts/cookies.json")).unwrap();
    for f in fixtures {
        let key = f["key"].as_str().unwrap();
        let token = f["token"].as_str().unwrap();
        assert!(authorize_cookie(
            Some(token),
            key,
            f["nowMs"].as_u64().unwrap()
        ));
        assert_eq!(
            issue_cookie(key, f["expiresMs"].as_u64().unwrap()).unwrap(),
            token
        );
        assert!(!authorize_cookie(
            Some(token),
            key,
            f["expiresMs"].as_u64().unwrap()
        ));
        assert!(!authorize_cookie(Some(token), &"b".repeat(64), 0));
        for bad in [
            format!("{token}.extra"),
            format!("0{token}"),
            token.replace('.', ".."),
            "NaN.bad".into(),
            "1e20.bad".into(),
        ] {
            assert!(!authorize_cookie(Some(&bad), key, 0));
        }
    }
}

#[test]
fn config_requires_valid_explicit_origin_and_loopback() {
    let mut values = HashMap::from([("APP_ORIGIN".into(), "http://127.0.0.1:3000".into())]);
    assert!(Config::from_values(&values, Path::new("/tmp/release/branch-backend")).is_ok());
    for (key, value) in [
        ("HOST", "0.0.0.0"),
        ("PORT", "-1"),
        ("APP_ORIGIN", "http://example.org"),
        ("APP_ORIGIN", "https://example.org/path"),
        ("APP_ORIGIN", "https://user:secret@example.org"),
        ("APP_ACCESS_KEY", "short"),
    ] {
        let old = values.insert(key.into(), value.into());
        assert!(
            Config::from_values(&values, Path::new("/tmp/branch")).is_err(),
            "{key}"
        );
        if let Some(old) = old {
            values.insert(key.into(), old);
        } else {
            values.remove(key);
        }
    }
}

#[tokio::test]
async fn private_access_redirect_sets_compatible_cookie() {
    let server = common::spawn_router(routes(common::config("https://private.example:8443"))).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let response = client
        .get(format!("{}/access?key={}", server.origin(), "a".repeat(64)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    assert_eq!(response.headers()["location"], "/");
    let cookie = response.headers()["set-cookie"].to_str().unwrap();
    for flag in [
        "branch_access=",
        "HttpOnly",
        "SameSite=Strict",
        "Secure",
        "Path=/",
        "Max-Age=604800",
    ] {
        assert!(cookie.contains(flag), "{flag}");
    }
    assert_eq!(response.headers()["cache-control"], "no-store");
    let denied = client
        .get(format!("{}/access?key=wrong", server.origin()))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 401);
}
