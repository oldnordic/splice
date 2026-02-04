//! Proof-based refactoring support.
//!
//! This module provides machine-checkable behavioral equivalence proofs
//! for refactoring operations. A proof captures before/after graph state
//! and validates that structural invariants are preserved.

pub mod data_structures;
pub mod generation;

pub use data_structures::{
    RefactoringProof, GraphSnapshot, ProofMetadata, InvariantCheck,
    InvariantViolation, ViolationSeverity, ProofChecksums,
    SymbolInfo, GraphStats,
};

pub use generation::{
    generate_snapshot, create_metadata, write_proof,
};

use crate::error::Result;
use std::path::Path;

/// Generate a proof for a refactoring operation.
///
/// This function should be called before and after a refactoring
/// to capture the graph state and validate invariants.
pub fn generate_proof(
    operation: &str,
    db_path: &Path,
    before_snapshot: GraphSnapshot,
    after_snapshot: GraphSnapshot,
) -> Result<RefactoringProof> {
    // Implementation in 31-02
    todo!()
}

/// Validate that refactoring invariants are preserved.
///
/// Checks:
/// - Reference counts are preserved (same number of incoming/outgoing edges)
/// - No orphaned symbols (all symbols remain reachable from entry points)
/// - Symbol IDs are stable (no new IDs generated, only name changes)
pub fn validate_invariants(
    before: &GraphSnapshot,
    after: &GraphSnapshot,
) -> Result<Vec<InvariantCheck>> {
    // Implementation in 31-03
    todo!()
}
