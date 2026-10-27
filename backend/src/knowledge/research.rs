use super::{matching::MatchingService, verification::ConnectionVerifier};
use crate::{
    contracts::{ResearchResult, library::*},
    error::AppError,
    jobs::{Coordinator, JobKey, JobWork},
    providers::ResearchProvider,
    research::research_topic_recorded,
    storage::KnowledgeStore,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::task::TaskTracker;
use tracing::Instrument;
pub(super) type MatchingHooks = (Arc<MatchingService>, Arc<dyn ConnectionVerifier>);
pub(super) struct ResearchRunner {
    pub store: Arc<KnowledgeStore>,
    pub jobs: Arc<Coordinator>,
    pub provider: Arc<dyn ResearchProvider>,
    pub tasks: TaskTracker,
    pub failed: Arc<AtomicBool>,
    pub hooks: Option<MatchingHooks>,
    pub paid_research_enabled: bool,
}
impl ResearchRunner {
    pub async fn start(self, command: Command<ResearchInput>) -> Result<OperationView, AppError> {
        let Self {
            store,
            jobs,
            provider,
            tasks,
            failed,
            hooks,
            paid_research_enabled,
        } = self;
        let reservation = store.reserve_research(&command).await?;
        if !reservation.leader {
            return Ok(reservation.operation);
        }
        let Some(attempt) = reservation.attempt else {
            return Ok(reservation.operation);
        };
        if let Some(version) = &attempt.reuse_version {
            let reused = async {
                let answer = store
                    .topic(&attempt.topic_id, Some(version))
                    .await?
                    .selected_answer
                    .ok_or_else(|| AppError::new(crate::error::ErrorCode::NotFound))?;
                store
                    .complete_attempt(
                        &attempt,
                        &ResearchResult::Topic {
                            content: answer.content,
                        },
                        false,
                    )
                    .await?;
                store.operation(&command.operation_id).await
            }
            .await;
            if let Err(error) = &reused
                && store.fail_attempt(&attempt, error.clone()).await.is_err()
            {
                failed.store(true, Ordering::SeqCst);
            }
            return reused;
        }
        if !paid_research_enabled {
            let error = AppError::new(crate::error::ErrorCode::SetupRequired);
            if store.fail_attempt(&attempt, error.clone()).await.is_err() {
                failed.store(true, Ordering::SeqCst);
            }
            return Err(error);
        }
        let (store_for_work, attempt_for_work) = (store.clone(), attempt.clone());
        let operation_id = command.operation_id;
        let work: JobWork = Box::new(move |ctx| {
            Box::pin(async move {
                let generated =
                    research_topic_recorded(&attempt_for_work.request, &ctx, provider.as_ref())
                        .await?;
                let outcome = generated.result;
                let mut attempt_for_work = attempt_for_work;
                attempt_for_work.generation = generated.generation;
                ctx.check()?;
                let keep_active =
                    hooks.is_some() && matches!(outcome, ResearchResult::Topic { .. });
                store_for_work
                    .complete_attempt(&attempt_for_work, &outcome, keep_active)
                    .await?;
                if keep_active
                    && let Some((matching, verifier)) = hooks
                    && let Err(error) = matching
                        .after_research(&attempt_for_work, &operation_id, &ctx, verifier.as_ref())
                        .await
                {
                    tracing::warn!(
                        code = error.code(),
                        "saved research connection check did not finish"
                    );
                }
                store_for_work
                    .release_research_attempt(&attempt_for_work, false)
                    .await?;
                Ok(outcome)
            })
        });
        match jobs.admit_work(JobKey::new(attempt.id.to_string()), work) {
            Ok(mut waiter) => {
                tasks.spawn(
                    async move {
                        let outcome = waiter.result().await;
                        if let Err(error) = &outcome
                            && store
                                .fail_attempt(&attempt, (**error).clone())
                                .await
                                .is_err()
                        {
                            failed.store(true, Ordering::SeqCst);
                        }
                        if store
                            .release_research_attempt(&attempt, outcome.is_err())
                            .await
                            .is_err()
                        {
                            failed.store(true, Ordering::SeqCst);
                        }
                    }
                    .in_current_span(),
                );
            }
            Err(error) => {
                if store.fail_attempt(&attempt, error.clone()).await.is_err() {
                    failed.store(true, Ordering::SeqCst);
                }
                return Err(error);
            }
        }
        Ok(reservation.operation)
    }
}
