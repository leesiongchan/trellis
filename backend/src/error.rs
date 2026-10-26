//! Safe errors; raw upstream text never crosses this boundary.
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    AccessRequired,
    OriginRejected,
    InvalidContentType,
    BodyTooLarge,
    BodyTimeout,
    InvalidJson,
    InvalidQuestion,
    SetupRequired,
    ResearchLimit,
    ResearchTimeout,
    ProviderLimit,
    SearchFailed,
    InvalidExplanation,
    GroundingFailed,
    InvalidEvidence,
    UpstreamFailure,
    NotFound,
    MethodNotAllowed,
    ServerDraining,
    InternalFailure,
    InvalidConfig,
    StorageUnavailable,
    StorageBusy,
    RevisionConflict,
    IntentConflict,
    WorkInterrupted,
    InvalidImport,
    ReloadRequired,
}
#[derive(Debug, Clone, thiserror::Error)]
#[error("{code}: {message}", code = self.code(), message = self.message())]
pub struct AppError {
    pub kind: ErrorCode,
}
impl AppError {
    pub fn new(kind: ErrorCode) -> Self {
        Self { kind }
    }
    pub fn invalid_question() -> Self {
        Self::new(ErrorCode::InvalidQuestion)
    }
    pub fn code(&self) -> &'static str {
        self.details().0
    }
    pub fn message(&self) -> &'static str {
        self.details().1
    }
    pub fn status(&self) -> StatusCode {
        self.details().2
    }
    pub fn retryable(&self) -> bool {
        self.details().3
    }
    fn details(&self) -> (&'static str, &'static str, StatusCode, bool) {
        use ErrorCode::*;
        match self.kind {
            AccessRequired => (
                "ACCESS_REQUIRED",
                "Open your private access link to enable research.",
                StatusCode::UNAUTHORIZED,
                false,
            ),
            OriginRejected => (
                "ORIGIN_REJECTED",
                "This request must come from your private app address.",
                StatusCode::FORBIDDEN,
                false,
            ),
            InvalidContentType => (
                "INVALID_CONTENT_TYPE",
                "Send a JSON request.",
                StatusCode::BAD_REQUEST,
                false,
            ),
            BodyTooLarge => (
                "BODY_TOO_LARGE",
                "The question and context are too large.",
                StatusCode::PAYLOAD_TOO_LARGE,
                false,
            ),
            BodyTimeout => (
                "BODY_TIMEOUT",
                "The request body took too long to arrive.",
                StatusCode::REQUEST_TIMEOUT,
                true,
            ),
            InvalidJson => (
                "INVALID_JSON",
                "The request could not be read. Please try again.",
                StatusCode::BAD_REQUEST,
                false,
            ),
            InvalidQuestion => (
                "INVALID_QUESTION",
                "Enter a question of 1–500 characters with a shorter context.",
                StatusCode::UNPROCESSABLE_ENTITY,
                false,
            ),
            SetupRequired => (
                "SETUP_REQUIRED",
                "Live research needs OPENAI_API_KEY and TAVILY_API_KEY in the server’s .env file. Saved explorations remain available.",
                StatusCode::SERVICE_UNAVAILABLE,
                false,
            ),
            ResearchLimit => (
                "RESEARCH_LIMIT",
                "A few topics are already being researched. Try again shortly.",
                StatusCode::TOO_MANY_REQUESTS,
                true,
            ),
            ResearchTimeout => (
                "RESEARCH_TIMEOUT",
                "Research took too long. Please retry.",
                StatusCode::GATEWAY_TIMEOUT,
                true,
            ),
            ProviderLimit => (
                "PROVIDER_LIMIT",
                "The research service is busy. Please try again later.",
                StatusCode::TOO_MANY_REQUESTS,
                true,
            ),
            SearchFailed => (
                "SEARCH_FAILED",
                "The web research service could not complete this search. Please try again later.",
                StatusCode::BAD_GATEWAY,
                true,
            ),
            InvalidExplanation => (
                "INVALID_EXPLANATION",
                "The explanation could not be completed. Please retry.",
                StatusCode::BAD_GATEWAY,
                true,
            ),
            GroundingFailed => (
                "GROUNDING_FAILED",
                "The source check could not finish. Please retry.",
                StatusCode::BAD_GATEWAY,
                true,
            ),
            InvalidEvidence => (
                "INVALID_EVIDENCE",
                "A source could not be verified.",
                StatusCode::BAD_GATEWAY,
                true,
            ),
            UpstreamFailure => (
                "UPSTREAM_FAILURE",
                "Research could not finish. Please try again shortly.",
                StatusCode::BAD_GATEWAY,
                true,
            ),
            NotFound => (
                "NOT_FOUND",
                "This endpoint does not exist.",
                StatusCode::NOT_FOUND,
                false,
            ),
            MethodNotAllowed => (
                "METHOD_NOT_ALLOWED",
                "This method is not supported.",
                StatusCode::METHOD_NOT_ALLOWED,
                false,
            ),
            ServerDraining => (
                "SERVER_DRAINING",
                "The service is restarting. Please try again shortly.",
                StatusCode::SERVICE_UNAVAILABLE,
                true,
            ),
            InternalFailure => (
                "INTERNAL_FAILURE",
                "Research could not finish. Please try again shortly.",
                StatusCode::INTERNAL_SERVER_ERROR,
                true,
            ),
            StorageUnavailable => (
                "STORAGE_UNAVAILABLE",
                "The saved library is temporarily unavailable.",
                StatusCode::SERVICE_UNAVAILABLE,
                true,
            ),
            StorageBusy => (
                "STORAGE_BUSY",
                "The saved library is busy. Try again shortly.",
                StatusCode::SERVICE_UNAVAILABLE,
                true,
            ),
            RevisionConflict => (
                "REVISION_CONFLICT",
                "The library changed. Reload it before trying again.",
                StatusCode::CONFLICT,
                false,
            ),
            IntentConflict => (
                "INTENT_CONFLICT",
                "This operation ID was already used for another request.",
                StatusCode::CONFLICT,
                false,
            ),
            WorkInterrupted => (
                "WORK_INTERRUPTED",
                "Research was interrupted. Retry explicitly to continue.",
                StatusCode::CONFLICT,
                false,
            ),
            InvalidImport => (
                "INVALID_IMPORT",
                "The saved exploration could not be imported. Keep the browser copy and export it for recovery.",
                StatusCode::UNPROCESSABLE_ENTITY,
                false,
            ),
            ReloadRequired => (
                "RELOAD_REQUIRED",
                "Reload the app to use the current library protocol.",
                StatusCode::UPGRADE_REQUIRED,
                false,
            ),
            InvalidConfig => (
                "INVALID_CONFIG",
                "Server configuration is invalid. Check the documented settings.",
                StatusCode::INTERNAL_SERVER_ERROR,
                false,
            ),
        }
    }
    pub fn response(&self, request_id: &str) -> Response {
        let mut response = (self.status(), Json(json!({"error": {"code": self.code(), "message": self.message(), "retryable": self.retryable(), "requestId": request_id}}))).into_response();
        response.headers_mut().insert(
            "cache-control",
            axum::http::HeaderValue::from_static("no-store"),
        );
        response
    }
}
