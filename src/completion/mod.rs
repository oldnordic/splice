//! Grounded code completion using Magellan database
//!
//! Provides project-aware code suggestions by querying actual codebase patterns
//! instead of relying on LLM training data.

pub mod engine;
pub mod types;
pub mod context;
pub mod imports;
pub mod ranking;
pub mod tokenizer;

pub use engine::CompletionEngine;
pub use types::{
    CompletionSuggestion,
    CompletionRequest,
    CompletionResponse,
    SuggestionSource,
    SymbolKind,
};
pub use context::CompletionContext;
