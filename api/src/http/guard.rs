use crate::{
    config::Limits,
    error::{AppError, ErrorCode},
};
use axum::extract::Request;
use http_body_util::BodyExt;
pub async fn read_json(request: Request, limits: &Limits) -> Result<serde_json::Value, AppError> {
    let content_type = request
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .map(str::trim);
    if !content_type.is_some_and(|v| v.ends_with("/json")) {
        return Err(AppError::new(ErrorCode::InvalidContentType));
    }
    if request
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .is_some_and(|size| size > limits.body_bytes)
    {
        return Err(AppError::new(ErrorCode::BodyTooLarge));
    }
    let operation = async {
        let mut body = request.into_body();
        let mut bytes = Vec::new();
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|_| AppError::new(ErrorCode::InvalidJson))?;
            if let Ok(data) = frame.into_data() {
                if data.len() > limits.body_bytes.saturating_sub(bytes.len()) {
                    return Err(AppError::new(ErrorCode::BodyTooLarge));
                }
                bytes.extend_from_slice(&data);
            }
        }
        serde_json::from_slice(&bytes).map_err(|_| AppError::new(ErrorCode::InvalidJson))
    };
    tokio::time::timeout(limits.body_read, operation)
        .await
        .map_err(|_| AppError::new(ErrorCode::BodyTimeout))?
}

pub(crate) fn authorize(
    state: &crate::app::AppState,
    headers: &axum::http::HeaderMap,
    mutation: bool,
) -> Result<(), AppError> {
    use crate::access::{authorize_cookie, cookie_value, now_ms};
    if !authorize_cookie(
        cookie_value(headers),
        state.config.access_key.expose(),
        now_ms()?,
    ) {
        return Err(AppError::new(ErrorCode::AccessRequired));
    }
    if mutation {
        let expected = state.config.origin.origin().ascii_serialization();
        if headers.get_all("origin").iter().count() != 1
            || headers.get_all("host").iter().count() != 1
            || headers.get("origin").and_then(|v| v.to_str().ok()) != Some(expected.as_str())
            || headers.get("host").and_then(|v| v.to_str().ok()) != Some(state.config.authority())
        {
            return Err(AppError::new(ErrorCode::OriginRejected));
        }
    }
    Ok(())
}
pub(crate) fn library_scope(
    state: &crate::app::AppState,
    headers: &axum::http::HeaderMap,
    mutation: bool,
) -> Result<(), AppError> {
    authorize(state, headers, mutation)?;
    if headers.get_all("x-branch-protocol").iter().count() != 1
        || headers
            .get("x-branch-protocol")
            .and_then(|v| v.to_str().ok())
            != Some("2")
    {
        return Err(AppError::new(ErrorCode::ReloadRequired));
    }
    let count = headers.get_all("x-branch-library").iter().count();
    if count > 0 || mutation {
        let supplied = headers
            .get("x-branch-library")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::new(ErrorCode::RevisionConflict))?;
        if count != 1 || state.library_id.as_str() != supplied {
            return Err(AppError::new(ErrorCode::RevisionConflict));
        }
    }
    if mutation && state.draining.is_cancelled() {
        return Err(AppError::new(ErrorCode::ServerDraining));
    }
    Ok(())
}
