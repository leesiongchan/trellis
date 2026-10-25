#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::contracts::{ResearchRequest, ResearchResult, public_source_url, utf16_len};
use serde_json::{Value, json};

#[test]
fn shared_requests_match_browser() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/contracts/requests.json")).unwrap();
    for sample in cases {
        let result = ResearchRequest::from_json(sample["input"].clone());
        assert_eq!(
            result.is_ok(),
            sample["valid"].as_bool().unwrap(),
            "{}",
            sample["name"]
        );
        if let Some(expected) = sample.get("normalized") {
            assert_eq!(
                serde_json::to_value(result.unwrap()).unwrap(),
                *expected,
                "{}",
                sample["name"]
            );
        }
    }
}

#[test]
fn utf16_and_source_validation_match_browser_boundaries() {
    assert_eq!(utf16_len("😀"), 2);
    assert!(public_source_url("https://example.org/reference"));
    for url in [
        "javascript:alert(1)",
        "https://localhost/",
        "https://10.1.2.3/",
        "https://[::1]/",
        "https://user:pass@example.org",
        "https://example.org:444/",
    ] {
        assert!(!public_source_url(url), "{url}");
    }
}

#[test]
fn required_nullable_field_cannot_be_omitted() {
    assert!(
        ResearchRequest::from_json(
            json!({"explorationId":"e", "nodeId":"n", "question":"Q", "ancestors":[]})
        )
        .is_err()
    );
}

#[test]
fn shared_results_match_browser() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/contracts/results.json")).unwrap();
    for sample in cases {
        let result = ResearchResult::from_json(sample["input"].clone());
        assert_eq!(
            result.is_ok(),
            sample["valid"].as_bool().unwrap(),
            "{}",
            sample["name"]
        );
        if let Some(expected) = sample.get("normalized") {
            assert_eq!(serde_json::to_value(result.unwrap()).unwrap(), *expected);
        }
    }
}
