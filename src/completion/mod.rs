//! Grounded code completion using Magellan database
//!
//! Provides project-aware code suggestions by querying actual codebase patterns
//! instead of relying on LLM training data.

/// Cursor context extraction for completion.
pub mod context;
/// Completion engine that queries the database for suggestions.
pub mod engine;
pub mod imports;
pub mod module_index;
/// Suggestion ranking and scoring.
pub mod ranking;
pub mod tokenizer;
/// Completion request/response types.
pub mod types;

pub use context::CompletionContext;
pub use engine::CompletionEngine;
pub use imports::ImportEntity;
pub use types::{
    CompletionRequest, CompletionResponse, CompletionSuggestion, SuggestionSource, SymbolKind,
};
