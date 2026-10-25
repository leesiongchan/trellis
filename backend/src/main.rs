use branch_backend::{
    app::{AppState, router},
    config::{Config, load_values},
    error::{AppError, ErrorCode},
    http::static_files::StaticAssets,
    jobs::Coordinator,
    providers::HttpProviders,
    server::Server,
    telemetry::{Telemetry, init_logging, install_redacted_panic_hook},
};
use std::{process::ExitCode, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
#[tokio::main]
async fn main() -> ExitCode {
    install_redacted_panic_hook();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "{{\"level\":\"ERROR\",\"event\":\"startup_or_runtime_failed\",\"entryPoint\":\"runtime\",\"code\":\"{}\"}}",
                error.code()
            );
            ExitCode::FAILURE
        }
    }
}
async fn run() -> Result<(), AppError> {
    let executable =
        std::env::current_exe().map_err(|_| AppError::new(ErrorCode::InvalidConfig))?;
    let config = Arc::new(Config::from_values(&load_values()?, &executable)?);
    init_logging(&config.log_filter)?;
    let assets = Arc::new(StaticAssets::load(&config.asset_dir)?);
    let telemetry = Arc::new(Telemetry::new());
    let provider =
        Arc::new(HttpProviders::production(config.clone())?.with_telemetry(telemetry.clone()));
    let jobs = Coordinator::instrumented(provider, config.limits.clone(), telemetry.clone());
    let draining = CancellationToken::new();
    let state = Arc::new(AppState {
        config: config.clone(),
        jobs: jobs.clone(),
        telemetry,
        draining: draining.clone(),
    });
    let server = Server::bind(
        router(state, assets),
        config.bind,
        jobs,
        draining,
        config.limits.clone(),
    )
    .await?;
    tracing::info!(
        event = "server_started",
        entryPoint = "runtime",
        port = server.local_addr().port(),
        researchConfigured = config.research_configured()
    );
    let finished = CancellationToken::new();
    let signal_finished = finished.clone();
    let max_shutdown = config.limits.shutdown_total;
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
    let shutdown = async move {
        tokio::select! {_=tokio::signal::ctrl_c()=>{},_=terminate.recv()=>{}}
        let hard_finished = signal_finished.clone();
        tokio::spawn(async move {
            tokio::select! {_=hard_finished.cancelled()=>{},_=tokio::time::sleep(max_shutdown)=>{tracing::error!(event="shutdown_hard_deadline",entryPoint="runtime");std::process::exit(1);}}
        });
        tokio::spawn(async move {
            tokio::select! {_=signal_finished.cancelled()=>{},_=tokio::signal::ctrl_c()=>{std::process::exit(130);},_=terminate.recv()=>{std::process::exit(130);}}
        });
    };
    let result = server.run(shutdown).await;
    finished.cancel();
    // Yield once so signal/deadline supervisors observe completion before runtime teardown.
    tokio::time::sleep(Duration::ZERO).await;
    result
}
