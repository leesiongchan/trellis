use crate::{config::Config, jobs::Coordinator, telemetry::Telemetry};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
pub struct AppState {
    pub store: Arc<crate::storage::KnowledgeStore>,
    pub knowledge: Arc<crate::knowledge::KnowledgeService>,
    pub knowledge_runtime: Arc<crate::knowledge::KnowledgeRuntime>,
    pub config: Arc<Config>,
    pub jobs: Arc<Coordinator>,
    pub telemetry: Arc<Telemetry>,
    pub draining: CancellationToken,
}
impl AppState {
    pub async fn start(
        config: Arc<Config>,
        provider: Arc<dyn crate::providers::ResearchProvider>,
        verifier: Option<Arc<dyn crate::knowledge::verification::ConnectionVerifier>>,
        telemetry: Arc<Telemetry>,
    ) -> Result<Arc<Self>, crate::error::AppError> {
        use crate::{
            embeddings::{Embedder, EmbeddingWorker, LocalEmbedder, ModelManifest},
            knowledge::{KnowledgeRuntime, KnowledgeService, matching::MatchingService},
            storage::KnowledgeStore,
        };
        let store = Arc::new(KnowledgeStore::open(&config.storage).await?);
        store.initialize_schema().await?;
        let root = config.storage.model_dir.clone();
        let embedder = tokio::task::spawn_blocking(move || {
            ModelManifest::pinned().and_then(|m| LocalEmbedder::load(&root, &m))
        })
        .await
        .ok()
        .and_then(Result::ok)
        .map(|e| Arc::new(e) as Arc<dyn Embedder>);
        if embedder.is_none() {
            tracing::warn!(
                event = "local_discovery_unavailable",
                code = "LOCAL_MODEL_UNAVAILABLE"
            );
        }
        let jobs =
            Coordinator::instrumented(provider.clone(), config.limits.clone(), telemetry.clone());
        let mut service = KnowledgeService::new(store.clone(), jobs.clone(), provider);
        if let (Some(embedder), Some(verifier)) = (&embedder, verifier) {
            service = service.with_matching(
                Arc::new(MatchingService::new(store.clone(), embedder.clone())),
                verifier,
            );
        }
        let service = Arc::new(service);
        let draining = CancellationToken::new();
        let worker = embedder.map(|e| EmbeddingWorker::start(store.clone(), e, draining.clone()));
        let knowledge_runtime = Arc::new(
            KnowledgeRuntime::start(
                config.clone(),
                store.clone(),
                service.clone(),
                worker,
                draining.clone(),
            )
            .await?,
        );
        Ok(Arc::new(Self {
            store,
            knowledge: service,
            knowledge_runtime,
            config,
            jobs,
            telemetry,
            draining,
        }))
    }
}
pub fn api_router(state: Arc<AppState>) -> axum::Router {
    finish(api_routes(state.clone()), state)
}
pub fn router(
    state: Arc<AppState>,
    assets: Arc<crate::http::static_files::StaticAssets>,
) -> axum::Router {
    finish(
        api_routes(state.clone()).merge(crate::http::static_files::routes(assets)),
        state,
    )
}
fn finish(router: axum::Router, state: Arc<AppState>) -> axum::Router {
    router
        .layer(axum::middleware::from_fn_with_state(
            state,
            crate::http::headers::boundary,
        ))
        .layer(
            tower_http::sensitive_headers::SetSensitiveResponseHeadersLayer::new([
                axum::http::header::SET_COOKIE,
            ]),
        )
}
fn api_routes(state: Arc<AppState>) -> axum::Router {
    use crate::{
        access,
        http::{health, research},
    };
    use axum::{
        Router,
        routing::{get, post},
    };
    Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/research", post(research::research))
                .fallback(research::not_found),
        )
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/metrics", get(health::metrics))
        .with_state(state.clone())
        .merge(access::routes(state.config.clone()))
        .method_not_allowed_fallback(research::wrong_method)
}
