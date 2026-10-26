//! Legacy clients must reload before research can establish authoritative saved content.
use crate::{
    app::AppState,
    error::{AppError, ErrorCode},
    http::{guard, headers::context_id},
};
use axum::{
    extract::{Request, State},
    response::Response,
};
use std::sync::Arc;
pub async fn research(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let error = guard::authorize(&state, request.headers(), true)
        .err()
        .unwrap_or_else(|| AppError::new(ErrorCode::ReloadRequired));
    error.response(context_id(&request))
}
pub async fn not_found(request: Request) -> Response {
    AppError::new(ErrorCode::NotFound).response(context_id(&request))
}
pub async fn wrong_method(request: Request) -> Response {
    AppError::new(ErrorCode::MethodNotAllowed).response(context_id(&request))
}
