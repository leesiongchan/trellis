use super::{Coordinator, Outcome};
use crate::{
    contracts::ResearchResult,
    error::{AppError, ErrorCode},
};
use std::sync::{Arc, Weak};
use tokio::sync::watch;
pub struct Waiter {
    pub(super) owner: Weak<Coordinator>,
    pub(super) key: String,
    pub(super) generation: String,
    pub(super) receiver: watch::Receiver<Option<Outcome>>,
}
impl Waiter {
    pub fn job_id(&self) -> &str {
        &self.generation
    }
    pub async fn result(&mut self) -> Result<Arc<ResearchResult>, Arc<AppError>> {
        loop {
            if let Some(result) = self.receiver.borrow().clone() {
                return result;
            }
            if self.receiver.changed().await.is_err() {
                return Err(Arc::new(AppError::new(ErrorCode::InternalFailure)));
            }
        }
    }
}
impl Drop for Waiter {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            owner.release(&self.key, &self.generation);
        }
    }
}
