//! Narrow provider boundary; the executable only uses fixed production endpoints.
mod openai;
mod tavily;
mod transport;
use crate::{
    config::Config,
    contracts::*,
    error::{AppError, ErrorCode},
    research::context::JobContext,
};
use std::{future::Future, pin::Pin, sync::Arc};
pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'a>>;
pub struct GeneratedResearch {
    pub result: ResearchResult,
    pub generation: Option<crate::contracts::library::GenerationMetadata>,
}
impl From<ResearchResult> for GeneratedResearch {
    fn from(result: ResearchResult) -> Self {
        Self {
            result,
            generation: None,
        }
    }
}
pub trait ResearchProvider: Send + Sync {
    fn search<'a>(
        &'a self,
        query: &'a str,
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, Vec<RetrievedSource>>;
    fn generate<'a>(
        &'a self,
        input: &'a ResearchRequest,
        sources: &'a [RetrievedSource],
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, ResearchResult>;
    fn generate_recorded<'a>(
        &'a self,
        input: &'a ResearchRequest,
        sources: &'a [RetrievedSource],
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, GeneratedResearch> {
        Box::pin(async move { Ok(self.generate(input, sources, ctx).await?.into()) })
    }
    fn verify<'a>(
        &'a self,
        content: &'a TopicContent,
        sources: &'a [RetrievedSource],
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, GroundingVerdict>;
}
pub struct ProviderEndpoints {
    pub openai: url::Url,
    pub tavily: url::Url,
}
pub struct HttpProviders {
    config: Arc<Config>,
    client: reqwest::Client,
    endpoints: ProviderEndpoints,
    telemetry: Option<Arc<crate::telemetry::Telemetry>>,
}
impl HttpProviders {
    pub fn production(config: Arc<Config>) -> Result<Self, AppError> {
        let endpoints = ProviderEndpoints {
            openai: url::Url::parse("https://api.openai.com/v1/responses")
                .map_err(|_| AppError::new(ErrorCode::InvalidConfig))?,
            tavily: url::Url::parse("https://api.tavily.com/search")
                .map_err(|_| AppError::new(ErrorCode::InvalidConfig))?,
        };
        Self::with_endpoints(config, endpoints)
    }
    pub fn with_endpoints(
        config: Arc<Config>,
        endpoints: ProviderEndpoints,
    ) -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(config.limits.connect)
            .pool_max_idle_per_host(2)
            .build()
            .map_err(|_| AppError::new(ErrorCode::InvalidConfig))?;
        Ok(Self {
            config,
            client,
            endpoints,
            telemetry: None,
        })
    }
    pub fn with_telemetry(mut self, telemetry: Arc<crate::telemetry::Telemetry>) -> Self {
        self.telemetry = Some(telemetry);
        self
    }
    async fn observed<T>(
        &self,
        stage: crate::telemetry::Stage,
        ctx: &JobContext,
        future: impl Future<Output = Result<T, AppError>>,
    ) -> Result<T, AppError> {
        use tracing::Instrument;
        let timer = ctx
            .telemetry
            .as_ref()
            .or(self.telemetry.as_ref())
            .map(|t| t.start(stage, &ctx.job_id));
        let span = tracing::info_span!("research_stage",entryPoint="research_endpoint",jobId=%ctx.job_id,stage=stage.name());
        let result = future.instrument(span).await;
        if let Some(timer) = timer {
            timer.finish(&result);
        }
        result
    }
}
impl ResearchProvider for HttpProviders {
    fn search<'a>(
        &'a self,
        query: &'a str,
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, Vec<RetrievedSource>> {
        Box::pin(self.observed(
            crate::telemetry::Stage::Search,
            ctx,
            self.search_evidence(query, ctx),
        ))
    }
    fn generate<'a>(
        &'a self,
        input: &'a ResearchRequest,
        sources: &'a [RetrievedSource],
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, ResearchResult> {
        Box::pin(
            self.observed(crate::telemetry::Stage::Generation, ctx, async {
                Ok(self.generate_topic(input, sources, ctx).await?.result)
            }),
        )
    }
    fn generate_recorded<'a>(
        &'a self,
        input: &'a ResearchRequest,
        sources: &'a [RetrievedSource],
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, GeneratedResearch> {
        Box::pin(self.observed(
            crate::telemetry::Stage::Generation,
            ctx,
            self.generate_topic(input, sources, ctx),
        ))
    }
    fn verify<'a>(
        &'a self,
        content: &'a TopicContent,
        sources: &'a [RetrievedSource],
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, GroundingVerdict> {
        Box::pin(self.observed(
            crate::telemetry::Stage::Verification,
            ctx,
            self.verify_grounding(content, sources, ctx),
        ))
    }
}
