use crate::{
    contracts::{ResearchResult, library::*},
    error::AppError,
    jobs::{Coordinator, JobKey, JobWork},
    providers::ResearchProvider,
    research::research_topic,
    storage::KnowledgeStore,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::task::TaskTracker;
pub(super) async fn start(
    store: Arc<KnowledgeStore>,
    jobs: Arc<Coordinator>,
    provider: Arc<dyn ResearchProvider>,
    tasks: TaskTracker,
    failed: Arc<AtomicBool>,
    command: Command<ResearchInput>,
) -> Result<OperationView, AppError> {
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
    let (store_for_work, attempt_for_work) = (store.clone(), attempt.clone());
    let work: JobWork = Box::new(move |ctx| {
        Box::pin(async move {
            let outcome =
                research_topic(&attempt_for_work.request, &ctx, provider.as_ref()).await?;
            ctx.check()?;
            store_for_work
                .complete_attempt(&attempt_for_work, &outcome)
                .await?;
            // Task 6 attaches derived matching here, after the commit and within this deadline.
            Ok(outcome)
        })
    });
    match jobs.admit_work(JobKey::new(attempt.id.to_string()), work) {
        Ok(mut waiter) => {
            tasks.spawn(async move {
                if let Err(error) = waiter.result().await
                    && store
                        .fail_attempt(&attempt, (*error).clone())
                        .await
                        .is_err()
                {
                    failed.store(true, Ordering::SeqCst);
                }
            });
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
