//! Grounded code completion using Magellan database
//!
//! Provides project-aware code suggestions by querying actual codebase patterns
//! instead of relying on LLM training data.

pub mod context;
pub mod engine;
pub mod imports;
pub mod module_index;
pub mod ranking;
pub mod tokenizer;
pub mod types;

pub use context::CompletionContext;
pub use engine::CompletionEngine;
pub use imports::ImportEntity;
pub use types::{
    CompletionRequest, CompletionResponse, CompletionSuggestion, SuggestionSource, SymbolKind,
};
