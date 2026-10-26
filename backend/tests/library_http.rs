#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
#[path = "support/saved_provider.rs"]
mod fixture;
use branch_backend::{
    access::issue_cookie,
    app::{AppState, api_router},
    contracts::{legacy::LegacyExploration, library::*},
    telemetry::Telemetry,
};
use serde_json::{Value, json};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::io::AsyncWriteExt;
struct Setup {
    server: common::TestServer,
    state: Arc<AppState>,
    provider: Arc<fixture::Provider>,
    library: String,
}
impl Drop for Setup {
    fn drop(&mut self) {
        self.state.draining.cancel();
        self.state.jobs.cancel_all();
    }
}
impl Setup {
    async fn new(configured: bool) -> Self {
        let (config, data) = common::config("http://127.0.0.1:3000");
        let mut config = (*config).clone();
        config.limits.body_read = Duration::from_millis(50);
        if !configured {
            config.openai_key = Default::default();
            config.tavily_key = Default::default();
        }
        let provider = fixture::Provider::new();
        let state = AppState::start(
            Arc::new(config),
            provider.clone(),
            None,
            Arc::new(Telemetry::new()),
        )
        .await
        .unwrap();
        let library = state
            .store
            .library_info()
            .await
            .unwrap()
            .library_id
            .to_string();
        let server = common::spawn_router(api_router(state.clone()))
            .await
            .keep_data_directory(data);
        Self {
            server,
            state,
            provider,
            library,
        }
    }
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        reqwest::Client::new()
            .request(method, format!("{}{path}", self.server.origin()))
            .header("cookie", cookie())
            .header("host", "127.0.0.1:3000")
            .header("origin", "http://127.0.0.1:3000")
            .header("x-branch-protocol", "2")
            .header("x-branch-library", &self.library)
    }
    async fn create(&self, question: &str) -> (ExplorationId, AppearanceId, TopicId) {
        let op = self
            .state
            .store
            .create_exploration(&command(
                CreateExploration {
                    question: question.into(),
                    locale: "en".into(),
                },
                None,
            ))
            .await
            .unwrap();
        let Some(OperationResult::Exploration {
            exploration_id,
            appearance_id,
            ..
        }) = op.result
        else {
            unreachable!()
        };
        let topic = self
            .state
            .store
            .exploration(&exploration_id)
            .await
            .unwrap()
            .appearances[0]
            .topic_id
            .clone();
        (exploration_id, appearance_id, topic)
    }
}
fn cookie() -> String {
    format!(
        "branch_access={}",
        issue_cookie(&"a".repeat(64), 4102444800000).unwrap()
    )
}
fn command<T>(payload: T, revision: Option<u64>) -> Command<T> {
    Command {
        operation_id: OperationId::generate(),
        library_epoch: 1,
        expected_revision: revision,
        payload,
    }
}
async fn error(response: reqwest::Response, status: u16, code: &str) {
    assert_eq!(response.status().as_u16(), status);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-branch-protocol"], "2");
    let value: Value = response.json().await.unwrap();
    assert_eq!(value["error"]["code"], code);
    assert!(value["error"]["requestId"].is_string());
}
async fn changed_header(
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
#[tokio::test]
async fn library_requires_access_on_reads_and_writes() {
    let s = Setup::new(true).await;
    for path in [
        "/api/library",
        "/api/explorations",
        "/api/topics/unknown?answerVersionId=unknown",
        "/api/operations/unknown",
        "/api/imports/unknown",
    ] {
        let anonymous_read =
            changed_header(s.request(reqwest::Method::GET, path), "cookie", "").await;
        assert_eq!(anonymous_read.status(), reqwest::StatusCode::UNAUTHORIZED);
        error(anonymous_read, 401, "ACCESS_REQUIRED").await;
        error(
            changed_header(
                s.request(reqwest::Method::GET, path),
                "cookie",
                &format!(
                    "branch_access={}",
                    issue_cookie(&"a".repeat(64), 1).unwrap()
                ),
            )
            .await,
            401,
            "ACCESS_REQUIRED",
        )
        .await;
    }
    error(
        changed_header(
            s.request(reqwest::Method::POST, "/api/explorations")
                .json(&command(
                    CreateExploration {
                        question: "Question".into(),
                        locale: "en".into(),
                    },
                    None,
                )),
            "cookie",
            "",
        )
        .await,
        401,
        "ACCESS_REQUIRED",
    )
    .await;
    assert_eq!(s.provider.searches.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn mutation_scope_origin_protocol_and_revision_are_checked() {
    let s = Setup::new(true).await;
    let (e, a, _) = s.create("Root").await;
    let input = command(
        AddBranch {
            parent_appearance_id: a.clone(),
            question: "Child".into(),
        },
        Some(0),
    );
    let path = format!("/api/explorations/{e}/appearances");
    for (name, value, status, code) in [
        ("origin", "https://evil.example", 403, "ORIGIN_REJECTED"),
        ("host", "evil.example", 403, "ORIGIN_REJECTED"),
        ("origin", "", 403, "ORIGIN_REJECTED"),
        ("x-branch-protocol", "1", 426, "RELOAD_REQUIRED"),
        (
            "x-branch-library",
            "another-installation",
            409,
            "REVISION_CONFLICT",
        ),
    ] {
        error(
            changed_header(
                s.request(reqwest::Method::POST, &path).json(&input),
                name,
                value,
            )
            .await,
            status,
            code,
        )
        .await;
    }
    let mut stale = input.clone();
    stale.expected_revision = Some(99);
    let stale_revision = s
        .request(reqwest::Method::POST, &path)
        .json(&stale)
        .send()
        .await
        .unwrap();
    assert_eq!(stale_revision.status(), reqwest::StatusCode::CONFLICT);
    error(stale_revision, 409, "REVISION_CONFLICT").await;
    let (_, foreign, _) = s.create("Other exploration").await;
    let foreign = command(
        AddBranch {
            parent_appearance_id: foreign,
            question: "Wrong tree".into(),
        },
        Some(0),
    );
    error(
        s.request(reqwest::Method::POST, &path)
            .json(&foreign)
            .send()
            .await
            .unwrap(),
        404,
        "NOT_FOUND",
    )
    .await;
    let mismatch = command(
        ResearchInput {
            appearance_id: a,
            refresh: false,
            clarification: None,
        },
        Some(0),
    );
    error(
        s.request(reqwest::Method::POST, "/api/appearances/different/research")
            .json(&mismatch)
            .send()
            .await
            .unwrap(),
        404,
        "NOT_FOUND",
    )
    .await;
    assert_eq!(s.provider.searches.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn bound_question_text_and_saved_topic_navigation_preserve_context() {
    let s = Setup::new(true).await;
    let text = "A question'; DELETE topic; --";
    let created = s
        .request(reqwest::Method::POST, "/api/explorations")
        .json(&command(
            CreateExploration {
                question: text.into(),
                locale: "en".into(),
            },
            None,
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), 200);
    let op: OperationView = created.json().await.unwrap();
    let Some(OperationResult::Exploration {
        exploration_id: e,
        appearance_id: a,
        ..
    }) = op.result
    else {
        unreachable!()
    };
    let added = s
        .request(
            reqwest::Method::POST,
            &format!("/api/explorations/{e}/appearances"),
        )
        .json(&command(
            AddBranch {
                parent_appearance_id: a,
                question: "Branch question".into(),
            },
            Some(0),
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(added.status(), 200);
    let snapshot: ExplorationSnapshot = s
        .request(reqwest::Method::GET, &format!("/api/explorations/{e}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let root = snapshot
        .appearances
        .iter()
        .find(|a| a.parent_id.is_none())
        .unwrap();
    assert_eq!(root.question, text);
    let child = snapshot
        .appearances
        .iter()
        .find(|a| a.parent_id.is_some())
        .unwrap();
    let topics: Value = s
        .request(reqwest::Method::GET, "/api/topics?limit=100")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(topics["items"].as_array().unwrap().len(), 2);
    let locations: Value = s
        .request(
            reqwest::Method::GET,
            &format!("/api/topics/{}/appearances", child.topic_id),
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(locations["items"][0]["appearanceId"], child.id.as_str());
    let started:OperationView=s.request(reqwest::Method::POST,&format!("/api/topics/{}/explorations",child.topic_id)).json(&json!({"operationId":OperationId::generate(),"libraryEpoch":1,"expectedRevision":null,"payload":{"topicId":child.topic_id,"answerVersionId":null}})).send().await.unwrap().json().await.unwrap();
    let Some(OperationResult::Exploration {
        exploration_id: new,
        ..
    }) = started.result
    else {
        unreachable!()
    };
    let copy: ExplorationSnapshot = s
        .request(reqwest::Method::GET, &format!("/api/explorations/{new}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(copy.appearances[0].topic_id, child.topic_id);
    assert_eq!(
        serde_json::to_value(&copy.appearances[0].ancestors).unwrap(),
        serde_json::to_value(&child.ancestors).unwrap()
    );
    assert_eq!(s.provider.searches.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn research_replay_after_disconnect_is_durable_and_has_a_poll_location() {
    let s = Setup::new(true).await;
    let (e, a, _) = s.create("Explain NAND flash").await;
    let input = command(
        ResearchInput {
            appearance_id: a,
            refresh: false,
            clarification: None,
        },
        Some(0),
    );
    let body = serde_json::to_string(&input).unwrap();
    let path = format!("/api/appearances/{}/research", input.payload.appearance_id);
    let mut socket = tokio::net::TcpStream::connect(s.server.address)
        .await
        .unwrap();
    socket.write_all(format!("POST {path} HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nOrigin: http://127.0.0.1:3000\r\nCookie: {}\r\nX-Branch-Protocol: 2\r\nX-Branch-Library: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",cookie(),s.library,body.len()).as_bytes()).await.unwrap();
    drop(socket);
    tokio::time::timeout(Duration::from_secs(5), async {
        while s.provider.searches.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let replay = s
        .request(reqwest::Method::POST, &path)
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), 202);
    assert_eq!(
        replay.headers()["location"],
        format!("/api/operations/{}", input.operation_id)
    );
    s.provider.release.add_permits(1);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let op: OperationView = s
                .request(
                    reqwest::Method::GET,
                    &format!("/api/operations/{}", input.operation_id),
                )
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if op.state == OperationState::Completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        s.request(reqwest::Method::POST, &path)
            .json(&input)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(s.provider.searches.load(Ordering::SeqCst), 1);
    assert_eq!(
        s.state.store.exploration(&e).await.unwrap().answers.len(),
        1
    );
}
#[tokio::test]
async fn legacy_research_requires_reload_without_provider_work() {
    let s = Setup::new(true).await;
    error(s.request(reqwest::Method::POST,"/api/research").json(&json!({"explorationId":"old","nodeId":"old","question":"Old browser question","clarification":null,"ancestors":[]})).send().await.unwrap(),426,"RELOAD_REQUIRED").await;
    let legacy_provider_calls = s.provider.searches.load(Ordering::SeqCst);
    assert_eq!(legacy_provider_calls, 0);
}
#[tokio::test]
async fn import_and_purge_are_scoped_and_old_commands_cannot_resurrect_content() {
    let s = Setup::new(true).await;
    let raw: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/library/migration.json")).unwrap();
    let legacy = LegacyExploration::validate(raw.clone()).unwrap();
    let encoded = raw.to_string();
    let v: ImportView = s
        .request(reqwest::Method::POST, "/api/imports")
        .json(&command(
            BeginImport {
                legacy_exploration_id: legacy.id.clone(),
                content_digest: legacy.digest(),
                expected_bytes: encoded.len() as u64,
                expected_nodes: 3,
                explicit_restore: false,
            },
            None,
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v.state, ImportState::Staging);
    assert_eq!(
        s.request(
            reqwest::Method::POST,
            &format!("/api/imports/{}/batches", v.id)
        )
        .json(&command(
            ImportBatch {
                import_id: v.id.clone(),
                batch_index: 0,
                records: vec![json!(encoded)]
            },
            None
        ))
        .send()
        .await
        .unwrap()
        .status(),
        200
    );
    let done: ImportView = s
        .request(
            reqwest::Method::POST,
            &format!("/api/imports/{}/completion", v.id),
        )
        .json(&command(FinalizeImport { import_id: v.id }, None))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(done.state, ImportState::Completed);
    let purge = command(
        PurgeLibrary {
            confirmation: PURGE_CONFIRMATION.into(),
        },
        None,
    );
    error(
        changed_header(
            s.request(reqwest::Method::POST, "/api/library/purge")
                .json(&purge),
            "x-branch-library",
            "another-installation",
        )
        .await,
        409,
        "REVISION_CONFLICT",
    )
    .await;
    assert_eq!(
        s.state
            .store
            .explorations(None, 100)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    let purged: OperationView = s
        .request(reqwest::Method::POST, "/api/library/purge")
        .json(&purge)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(matches!(
        purged.result,
        Some(OperationResult::Purge { library_epoch: 2 })
    ));
    error(
        s.request(reqwest::Method::POST, "/api/explorations")
            .json(&command(
                CreateExploration {
                    question: "Stale copy".into(),
                    locale: "en".into(),
                },
                None,
            ))
            .send()
            .await
            .unwrap(),
        409,
        "REVISION_CONFLICT",
    )
    .await;
    assert!(
        s.state
            .store
            .explorations(None, 100)
            .await
            .unwrap()
            .items
            .is_empty()
    );
}
#[tokio::test]
async fn read_only_saved_access_and_free_reuse_work_without_provider_configuration() {
    let s = Setup::new(false).await;
    let raw: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/library/migration.json")).unwrap();
    let legacy = LegacyExploration::validate(raw.clone()).unwrap();
    let encoded = raw.to_string();
    let v = s
        .state
        .store
        .begin_import(&command(
            BeginImport {
                legacy_exploration_id: legacy.id.clone(),
                content_digest: legacy.digest(),
                expected_bytes: encoded.len() as u64,
                expected_nodes: 3,
                explicit_restore: false,
            },
            None,
        ))
        .await
        .unwrap();
    s.state
        .store
        .stage_import(&command(
            ImportBatch {
                import_id: v.id.clone(),
                batch_index: 0,
                records: vec![json!(encoded)],
            },
            None,
        ))
        .await
        .unwrap();
    let done = s
        .state
        .store
        .finalize_import(&command(FinalizeImport { import_id: v.id }, None))
        .await
        .unwrap();
    let a = AppearanceId::new(done.mapping["root"].clone()).unwrap();
    assert_eq!(
        s.request(
            reqwest::Method::GET,
            &format!("/api/explorations/{}", done.exploration_id.unwrap())
        )
        .send()
        .await
        .unwrap()
        .status(),
        200
    );
    let op: OperationView = s
        .request(
            reqwest::Method::POST,
            &format!("/api/appearances/{a}/research"),
        )
        .json(&command(
            ResearchInput {
                appearance_id: a.clone(),
                refresh: false,
                clarification: None,
            },
            Some(0),
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(op.state, OperationState::Completed);
    let Some(OperationResult::Research { revision, .. }) = op.result else {
        unreachable!()
    };
    error(
        s.request(
            reqwest::Method::POST,
            &format!("/api/appearances/{a}/research"),
        )
        .json(&command(
            ResearchInput {
                appearance_id: a,
                refresh: true,
                clarification: None,
            },
            Some(revision),
        ))
        .send()
        .await
        .unwrap(),
        503,
        "SETUP_REQUIRED",
    )
    .await;
    assert_eq!(s.provider.searches.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn request_and_graph_bounds_fail_safely_and_no_database_endpoint_exists() {
    let s = Setup::new(true).await;
    let (_, _, topic) = s.create("Graph root").await;
    for query in ["hops=4", "nodes=101", "edges=201", "hops=-1"] {
        error(
            s.request(
                reqwest::Method::GET,
                &format!("/api/topics/{topic}/neighborhood?{query}"),
            )
            .send()
            .await
            .unwrap(),
            422,
            "INVALID_QUESTION",
        )
        .await;
    }
    for path in [
        "/api/explorations?limit=101",
        "/api/explorations?cursor=not-a-cursor",
        "/api/topics?limit=0",
    ] {
        error(
            s.request(reqwest::Method::GET, path).send().await.unwrap(),
            422,
            "INVALID_QUESTION",
        )
        .await;
    }
    error(
        s.request(reqwest::Method::POST, "/api/explorations")
            .json(&command(
                CreateExploration {
                    question: "x".repeat(65537),
                    locale: "en".into(),
                },
                None,
            ))
            .send()
            .await
            .unwrap(),
        413,
        "BODY_TOO_LARGE",
    )
    .await;
    for path in ["/api/query", "/api/database", "/api/storage"] {
        assert_eq!(
            s.request(reqwest::Method::GET, path)
                .send()
                .await
                .unwrap()
                .status(),
            404
        );
    }
    s.state.store.close().await.unwrap();
    error(
        s.request(reqwest::Method::GET, "/api/library")
            .send()
            .await
            .unwrap(),
        503,
        "STORAGE_UNAVAILABLE",
    )
    .await;
}

#[tokio::test]
async fn only_import_batches_accept_large_bodies() {
    let s = Setup::new(true).await;
    let v: ImportView = s
        .request(reqwest::Method::POST, "/api/imports")
        .json(&command(
            BeginImport {
                legacy_exploration_id: "large-fixture".into(),
                content_digest: "a".repeat(64),
                expected_bytes: 100_000,
                expected_nodes: 1,
                explicit_restore: false,
            },
            None,
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let path = format!("/api/imports/{}/batches", v.id);
    let response = s
        .request(reqwest::Method::POST, &path)
        .json(&command(
            ImportBatch {
                import_id: v.id.clone(),
                batch_index: 0,
                records: vec![json!("x".repeat(100_000))],
            },
            None,
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    error(
        s.request(reqwest::Method::POST, &path)
            .json(&command(
                ImportBatch {
                    import_id: v.id,
                    batch_index: 1,
                    records: vec![json!("x".repeat(2 * 1024 * 1024))],
                },
                None,
            ))
            .send()
            .await
            .unwrap(),
        413,
        "BODY_TOO_LARGE",
    )
    .await;
}
