use branch_backend::{
    app::{AppState, router},
    config::{Config, load_values},
    error::{AppError, ErrorCode},
    http::static_files::StaticAssets,
    providers::HttpProviders,
    server::Server,
    telemetry::{Telemetry, init_logging, install_redacted_panic_hook},
};
use std::{process::ExitCode, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
fn main() -> ExitCode {
    install_redacted_panic_hook();
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => return ExitCode::FAILURE,
    };
    let result = runtime.block_on(run());
    runtime.shutdown_timeout(Duration::from_secs(1));
    match result {
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
    let values = load_values()?;
    if storage_command(&values, &executable).await? {
        return Ok(());
    }
    let config = Arc::new(Config::from_values(&values, &executable)?);
    let telemetry = Arc::new(Telemetry::new());
    let _logging = init_logging(&config.log_filter, &telemetry)?;
    let assets = Arc::new(StaticAssets::load(&config.asset_dir)?);
    let provider =
        Arc::new(HttpProviders::production(config.clone())?.with_telemetry(telemetry.clone()));
    let state =
        AppState::start(config.clone(), provider.clone(), Some(provider), telemetry).await?;
    let runtime = state.knowledge_runtime.clone();
    let jobs = state.jobs.clone();
    let draining = state.draining.clone();
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
    let storage_result = runtime.shutdown().await;
    finished.cancel();
    // Yield once so signal/deadline supervisors observe completion before runtime teardown.
    tokio::time::sleep(Duration::ZERO).await;
    result.and(storage_result)
}

async fn storage_command(
    values: &std::collections::HashMap<String, String>,
    executable: &std::path::Path,
) -> Result<bool, AppError> {
    use branch_backend::storage::{KnowledgeStore, StorageConfig};
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return Ok(false);
    }
    let invalid = || AppError::new(ErrorCode::InvalidConfig);
    if args.first().map(String::as_str) != Some("storage")
        || args.len() < 4
        || (args.len() - 2) % 2 != 0
    {
        return Err(invalid());
    }
    let mut flags = std::collections::HashMap::new();
    for pair in args[2..].as_chunks::<2>().0 {
        if flags.insert(pair[0].as_str(), pair[1].as_str()).is_some() {
            return Err(invalid());
        }
    }
    let manifest = match args[1].as_str() {
        "backup" if flags.len() == 1 => {
            let output = flags.get("--output").ok_or_else(invalid)?;
            let config = StorageConfig::from_values(values, executable)?;
            let store = KnowledgeStore::open(&config).await?;
            store.check_schema().await?;
            let result = store.backup(std::path::Path::new(output)).await;
            let close = store.close().await;
            result.and(close.map(|_| ())).map(|_| ())?;
            // Read the completed manifest; it contains no question or answer text.
            std::fs::read(std::path::Path::new(output).join("manifest.json"))
                .map_err(|_| invalid())?
        }
        "restore" if flags.len() == 2 => {
            let source = flags.get("--input").ok_or_else(invalid)?;
            let target = flags.get("--data-dir").ok_or_else(invalid)?;
            let mut check = values.clone();
            check.insert("APP_DATA_DIR".into(), target.to_string());
            StorageConfig::from_values(&check, executable)?;
            serde_json::to_vec(
                &KnowledgeStore::restore(
                    std::path::Path::new(source),
                    std::path::Path::new(target),
                )
                .await?,
            )
            .map_err(|_| invalid())?
        }
        _ => return Err(invalid()),
    };
    println!("{}", std::str::from_utf8(&manifest).map_err(|_| invalid())?);
    Ok(true)
}
