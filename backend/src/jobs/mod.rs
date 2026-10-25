//! Bounded, cancellable in-flight research sharing.
mod waiter;
use crate::{
    config::Limits,
    contracts::{ResearchRequest, ResearchResult},
    error::{AppError, ErrorCode},
    providers::ResearchProvider,
    research::{context::JobContext, research_topic},
};
use futures_util::FutureExt;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    panic::AssertUnwindSafe,
    sync::{Arc, Mutex, MutexGuard, Weak},
    time::Duration,
};
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore, watch},
    time::{Instant, timeout_at},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
pub use waiter::Waiter;
type Outcome = Result<Arc<ResearchResult>, Arc<AppError>>;
struct Entry {
    generation: String,
    cancel: CancellationToken,
    sender: watch::Sender<Option<Outcome>>,
    waiters: usize,
}
#[derive(Default)]
struct Inner {
    entries: HashMap<String, Entry>,
    starts: VecDeque<Instant>,
    waiters: usize,
    draining: bool,
}
pub struct Coordinator {
    provider: Arc<dyn ResearchProvider>,
    limits: Limits,
    inner: Mutex<Inner>,
    capacity: Arc<Semaphore>,
    tasks: TaskTracker,
}
pub struct JobSnapshot {
    pub active: usize,
    pub waiters: usize,
    pub new_starts: usize,
}
impl Coordinator {
    pub fn new(provider: Arc<dyn ResearchProvider>, limits: Limits) -> Arc<Self> {
        Arc::new(Self {
            provider,
            capacity: Arc::new(Semaphore::new(limits.concurrent)),
            limits,
            inner: Mutex::new(Inner::default()),
            tasks: TaskTracker::new(),
        })
    }
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub fn admit(self: &Arc<Self>, input: ResearchRequest) -> Result<Waiter, AppError> {
        let canonical =
            serde_json::to_vec(&input).map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
        let key = format!("{:x}", Sha256::digest(canonical));
        let mut state = self.lock();
        let now = Instant::now();
        while state
            .starts
            .front()
            .is_some_and(|t| now.duration_since(*t) >= Duration::from_secs(60))
        {
            state.starts.pop_front();
        }
        if state.draining {
            return Err(AppError::new(ErrorCode::ServerDraining));
        }
        if state.waiters >= self.limits.waiters {
            return Err(AppError::new(ErrorCode::ResearchLimit));
        }
        if let Some(entry) = state.entries.get_mut(&key) {
            entry.waiters += 1;
            let waiter = Waiter {
                owner: Arc::downgrade(self),
                key,
                generation: entry.generation.clone(),
                receiver: entry.sender.subscribe(),
            };
            state.waiters += 1;
            return Ok(waiter);
        }
        if state.starts.len() >= self.limits.per_minute {
            return Err(AppError::new(ErrorCode::ResearchLimit));
        }
        let permit = self
            .capacity
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::new(ErrorCode::ResearchLimit))?;
        let generation = uuid::Uuid::new_v4().simple().to_string();
        let cancel = CancellationToken::new();
        let (sender, receiver) = watch::channel(None);
        let waiter = Waiter {
            owner: Arc::downgrade(self),
            key: key.clone(),
            generation: generation.clone(),
            receiver,
        };
        state.entries.insert(
            key.clone(),
            Entry {
                generation: generation.clone(),
                cancel: cancel.clone(),
                sender: sender.clone(),
                waiters: 1,
            },
        );
        state.starts.push_back(now);
        state.waiters += 1;
        let guard = JobGuard {
            owner: Arc::downgrade(self),
            key,
            generation: generation.clone(),
            sender,
            published: false,
            permit: Some(permit),
        };
        let owner = self.clone();
        let ctx = JobContext {
            deadline: now + self.limits.research,
            cancel,
            job_id: generation,
        };
        self.tasks.spawn(async move {
            let work=AssertUnwindSafe(async {
                tokio::select! {
                    biased;
                    _=ctx.cancel.cancelled()=>Err(AppError::new(ErrorCode::ResearchTimeout)),
                    result=timeout_at(ctx.deadline,research_topic(&input,&ctx,owner.provider.as_ref()))=>result.map_err(|_|AppError::new(ErrorCode::ResearchTimeout))?,
                }
            }).catch_unwind().await;
            let outcome=work.unwrap_or_else(|_|Err(AppError::new(ErrorCode::InternalFailure)));
            guard.publish(outcome.map(Arc::new).map_err(Arc::new));
        });
        Ok(waiter)
    }
    pub fn begin_drain(&self) {
        self.lock().draining = true;
    }
    pub fn cancel_all(&self) {
        for entry in self.lock().entries.values() {
            entry.cancel.cancel();
        }
    }
    pub async fn idle(&self) {
        self.tasks.close();
        self.tasks.wait().await;
    }
    pub fn snapshot(&self) -> JobSnapshot {
        let state = self.lock();
        JobSnapshot {
            active: self.limits.concurrent - self.capacity.available_permits(),
            waiters: state.waiters,
            new_starts: state
                .starts
                .iter()
                .filter(|t| Instant::now().duration_since(**t) < Duration::from_secs(60))
                .count(),
        }
    }
    fn release(&self, key: &str, generation: &str) {
        let mut state = self.lock();
        state.waiters = state.waiters.saturating_sub(1);
        if let Some(entry) = state.entries.get_mut(key)
            && entry.generation == generation
        {
            entry.waiters = entry.waiters.saturating_sub(1);
            if entry.waiters == 0 {
                entry.cancel.cancel();
                state.entries.remove(key);
            }
        }
    }
}
struct JobGuard {
    owner: Weak<Coordinator>,
    key: String,
    generation: String,
    sender: watch::Sender<Option<Outcome>>,
    published: bool,
    permit: Option<OwnedSemaphorePermit>,
}
impl JobGuard {
    fn publish(mut self, outcome: Outcome) {
        self.sender.send_replace(Some(outcome));
        self.published = true;
    }
}
impl Drop for JobGuard {
    fn drop(&mut self) {
        if !self.published {
            self.sender.send_replace(Some(Err(Arc::new(AppError::new(
                ErrorCode::InternalFailure,
            )))));
        }
        if let Some(owner) = self.owner.upgrade() {
            let mut state = owner.lock();
            if state
                .entries
                .get(&self.key)
                .is_some_and(|e| e.generation == self.generation)
            {
                state.entries.remove(&self.key);
            }
        }
        self.permit.take();
    }
}
