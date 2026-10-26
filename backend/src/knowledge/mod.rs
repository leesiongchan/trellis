//! Server-owned library research lifecycle.
mod lifecycle;
mod research;
mod service;
pub use lifecycle::{KnowledgeRuntime, KnowledgeStatus};
pub use service::KnowledgeService;

pub mod matching;
pub mod verification;
