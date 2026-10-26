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
pub struct KnowledgeService {
    pub(super) store: Arc<KnowledgeStore>,
    pub(super) jobs: Arc<Coordinator>,
    pub(super) provider: Arc<dyn ResearchProvider>,
    pub(super) tasks: TaskTracker,
    pub(super) failed: Arc<AtomicBool>,
    draining: AtomicBool,
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
        }
    }
    pub async fn start_research(
        &self,
        command: Command<ResearchInput>,
    ) -> Result<OperationView, AppError> {
        if self.draining.load(Ordering::SeqCst) {
            return Err(AppError::new(ErrorCode::ServerDraining));
        }
        let (store, jobs, provider, tasks, failed) = (
            self.store.clone(),
            self.jobs.clone(),
            self.provider.clone(),
            self.tasks.clone(),
            self.failed.clone(),
        );
        let (tx, rx) = oneshot::channel();
        // The server owns reservation/admission even if the requesting connection disappears.
        self.tasks.spawn(async move {
            let result =
                super::research::start(store, jobs, provider, tasks, failed, command).await;
            let _ = tx.send(result);
        });
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
