#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use crate::common::{TestServer, spawn_router};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::Value;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
#[derive(Clone)]
pub struct Script {
    pub status: StatusCode,
    pub body: Vec<u8>,
    pub delay: Duration,
    pub location: Option<String>,
}
impl Script {
    pub fn json(body: Value) -> Self {
        Self {
            status: StatusCode::OK,
            body: serde_json::to_vec(&body).unwrap(),
            delay: Duration::ZERO,
            location: None,
        }
    }
}
#[derive(Default)]
pub struct StubState {
    pub scripts: Mutex<VecDeque<Script>>,
    pub requests: Mutex<Vec<(HeaderMap, Value)>>,
}
pub struct StubProviders {
    pub server: TestServer,
    pub state: Arc<StubState>,
}
impl StubProviders {
    pub async fn start(scripts: Vec<Script>) -> Self {
        let state = Arc::new(StubState {
            scripts: Mutex::new(scripts.into()),
            ..Default::default()
        });
        let server = spawn_router(
            Router::new()
                .route("/", post(answer))
                .with_state(state.clone()),
        )
        .await;
        Self { server, state }
    }
    pub fn url(&self) -> url::Url {
        url::Url::parse(&self.server.origin()).unwrap()
    }
    pub fn count(&self) -> usize {
        self.state.requests.lock().unwrap().len()
    }
}
async fn answer(
    State(state): State<Arc<StubState>>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> Response {
    state.requests.lock().unwrap().push((headers, input));
    let script = state
        .scripts
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or_else(|| Script {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            ..Script::json(Value::Null)
        });
    tokio::time::sleep(script.delay).await;
    let chunks: Vec<_> = script
        .body
        .chunks(8192)
        .map(|s| Ok::<_, std::io::Error>(Bytes::copy_from_slice(s)))
        .collect();
    let mut response = (
        script.status,
        Body::from_stream(futures_util::stream::iter(chunks)),
    )
        .into_response();
    response.headers_mut().insert(
        "content-type",
        axum::http::HeaderValue::from_static("application/json"),
    );
    if let Some(location) = script.location {
        response
            .headers_mut()
            .insert("location", location.parse().unwrap());
    }
    response
}
