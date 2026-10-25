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
