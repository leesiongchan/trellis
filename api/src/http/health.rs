use crate::{
    access::{authorize_cookie, cookie_value, now_ms},
    app::AppState,
    error::{AppError, ErrorCode},
    http::headers::context_id,
};
use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response
}
pub async fn live() -> Response {
    no_store(Json(serde_json::json!({"status":"live"})).into_response())
}
pub async fn ready(State(state): State<Arc<AppState>>) -> Response {
    let knowledge = state.knowledge_runtime.status().await;
    state.telemetry.knowledge_status(&knowledge);
    let ready =
        !state.draining.is_cancelled() && knowledge.storage_ready && knowledge.library_ready;
    no_store((if ready{StatusCode::OK}else{StatusCode::SERVICE_UNAVAILABLE},Json(serde_json::json!({"ready":ready,"researchConfigured":state.config.research_configured(),"storageReady":knowledge.storage_ready,"libraryReady":knowledge.library_ready,"discovery":knowledge.discovery}))).into_response())
}
pub async fn metrics(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let id = context_id(&request);
    if !authorize_cookie(
        cookie_value(request.headers()),
        state.config.access_key.expose(),
        now_ms().unwrap_or(0),
    ) {
        return AppError::new(ErrorCode::AccessRequired).response(id);
    }
    state
        .telemetry
        .knowledge_status(&state.knowledge_runtime.status().await);
    match state.telemetry.render() {
        Ok(text) => {
            let mut response = no_store(text.into_response());
            response.headers_mut().insert(
                "content-type",
                HeaderValue::from_static(
                    "application/openmetrics-text; version=1.0.0; charset=utf-8",
                ),
            );
            response
        }
        Err(error) => error.response(id),
    }
}
