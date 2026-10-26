use super::*;
use crate::{
    contracts::library::DiscoveryStatus,
    error::{AppError, ErrorCode},
    storage::KnowledgeStore,
};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
pub struct EmbeddingWorker {
    cancel: CancellationToken,
    task: Mutex<Option<JoinHandle<Result<(), AppError>>>>,
}
impl EmbeddingWorker {
    pub fn start(
        store: Arc<KnowledgeStore>,
        embedder: Arc<dyn Embedder>,
        cancel: CancellationToken,
    ) -> Self {
        let cancel = cancel.child_token();
        let owned_cancel = cancel.clone();
        let task = tokio::spawn(async move {
            let result = run(&store, embedder, &owned_cancel).await;
            if result.is_err() {
                let _ = store
                    .set_discovery(DiscoveryStatus::Unavailable, None)
                    .await;
            }
            result
        });
        Self {
            cancel,
            task: Mutex::new(Some(task)),
        }
    }
    pub async fn shutdown(&self) -> Result<(), AppError> {
        self.cancel.cancel();
        let task = self
            .task
            .lock()
            .map_err(|_| AppError::new(ErrorCode::InternalFailure))?
            .take();
        if let Some(task) = task {
            task.await
                .map_err(|_| AppError::new(ErrorCode::InternalFailure))?
        } else {
            Ok(())
        }
    }
}
async fn run(
    store: &Arc<KnowledgeStore>,
    embedder: Arc<dyn Embedder>,
    cancel: &CancellationToken,
) -> Result<(), AppError> {
    let model = embedder.manifest().clone();
    let version = model.version();
    store.register_embedding_model(&model).await?;
    store.set_discovery(DiscoveryStatus::Indexing, None).await?;
    let mut cursor = store.embedding_cursor(&version).await?;
    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let page = store
            .embedding_sources(cursor.kind, cursor.after.as_deref(), 1)
            .await?;
        if let Some(source) = page.items.first() {
            if source.library_epoch != cursor.library_epoch {
                cursor = store.embedding_cursor(&version).await?;
                continue;
            }
            if !store.embedding_completed(source, &version).await? {
                store.set_discovery(DiscoveryStatus::Indexing, None).await?;
                let result = async {
                    let prepared = source.prepare(&model)?;
                    let mut vectors = vec![];
                    for batch in prepared.inputs.chunks(32) {
                        let embedder = embedder.clone();
                        let batch = batch.to_vec();
                        let mut result =
                            tokio::task::spawn_blocking(move || embedder.embed(&batch))
                                .await
                                .map_err(|_| AppError::new(ErrorCode::InternalFailure))?
                                .map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
                        vectors.append(&mut result);
                        if cancel.is_cancelled() {
                            return Ok(());
                        }
                    }
                    store.index_source(source, &model, &vectors).await
                }
                .await;
                if cancel.is_cancelled() {
                    return Ok(());
                }
                if let Err(error) = result {
                    if error.kind == ErrorCode::RevisionConflict {
                        cursor = store.embedding_cursor(&version).await?;
                        continue;
                    }
                    let _ = store.embedding_failed(source, &version).await;
                    return Err(error);
                }
            }
            let id = source
                .answer_version_id
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or(source.topic_id.as_str());
            cursor.after = Some(id.into());
            store.save_embedding_cursor(&version, &cursor).await?;
            tokio::task::yield_now().await;
        } else if cursor.kind == SourceKind::Question {
            cursor.kind = SourceKind::Passage;
            cursor.after = None;
            store.save_embedding_cursor(&version, &cursor).await?;
        } else {
            if !store.embedding_backfill_complete(&version).await? {
                cursor.kind = SourceKind::Question;
                cursor.after = None;
                store.save_embedding_cursor(&version, &cursor).await?;
                continue;
            }
            store
                .set_discovery(DiscoveryStatus::Ready, Some(&version))
                .await?;
            cursor.kind = SourceKind::Question;
            cursor.after = None;
            store.save_embedding_cursor(&version, &cursor).await?;
            tokio::select! {_=cancel.cancelled()=>return Ok(()),_=tokio::time::sleep(std::time::Duration::from_secs(1))=>{}}
        }
    }
}
