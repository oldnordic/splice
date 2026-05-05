//! Proof-based refactoring support.
//!
//! This module provides machine-checkable behavioral equivalence proofs
//! for refactoring operations. A proof captures before/after graph state
//! and validates that structural invariants are preserved.

pub mod checksums;
pub mod comparison;
/// Data structures for proof snapshots and invariants.
pub mod data_structures;
pub mod generation;
pub mod storage;
pub mod validation;

pub use checksums::{
    compute_proof_checksums, compute_snapshot_hash, validate_proof_checksums, validate_proof_file,
};

pub use data_structures::{
    GraphSnapshot, GraphStats, InvariantCheck, InvariantViolation, ProofChecksums, ProofMetadata,
    RefactoringProof, SymbolInfo, ViolationSeverity,
};

pub use generation::{create_metadata, generate_snapshot, write_proof};

pub use storage::{RestoreResult, SnapshotMetadata, SnapshotStorage};

pub use validation::validate_invariants;

pub use comparison::{compare_snapshots, ChangeType, EdgeDiff, SnapshotDiff, SymbolDiff};

use crate::error::Result;
use std::path::Path;

/// Generate a proof for a refactoring operation.
///
/// This function should be called before and after a refactoring
/// to capture the graph state and validate invariants.
///
/// # Arguments
/// * `operation` - The type of refactoring operation (e.g., "rename", "delete")
/// * `db_path` - Path to the Magellan database
/// * `before_snapshot` - Graph state before the refactoring
/// * `after_snapshot` - Graph state after the refactoring
///
/// # Returns
/// A complete refactoring proof with invariant validation results and checksums.
///
/// # Examples
///
/// ```no_run
/// use splice::proof::{generate_proof, generate_snapshot};
/// use std::path::Path;
///
/// # fn main() -> splice::error::Result<()> {
/// let db_path = Path::new(".magellan/splice.db");
///
/// // Capture before state
/// let before = generate_snapshot(db_path)?;
///
/// // Perform refactoring...
///
/// // Capture after state
/// let after = generate_snapshot(db_path)?;
///
/// // Generate proof with checksums
/// let proof = generate_proof("rename", db_path, before, after)?;
///
/// // Validate invariants
/// for check in &proof.invariants {
///     if !check.passed {
///         eprintln!("Invariant violation: {}", check.invariant_name);
///         for violation in &check.violations {
///             eprintln!("  - {}: {}", violation.subject, violation.message);
///         }
///     }
/// }
///
/// // Write proof to file
/// let output_dir = Path::new(".splice/proofs/");
/// let proof_path = splice::proof::write_proof(&proof, output_dir)?;
/// println!("Proof written to: {}", proof_path.display());
///
/// // Later, validate the proof file
/// let is_valid = splice::proof::validate_proof_file(&proof_path)?;
/// assert!(is_valid, "Proof checksums are invalid!");
///
/// # Ok(())
/// # }
/// ```
pub fn generate_proof(
    operation: &str,
    db_path: &Path,
    before_snapshot: GraphSnapshot,
    after_snapshot: GraphSnapshot,
) -> Result<RefactoringProof> {
    // Validate invariants
    let invariants = validate_invariants(&before_snapshot, &after_snapshot)?;

    // Create metadata
    let metadata = create_metadata(operation, db_path);

    // Create proof without checksums first
    let mut proof = RefactoringProof {
        metadata,
        before: before_snapshot,
        after: after_snapshot,
        invariants,
        checksums: None,
    };

    // Compute and set checksums for audit trail integrity
    let checksums = compute_proof_checksums(&proof)?;
    proof.checksums = Some(checksums);

    Ok(proof)
}
