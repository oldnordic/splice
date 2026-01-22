//! Splice: Span-safe refactoring kernel for Rust.
//!
//! This library provides byte-accurate, AST-validated refactoring operations
//! for Rust code using SQLiteGraph as the ground-truth code graph.

#![warn(missing_docs)]
// env_logger is used by src/main.rs (binary), not this library
#![expect(unused_crate_dependencies)]

pub mod action;
pub mod checksum;
pub mod cli;
pub mod context;
pub mod diff;
pub mod error;
pub mod error_codes;
pub mod execution;
pub mod graph;
pub mod hints;
pub mod ingest;
pub mod output;
pub mod patch;
pub mod plan;
pub mod relationships;
pub mod resolve;
pub mod symbol;
pub mod validate;
pub mod verify;

/// Re-export common error types for convenience.
pub use error::{Result, SpliceError};

/// Re-export graph types for convenience.
pub use graph::CodeGraph;

/// Re-export context types for convenience.
pub use context::{extract_context, extract_context_asymmetric, extract_context_with_before_after};
pub use output::SpanContext;

/// Re-export diff utilities for convenience.
pub use diff::{format_unified_diff, should_use_color, format_colored_diff, format_diff_summary};

/// Re-export error codes for convenience.
pub use error_codes::{ErrorCode, ErrorSeverity, SpliceErrorCode};

/// Re-export semantic kind detection for convenience.
pub use ingest::{detect_semantic_kind, SemanticKind};

/// Re-export tool hints for convenience.
pub use hints::{ToolHints, ToolHintOperation, derive_tool_hints};

/// Re-export action types for convenience.
pub use action::{SuggestedAction, ActionType, Confidence, suggest_action};

/// Re-export relationship types for convenience.
pub use relationships::{Relationship, Relationships, RelationshipCache};

/// Splice version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
