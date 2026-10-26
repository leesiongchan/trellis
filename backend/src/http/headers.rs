use crate::{
    app::AppState,
    error::{AppError, ErrorCode},
};
use axum::{
    extract::{Request, State},
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};
use std::{sync::Arc, time::Instant};
use tracing::Instrument;
pub const BASE_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; object-src 'none'";
#[derive(Clone)]
pub struct RequestContext {
    pub id: String,
}
pub fn request_id(request: &Request) -> String {
    request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| {
            !v.is_empty()
                && v.len() <= 100
                && v.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string())
}
pub fn route_name(path: &str) -> &'static str {
    match path {
        "/api/library" => "/api/library",
        "/api/library/purge" => "/api/library/purge",
        "/api/explorations" => "/api/explorations",
        "/api/topics" => "/api/topics",
        "/api/imports" => "/api/imports",
        "/api/research" => "/api/research",
        "/access" => "/access",
        "/health/live" => "/health/live",
        "/health/ready" => "/health/ready",
        "/metrics" => "/metrics",
        "/" => "/",
        _ if path.starts_with("/_app/") => "/_app/*",
        _ => match path.split('/').collect::<Vec<_>>().as_slice() {
            ["", "api", "explorations", _] => "/api/explorations/{id}",
            ["", "api", "explorations", _, "appearances"] => "/api/explorations/{id}/appearances",
            ["", "api", "appearances", _, "research"] => "/api/appearances/{id}/research",
            ["", "api", "appearances", _, "answer"] => "/api/appearances/{id}/answer",
            ["", "api", "topics", _] => "/api/topics/{id}",
            ["", "api", "topics", _, "connections"] => "/api/topics/{id}/connections",
            ["", "api", "topics", _, "neighborhood"] => "/api/topics/{id}/neighborhood",
            ["", "api", "topics", _, "appearances"] => "/api/topics/{id}/appearances",
            ["", "api", "topics", _, "explorations"] => "/api/topics/{id}/explorations",
            ["", "api", "connections", _] => "/api/connections/{id}",
            ["", "api", "operations", _] => "/api/operations/{id}",
            ["", "api", "imports", _] => "/api/imports/{id}",
            ["", "api", "imports", _, "batches"] => "/api/imports/{id}/batches",
            ["", "api", "imports", _, "completion"] => "/api/imports/{id}/completion",
            _ => "unknown",
        },
    }
}
pub async fn boundary(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let id = request_id(&request);
    let route = route_name(request.uri().path());
    let library_api = request.uri().path().starts_with("/api/");
    let method = request.method().clone();
    let started = Instant::now();
    request
        .extensions_mut()
        .insert(RequestContext { id: id.clone() });
    for name in [header::AUTHORIZATION, header::COOKIE] {
        if let Some(value) = request.headers_mut().get_mut(name) {
            value.set_sensitive(true);
        }
    }
    let span = tracing::info_span!("http_request",requestId=%id,entryPoint="http",route);
    tracing::info!(parent:&span,event="http_started");
    let mut response = next.run(request).instrument(span.clone()).await;
    if response.status().is_server_error() {
        tracing::warn!(parent:&span,event="http_failed",status=response.status().as_u16());
    }
    state
        .telemetry
        .http(route, &method, response.status(), started.elapsed());
    tracing::info!(parent:&span,event="http_completed",status=response.status().as_u16(),durationMs=started.elapsed().as_millis() as u64);
    let headers = response.headers_mut();
    if library_api {
        headers.insert("x-branch-protocol", HeaderValue::from_static("2"));
        headers.insert("cache-control", HeaderValue::from_static("no-store"));
    }
    if let Ok(id) = HeaderValue::from_str(&id) {
        headers.insert("x-request-id", id);
    }
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    if !headers.contains_key("content-security-policy") {
        headers.insert(
            "content-security-policy",
            HeaderValue::from_static(BASE_CSP),
        );
    }
    if state.config.origin.scheme() == "https" {
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000"),
        );
    }
    response
}
pub fn context_id(request: &Request) -> &str {
    request
        .extensions()
        .get::<RequestContext>()
        .map(|c| c.id.as_str())
        .unwrap_or("internal")
}
pub fn internal_error() -> AppError {
    AppError::new(ErrorCode::InternalFailure)
}
