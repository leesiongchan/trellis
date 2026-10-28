use super::*;
use serde_json::Value;
use tokio::time::{Instant, timeout_at};
impl HttpProviders {
    pub(super) async fn post(
        &self,
        url: &url::Url,
        key: &str,
        body: &Value,
        ctx: &JobContext,
        deadline: Instant,
        failure: ErrorCode,
    ) -> Result<Value, AppError> {
        ctx.check()?;
        if key.is_empty() {
            return Err(AppError::new(ErrorCode::SetupRequired));
        }
        let operation = async {
            let mut response = self
                .client
                .post(url.clone())
                .bearer_auth(key)
                .json(body)
                .send()
                .await
                .map_err(|_| AppError::new(failure))?;
            if !response.status().is_success() {
                return Err(AppError::new(
                    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        ErrorCode::ProviderLimit
                    } else {
                        failure
                    },
                ));
            }
            if response
                .content_length()
                .is_some_and(|n| n > self.config.limits.provider_bytes as u64)
            {
                return Err(AppError::new(failure));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| AppError::new(failure))? {
                if chunk.len()
                    > self
                        .config
                        .limits
                        .provider_bytes
                        .saturating_sub(bytes.len())
                {
                    return Err(AppError::new(failure));
                }
                bytes.extend_from_slice(&chunk);
            }
            serde_json::from_slice(&bytes).map_err(|_| AppError::new(failure))
        };
        tokio::select! {
            biased;
            _=ctx.cancel.cancelled()=>Err(AppError::new(ErrorCode::ResearchTimeout)),
            result=timeout_at(deadline.min(ctx.deadline),operation)=>result.map_err(|_|AppError::new(ErrorCode::ResearchTimeout))?,
        }
    }
}
