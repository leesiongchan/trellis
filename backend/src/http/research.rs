use crate::{
    access::{authorize_cookie, cookie_value, now_ms},
    app::AppState,
    contracts::ResearchRequest,
    error::{AppError, ErrorCode},
    http::{guard::read_json, headers::context_id},
};
use axum::{
    Json,
    extract::{Request, State},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
pub async fn research(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let id = context_id(&request).to_owned();
    match run(state, request, &id).await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(event="research_failed",entryPoint="research_endpoint",requestId=%id,code=error.code());
            error.response(&id)
        }
    }
}
async fn run(state: Arc<AppState>, request: Request, id: &str) -> Result<Response, AppError> {
    if !authorize_cookie(
        cookie_value(request.headers()),
        state.config.access_key.expose(),
        now_ms()?,
    ) {
        return Err(AppError::new(ErrorCode::AccessRequired));
    }
    let expected_origin = state.config.origin.origin().ascii_serialization();
    if request.headers().get_all("origin").iter().count() != 1
        || request.headers().get_all("host").iter().count() != 1
        || request
            .headers()
            .get("origin")
            .and_then(|v| v.to_str().ok())
            != Some(expected_origin.as_str())
        || request.headers().get("host").and_then(|v| v.to_str().ok())
            != Some(state.config.authority())
    {
        return Err(AppError::new(ErrorCode::OriginRejected));
    }
    if state.draining.is_cancelled() {
        return Err(AppError::new(ErrorCode::ServerDraining));
    }
    let input = ResearchRequest::from_json(read_json(request, &state.config.limits).await?)?;
    if !state.config.research_configured() {
        return Err(AppError::new(ErrorCode::SetupRequired));
    }
    let mut waiter = state.jobs.admit(input)?;
    tracing::info!(
        event = "research_waiting",
        entryPoint = "research_endpoint",
        requestId = id,
        jobId = waiter.job_id()
    );
    let result = waiter.result().await.map_err(|e| (*e).clone())?;
    let mut response = Json(result.as_ref()).into_response();
    response.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store"),
    );
    Ok(response)
}
pub async fn not_found(request: Request) -> Response {
    AppError::new(ErrorCode::NotFound).response(context_id(&request))
}
pub async fn wrong_method(request: Request) -> Response {
    AppError::new(ErrorCode::MethodNotAllowed).response(context_id(&request))
}
