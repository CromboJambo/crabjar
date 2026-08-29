pub mod context;
pub mod error;
pub mod habitat;
pub mod models;
pub mod schema;
pub mod state_docs;
pub mod store;

pub use context::{
    ContextBudget, ContextError, ContextFragment, ContextFragmentBuilder, ContextQueryResult,
    DEFAULT_CONTEXT_BUDGET, MAX_TOKENS_PER_FRAGMENT, P0_ALERT_TOKENS, estimate_tokens,
};
pub use error::{Error, Result};
pub use models::{EventKind, KnowledgeEntry, KnowledgeKind, KnowledgeRow, Source};
pub use state_docs::StateDocQuerier;
pub use state_docs::models::{
    Annotation, CodeBlock, ConfidenceAssessment, DocMetadata, Section, Table,
};
pub use store::Store;
