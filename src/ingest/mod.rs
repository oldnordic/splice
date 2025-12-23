//! Filesystem → AST → SQLiteGraph ingestion pipeline.
//!
//! This module handles reading Rust source files, parsing them with
//! tree-sitter, and storing symbols and spans in the code graph.

pub mod rust;

use crate::error::Result;
use crate::graph::CodeGraph;
use std::path::Path;

/// Main ingest orchestrator.
///
/// Reads Rust files from the filesystem, parses them with tree-sitter,
/// and stores symbols/spans in the SQLiteGraph database.
pub struct Ingestor {
    /// Graph database handle.
    graph: CodeGraph,
}

impl Ingestor {
    /// Create a new ingestor with the given graph database.
    pub fn new(graph: CodeGraph) -> Self {
        Self { graph }
    }

    /// Ingest a single Rust source file.
    pub fn ingest_file(&mut self, path: &Path) -> Result<()> {
        // TODO: Implement in Task 1
        Err(crate::error::SpliceError::Other(
            "Not implemented yet".to_string(),
        ))
    }

    /// Ingest a directory of Rust source files recursively.
    pub fn ingest_dir(&mut self, path: &Path) -> Result<()> {
        // TODO: Implement in Task 1
        Err(crate::error::SpliceError::Other(
            "Not implemented yet".to_string(),
        ))
    }
}
