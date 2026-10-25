use crate::{config::Config, jobs::Coordinator, telemetry::Telemetry};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
pub struct AppState {
    pub config: Arc<Config>,
    pub jobs: Arc<Coordinator>,
    pub telemetry: Arc<Telemetry>,
    pub draining: CancellationToken,
}
pub fn api_router(state: Arc<AppState>) -> axum::Router {
    finish(api_routes(state.clone()),state)
}
pub fn router(state:Arc<AppState>,assets:Arc<crate::http::static_files::StaticAssets>)->axum::Router{
    finish(api_routes(state.clone()).merge(crate::http::static_files::routes(assets)),state)
}
fn finish(router:axum::Router,state:Arc<AppState>)->axum::Router{
    router.layer(axum::middleware::from_fn_with_state(state,crate::http::headers::boundary)).layer(tower_http::sensitive_headers::SetSensitiveResponseHeadersLayer::new([axum::http::header::SET_COOKIE]))
}
fn api_routes(state:Arc<AppState>)->axum::Router{
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
