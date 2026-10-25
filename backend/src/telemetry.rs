//! Allowlisted logs and bounded-label process metrics.
mod log_writer;
use crate::error::{AppError, ErrorCode};
use axum::http::StatusCode;
pub use log_writer::LoggingGuard;
use prometheus_client::{
    encoding::{EncodeLabelSet, text::encode},
    metrics::{counter::Counter, family::Family, gauge::Gauge, histogram::Histogram},
    registry::Registry,
};
use std::time::Duration;
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct HttpLabels {
    route: &'static str,
    method: &'static str,
    status: &'static str,
}
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct StageLabels {
    stage: &'static str,
    outcome: &'static str,
}
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct RejectionLabels {
    code: &'static str,
}
#[derive(Clone, Copy)]
pub enum Stage {
    Search,
    Generation,
    Grounding,
    Verification,
}
impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Generation => "generation",
            Self::Grounding => "grounding",
            Self::Verification => "verification",
        }
    }
}
fn histogram() -> Histogram {
    Histogram::new([0.01, 0.05, 0.1, 0.5, 1., 5., 20., 60., 120.])
}
pub struct Telemetry {
    registry: Registry,
    http_requests: Family<HttpLabels, Counter>,
    http_duration: Family<HttpLabels, Histogram>,
    stages: Family<StageLabels, Counter>,
    stage_duration: Family<StageLabels, Histogram>,
    active_jobs: Gauge,
    waiters: Gauge,
    deduplicated: Counter,
    rejections: Family<RejectionLabels, Counter>,
    log_drops: Counter,
}
impl Default for Telemetry {
    fn default() -> Self {
        Self::new()
    }
}
impl Telemetry {
    pub fn start(self: &std::sync::Arc<Self>, stage: Stage, job_id: &str) -> StageTimer {
        StageTimer {
            telemetry: self.clone(),
            stage,
            job_id: job_id.into(),
            started: std::time::Instant::now(),
            finished: false,
        }
    }
    pub fn new() -> Self {
        let mut registry = Registry::default();
        let http_requests = Family::<HttpLabels, Counter>::default();
        let http_duration = Family::new_with_constructor(histogram as fn() -> Histogram);
        let stages = Family::<StageLabels, Counter>::default();
        let stage_duration = Family::new_with_constructor(histogram as fn() -> Histogram);
        let active_jobs = Gauge::default();
        let waiters = Gauge::default();
        let deduplicated = Counter::default();
        let rejections = Family::<RejectionLabels, Counter>::default();
        let log_drops = Counter::default();
        registry.register(
            "branch_log_events_dropped",
            "Log events dropped by the bounded output queue or failed sink",
            log_drops.clone(),
        );
        registry.register(
            "branch_http_requests",
            "HTTP request outcomes",
            http_requests.clone(),
        );
        registry.register(
            "branch_http_duration_seconds",
            "HTTP latency",
            http_duration.clone(),
        );
        registry.register(
            "branch_research_stages",
            "Research stage outcomes",
            stages.clone(),
        );
        registry.register(
            "branch_research_stage_duration_seconds",
            "Research stage latency",
            stage_duration.clone(),
        );
        registry.register(
            "branch_active_jobs",
            "Currently executing research jobs",
            active_jobs.clone(),
        );
        registry.register(
            "branch_waiters",
            "Currently waiting research requests",
            waiters.clone(),
        );
        registry.register(
            "branch_deduplicated_requests",
            "Requests sharing active work",
            deduplicated.clone(),
        );
        registry.register(
            "branch_admission_rejections",
            "Rejected research admissions",
            rejections.clone(),
        );
        Self {
            registry,
            http_requests,
            http_duration,
            stages,
            stage_duration,
            active_jobs,
            waiters,
            deduplicated,
            rejections,
            log_drops,
        }
    }
    pub fn render(&self) -> Result<String, AppError> {
        let mut text = String::new();
        encode(&mut text, &self.registry).map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
        Ok(text)
    }
    pub fn http(
        &self,
        route: &'static str,
        method: &axum::http::Method,
        status: StatusCode,
        duration: Duration,
    ) {
        let method = match *method {
            axum::http::Method::GET => "GET",
            axum::http::Method::POST => "POST",
            axum::http::Method::HEAD => "HEAD",
            _ => "OTHER",
        };
        let status = match status.as_u16() / 100 {
            1 => "1xx",
            2 => "2xx",
            3 => "3xx",
            4 => "4xx",
            5 => "5xx",
            _ => "other",
        };
        let labels = HttpLabels {
            route,
            method,
            status,
        };
        self.http_requests.get_or_create(&labels).inc();
        self.http_duration
            .get_or_create(&labels)
            .observe(duration.as_secs_f64());
    }
    pub fn stage<T>(
        &self,
        stage: Stage,
        result: &Result<T, AppError>,
        duration: Duration,
        job_id: &str,
    ) {
        let code = result.as_ref().err().map(AppError::code).unwrap_or("ok");
        let labels = StageLabels {
            stage: stage.name(),
            outcome: code,
        };
        self.stages.get_or_create(&labels).inc();
        self.stage_duration
            .get_or_create(&labels)
            .observe(duration.as_secs_f64());
        tracing::info!(
            event = "research_stage_completed",
            entryPoint = "research_endpoint",
            jobId = job_id,
            stage = stage.name(),
            code,
            durationMs = duration.as_millis() as u64
        );
    }
    pub(crate) fn job_started(&self) {
        self.active_jobs.inc();
    }
    pub(crate) fn job_finished(&self) {
        self.active_jobs.dec();
    }
    pub(crate) fn waiter_added(&self) {
        self.waiters.inc();
    }
    pub(crate) fn waiter_removed(&self) {
        self.waiters.dec();
    }
    pub(crate) fn shared(&self) {
        self.deduplicated.inc();
    }
    pub(crate) fn rejected(&self, error: &AppError) {
        self.rejections
            .get_or_create(&RejectionLabels { code: error.code() })
            .inc();
    }
}
pub struct StageTimer {
    telemetry: std::sync::Arc<Telemetry>,
    stage: Stage,
    job_id: String,
    started: std::time::Instant,
    finished: bool,
}
impl StageTimer {
    pub fn finish<T>(mut self, result: &Result<T, AppError>) {
        self.telemetry
            .stage(self.stage, result, self.started.elapsed(), &self.job_id);
        self.finished = true;
    }
}
impl Drop for StageTimer {
    fn drop(&mut self) {
        if !self.finished {
            self.telemetry.stage(
                self.stage,
                &Err::<(), _>(AppError::new(ErrorCode::ResearchTimeout)),
                self.started.elapsed(),
                &self.job_id,
            );
        }
    }
}
pub fn init_logging(filter: &str, telemetry: &Telemetry) -> Result<LoggingGuard, AppError> {
    use tracing_subscriber::util::SubscriberInitExt;
    let filter = tracing_subscriber::EnvFilter::try_new(filter)
        .map_err(|_| AppError::new(ErrorCode::InvalidConfig))?;
    let (writer, guard) = log_writer::start(telemetry.log_drops.clone())?;
    tracing_subscriber::fmt()
        .json()
        .with_writer(writer)
        .with_env_filter(filter)
        .with_current_span(true)
        .with_span_list(true)
        .finish()
        .try_init()
        .map_err(|_| AppError::new(ErrorCode::InvalidConfig))?;
    Ok(guard)
}
pub fn install_redacted_panic_hook() {
    std::panic::set_hook(Box::new(|_| {
        tracing::error!(
            event = "task_panicked",
            entryPoint = "runtime",
            code = "INTERNAL_FAILURE"
        );
    }));
}
