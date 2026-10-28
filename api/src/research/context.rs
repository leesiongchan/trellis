use crate::error::{AppError, ErrorCode};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
pub struct JobContext {
    pub deadline: Instant,
    pub cancel: CancellationToken,
    pub job_id: String,
    pub telemetry: Option<std::sync::Arc<crate::telemetry::Telemetry>>,
}
impl JobContext {
    pub fn check(&self) -> Result<(), AppError> {
        if self.cancel.is_cancelled() || Instant::now() >= self.deadline {
            Err(AppError::new(ErrorCode::ResearchTimeout))
        } else {
            Ok(())
        }
    }
}
