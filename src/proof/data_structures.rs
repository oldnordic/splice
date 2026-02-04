use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A complete refactoring proof.
///
/// Contains before/after snapshots of the code graph, invariant checks,
/// and metadata for audit trails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringProof {
    /// Proof metadata (operation, timestamp, etc.)
    pub metadata: ProofMetadata,
    /// Graph state before the refactoring
    pub before: GraphSnapshot,
    /// Graph state after the refactoring
    pub after: GraphSnapshot,
    /// Invariant validation results
    pub invariants: Vec<InvariantCheck>,
    /// Checksums for audit trail (added in 31-04)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksums: Option<ProofChecksums>,
}

/// Metadata about the proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    /// Operation type (e.g., "rename", "delete", "extract")
    pub operation: String,
    /// User who performed the operation (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// When the operation was performed
    pub timestamp: i64,
    /// Git commit hash (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
    /// Splice version
    pub splice_version: String,
    /// Database path used for the operation
    pub database_path: PathBuf,
}

/// A snapshot of the code graph at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    /// Snapshot timestamp
    pub timestamp: i64,
    /// All symbols in the graph
    pub symbols: HashMap<String, SymbolInfo>,
    /// All edges (caller -> callees)
    pub edges: HashMap<String, Vec<String>>,
    /// Entry points (roots of the call graph)
    pub entry_points: Vec<String>,
    /// Statistics about the snapshot
    pub stats: GraphStats,
}

/// Information about a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    /// Unique symbol ID
    pub id: String,
    /// Symbol name
    pub name: String,
    /// File path
    pub file_path: String,
    /// Symbol kind (function, struct, etc.)
    pub kind: String,
    /// Byte span
    pub byte_span: (usize, usize),
    /// Number of incoming references (fan-in)
    pub fan_in: usize,
    /// Number of outgoing references (fan-out)
    pub fan_out: usize,
}

/// Statistics about a graph snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    /// Total number of symbols
    pub total_symbols: usize,
    /// Total number of edges
    pub total_edges: usize,
    /// Number of entry points
    pub entry_point_count: usize,
    /// Maximum cyclomatic complexity (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_complexity: Option<usize>,
}

/// Result of an invariant check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantCheck {
    /// Name of the invariant being checked
    pub invariant_name: String,
    /// Whether the invariant passed
    pub passed: bool,
    /// Violations found (empty if passed)
    pub violations: Vec<InvariantViolation>,
}

/// An invariant violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantViolation {
    /// Severity of the violation
    pub severity: ViolationSeverity,
    /// Symbol or edge where violation occurred
    pub subject: String,
    /// Description of the violation
    pub message: String,
    /// Suggested fix (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Severity of an invariant violation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    /// Informational (no impact on correctness)
    Info,
    /// Warning (may indicate a problem)
    Warning,
    /// Error (definitely breaks invariants)
    Error,
    /// Critical (makes the codebase unusable)
    Critical,
}

/// Checksums for proof integrity (added in 31-04).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofChecksums {
    /// SHA-256 hash of the before snapshot
    pub before_hash: String,
    /// SHA-256 hash of the after snapshot
    pub after_hash: String,
    /// SHA-256 hash of the combined proof
    pub proof_hash: String,
}
