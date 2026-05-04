//! Splice: Span-safe refactoring kernel for Rust.
//!
//! This library provides byte-accurate, AST-validated refactoring operations
//! for Rust code using SQLiteGraph as the ground-truth code graph.

#![warn(missing_docs)]
// env_logger is used by src/main.rs (binary), not this library
#![expect(unused_crate_dependencies)]

pub mod action;
pub mod batch;
pub mod cfg_analysis;
pub mod checksum;
pub mod cli;
pub mod commands;
pub mod completion;
pub mod context;
pub mod create;
pub mod diff;
pub mod error;
pub mod error_codes;
pub mod execution;
pub mod expand;
pub mod format;
pub mod graph;
pub mod hints;
pub mod ingest;
pub mod output;
pub mod patch;
pub mod plan;
pub mod platform;
pub mod proof;
pub mod relationships;
pub mod resolve;
pub mod suggestions;
pub mod symbol;
pub mod symbol_id;
pub mod syntax_validator;
pub mod validate;
pub mod verify;
pub mod write;

/// Re-export common error types for convenience.
pub use error::{Result, SpliceError};

/// Re-export graph types for convenience.
pub use graph::CodeGraph;

/// Re-export migration types for convenience.
pub use graph::migrate::{check_schema_version, migrate_database, MigrationResult};

/// Re-export rename types for convenience.
pub use graph::rename::{
    apply_replacements_in_file, apply_with_rollback, create_rename_backup,
    generate_colored_preview, generate_preview_diff, group_references_by_file, replace_at_span,
    simulate_replacements, simulate_replacements_content, RenameBackupManifest, RenameTransaction,
};

/// Re-export context types for convenience.
pub use context::{
    extract_context, extract_context_asymmetric, extract_context_with_before_after,
    resolve_context_counts,
};
pub use output::SpanContext;

/// Re-export diff utilities for convenience.
pub use diff::{format_colored_diff, format_diff_summary, format_unified_diff, should_use_color};

/// Re-export error codes for convenience.
pub use error_codes::{get_error_explanation, ErrorCode, ErrorSeverity, SpliceErrorCode};

/// Re-export semantic kind detection for convenience.
pub use ingest::{detect_semantic_kind, SemanticKind};

/// Re-export tool hints for convenience.
pub use hints::{derive_tool_hints, ToolHintOperation, ToolHints};

/// Re-export action types for convenience.
pub use action::{suggest_action, ActionType, Confidence, SuggestedAction};

/// Re-export relationship types for convenience.
pub use relationships::{Relationship, RelationshipCache, Relationships};

/// Re-export expansion API for convenience.
pub use expand::{expand_symbol, expand_symbol_with_level, ExpansionLevel, SymbolExpander};

/// Re-export batch operations for convenience.
pub use batch::{
    BatchExecutor, BatchOperation, BatchResult, BatchSpec, BatchTransaction, ExecutionMode,
    RollbackMode, TransactionResult,
};

// ============================================================================
// Graph Algorithm Response Types (Phase 30)
// ============================================================================

/// Re-export graph algorithm response types for convenience.
pub use output::{
    AffectedFile,
    // Condensation graph types
    CondensationResult,
    CondensedScc,
    // Cycle detection types
    CycleDetectionResult,
    CycleInfo,
    DeadCodeByFile,
    // Dead code detection types
    DeadCodeResult,
    DeadSymbol,
    LevelInfo,
    // Reachability analysis types
    ReachabilityResult,
    ReachableSymbol,
    SccEdge,
    // Program slicing types
    SliceResult,
    SliceStats,
    SlicedSymbol,
    // Symbol info (shared across graph algorithms)
    SymbolInfo,
};

// ============================================================================
// CLI Enums (Phase 30)
// ============================================================================

/// Re-export CLI enums for programmatic use.
pub use cli::{ReachabilityDirection, SliceDirection};

// ============================================================================
// Proof-Based Refactoring Types (Phase 31)
// ============================================================================

/// Re-export proof types for convenience.
pub use proof::{
    GraphSnapshot, GraphStats, InvariantCheck, InvariantViolation, ProofChecksums, ProofMetadata,
    RefactoringProof, RestoreResult, SnapshotMetadata, SnapshotStorage,
    SymbolInfo as ProofSymbolInfo, ViolationSeverity,
};

/// Splice version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
