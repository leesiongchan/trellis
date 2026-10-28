use crate::{
    contracts::library::*,
    error::{AppError, ErrorCode},
    jobs::Coordinator,
    providers::ResearchProvider,
    storage::KnowledgeStore,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::oneshot;
use tokio_util::task::TaskTracker;
use tracing::Instrument;
pub struct KnowledgeService {
    pub(super) store: Arc<KnowledgeStore>,
    pub(super) jobs: Arc<Coordinator>,
    pub(super) provider: Arc<dyn ResearchProvider>,
    pub(super) tasks: TaskTracker,
    pub(super) failed: Arc<AtomicBool>,
    draining: AtomicBool,
    hooks: Option<super::research::MatchingHooks>,
    paid_research_enabled: bool,
}
impl KnowledgeService {
    pub fn new(
        store: Arc<KnowledgeStore>,
        jobs: Arc<Coordinator>,
        provider: Arc<dyn ResearchProvider>,
    ) -> Self {
        Self {
            store,
            jobs,
            provider,
            tasks: TaskTracker::new(),
            failed: Arc::new(AtomicBool::new(false)),
            draining: AtomicBool::new(false),
            hooks: None,
            paid_research_enabled: true,
        }
    }
    pub fn with_research_enabled(mut self, enabled: bool) -> Self {
        self.paid_research_enabled = enabled;
        self
    }
    pub fn with_matching(
        mut self,
        matching: Arc<super::matching::MatchingService>,
        verifier: Arc<dyn super::verification::ConnectionVerifier>,
    ) -> Self {
        self.hooks = Some((matching, verifier));
        self
    }
    pub async fn start_research(
        &self,
        command: Command<ResearchInput>,
    ) -> Result<OperationView, AppError> {
        if self.draining.load(Ordering::SeqCst) {
            return Err(AppError::new(ErrorCode::ServerDraining));
        }
        let runner = super::research::ResearchRunner {
            store: self.store.clone(),
            jobs: self.jobs.clone(),
            provider: self.provider.clone(),
            tasks: self.tasks.clone(),
            failed: self.failed.clone(),
            hooks: self.hooks.clone(),
            paid_research_enabled: self.paid_research_enabled,
        };
        let (tx, rx) = oneshot::channel();
        // The server owns reservation/admission even if the requesting connection disappears.
        self.tasks.spawn(
            async move {
                let result = runner.start(command).await;
                let _ = tx.send(result);
            }
            .in_current_span(),
        );
        rx.await
            .map_err(|_| AppError::new(ErrorCode::WorkInterrupted))?
    }
    pub async fn drain(&self) -> Result<(), AppError> {
        self.draining.store(true, Ordering::SeqCst);
        self.jobs.begin_drain();
        self.tasks.close();
        self.jobs.idle().await;
        self.tasks.wait().await;
        if self.failed.load(Ordering::SeqCst) {
            Err(AppError::new(ErrorCode::StorageUnavailable))
        } else {
            Ok(())
        }
    }
}
