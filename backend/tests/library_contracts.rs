#![allow(clippy::unwrap_used, clippy::expect_used)]
use branch_backend::{
    contracts::library::{Command, CreateExploration, LibraryInfo, ResearchInput},
    storage::records::{ResolvedQuestion, canonical_hash, question_key},
};
use serde_json::{Value, json};
#[test]
fn shared_library_contracts() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/library/contracts.json")).unwrap();
    for case in cases {
        let value = case["input"].clone();
        let parsed = match case["kind"].as_str().unwrap() {
            "create" => Command::<CreateExploration>::from_json(value)
                .map(|v| serde_json::to_value(v).unwrap()),
            "research" => {
                Command::<ResearchInput>::from_json(value).map(|v| serde_json::to_value(v).unwrap())
            }
            "library" => LibraryInfo::from_json(value).map(|v| serde_json::to_value(v).unwrap()),
            _ => unreachable!(),
        };
        assert_eq!(
            parsed.is_ok(),
            case["valid"].as_bool().unwrap(),
            "{}",
            case["name"]
        );
        if let Some(expected) = case.get("normalized") {
            assert_eq!(parsed.unwrap(), *expected, "{}", case["name"]);
        }
    }
}
#[test]
fn exact_key_preserves_context() {
    let original = ResolvedQuestion::new(
        "Is volatile memory persistent?",
        "en",
        None,
        vec![],
        "literal",
    )
    .unwrap();
    let volatile_key = question_key(&original);
    let mut changed = original.clone();
    changed.wording = "Is nonvolatile memory persistent?".into();
    let nonvolatile_key = question_key(&changed);
    assert_ne!(volatile_key, nonvolatile_key);
    for (field, value) in [
        ("wording", "Is volatile memory persistent at 1 KB?"),
        ("wording", "Is volatile memory persistent at 1 MB?"),
        ("locale", "fr"),
        ("interpretation", "electrical"),
    ] {
        let mut v = serde_json::to_value(&original).unwrap();
        v[field] = json!(value);
        assert_ne!(
            volatile_key,
            question_key(&serde_json::from_value(v).unwrap())
        );
    }
    changed = original.clone();
    changed.clarification = Some("In a laptop".into());
    assert_ne!(volatile_key, question_key(&changed));
    changed = original.clone();
    changed.ancestors = vec![branch_backend::contracts::Ancestor {
        title: "Memory".into(),
        summary: "Computer hardware".into(),
    }];
    assert_ne!(volatile_key, question_key(&changed));
    changed = original.clone();
    changed.wording = "  IS\u{a0}volatile  memory persistent?\n".into();
    assert_eq!(volatile_key, question_key(&changed));
    let a = ResolvedQuestion::new("Café", "en", None, vec![], "literal").unwrap();
    let b = ResolvedQuestion::new("Cafe\u{301}", "en", None, vec![], "literal").unwrap();
    assert_eq!(question_key(&a), question_key(&b));
    assert_eq!(
        canonical_hash(&json!({"a":1,"b":{"x":2,"y":3}})),
        canonical_hash(&json!({"b":{"y":3,"x":2},"a":1}))
    );
}
