//! Splice: Span-safe refactoring kernel for Rust.
//!
//! This library provides byte-accurate, AST-validated refactoring operations
//! for Rust code using SQLiteGraph as the ground-truth code graph.

#![warn(missing_docs, unused_crate_dependencies)]

pub mod cli;
pub mod error;
pub mod graph;
pub mod ingest;
pub mod patch;
pub mod plan;
pub mod resolve;
pub mod validate;

/// Re-export common error types for convenience.
pub use error::{Result, SpliceError};

/// Re-export graph types for convenience.
pub use graph::CodeGraph;

/// Splice version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
