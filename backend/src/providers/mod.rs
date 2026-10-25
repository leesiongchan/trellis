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
        })
    }
}
impl ResearchProvider for HttpProviders {
    fn search<'a>(
        &'a self,
        query: &'a str,
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, Vec<RetrievedSource>> {
        Box::pin(self.search_evidence(query, ctx))
    }
    fn generate<'a>(
        &'a self,
        input: &'a ResearchRequest,
        sources: &'a [RetrievedSource],
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, ResearchResult> {
        Box::pin(self.generate_topic(input, sources, ctx))
    }
    fn verify<'a>(
        &'a self,
        content: &'a TopicContent,
        sources: &'a [RetrievedSource],
        ctx: &'a JobContext,
    ) -> ProviderFuture<'a, GroundingVerdict> {
        Box::pin(self.verify_grounding(content, sources, ctx))
    }
}
