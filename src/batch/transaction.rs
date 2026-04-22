//! Transaction-based batch execution with automatic rollback.
//!
//! Provides ACID-like semantics for batch refactoring operations.
//! On failure, automatically rolls back using snapshot restore.

use crate::batch::executor::BatchExecutor;
use crate::batch::spec::BatchSpec;
use crate::error::{Result, SpliceError};
use crate::graph::CodeGraph;
use crate::proof::storage::SnapshotStorage;
use std::path::{Path, PathBuf};

/// Configuration for batch transaction behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackMode {
    /// Never rollback (default for --continue-on-error)
    Never,
    /// Rollback on any failure (default for stop-on-error)
    OnFailure,
    /// Always rollback after successful batch (for testing)
    Always,
}

/// Transaction result with rollback information.
#[derive(Debug, Clone)]
pub struct TransactionResult {
    /// The batch execution result
    pub batch_result: BatchResult,
    /// Whether a rollback was performed
    pub rolled_back: bool,
    /// Path to the snapshot used for rollback (if any)
    pub rollback_snapshot: Option<PathBuf>,
    /// Time taken for rollback (ms)
    pub rollback_duration_ms: Option<u64>,
}

/// Result of executing a batch operation.
/// Re-exported from executor module for convenience.
pub use crate::batch::executor::BatchResult;

/// A batch transaction with automatic rollback support.
pub struct BatchTransaction {
    /// Database path for snapshots
    db_path: PathBuf,
    /// Rollback mode
    rollback_mode: RollbackMode,
    /// Whether to capture snapshot before execution
    snapshot_before: bool,
}

impl BatchTransaction {
    /// Create a new batch transaction.
    ///
    /// # Arguments
    /// * `db_path` - Path to the code graph database (required for snapshots)
    /// * `rollback_mode` - When to perform rollback
    /// * `snapshot_before` - Whether to capture snapshot before execution
    pub fn new(db_path: PathBuf, rollback_mode: RollbackMode, snapshot_before: bool) -> Self {
        Self {
            db_path,
            rollback_mode,
            snapshot_before,
        }
    }

    /// Execute a batch spec with automatic rollback on failure.
    pub fn execute(&self, spec: &BatchSpec, dry_run: bool) -> Result<TransactionResult> {
        use std::time::Instant;

        // Capture pre-execution snapshot if needed
        let snapshot_path = if self.snapshot_before || self.rollback_mode != RollbackMode::Never {
            Some(self.capture_pre_snapshot(spec)?)
        } else {
            None
        };

        // Execute the batch
        let start = Instant::now();
        let batch_result = self.execute_batch(spec, dry_run)?;
        let total_duration_ms = start.elapsed().as_millis() as u64;

        let batch_result = BatchResult {
            total_duration_ms,
            ..batch_result
        };

        // Determine if rollback is needed
        let should_rollback = match self.rollback_mode {
            RollbackMode::Never => false,
            RollbackMode::OnFailure => batch_result.failed > 0,
            RollbackMode::Always => true,
        };

        // Perform rollback if needed
        let (rolled_back, rollback_duration_ms) = if should_rollback {
            if let Some(ref snapshot) = snapshot_path {
                let start = std::time::Instant::now();
                self.perform_rollback(snapshot)?;
                let duration = start.elapsed().as_millis() as u64;
                (true, Some(duration))
            } else {
                // Rollback requested but no snapshot available
                eprintln!("Warning: Rollback requested but no snapshot available");
                (false, None)
            }
        } else {
            (false, None)
        };

        Ok(TransactionResult {
            batch_result,
            rolled_back,
            rollback_snapshot: snapshot_path,
            rollback_duration_ms,
        })
    }

    /// Execute a batch spec using the provided executor.
    fn execute_batch(&self, spec: &BatchSpec, dry_run: bool) -> Result<BatchResult> {
        // Create executor with the same dry_run setting
        let mut executor = BatchExecutor::new(dry_run, Some(self.db_path.clone()));
        executor.execute(spec)
    }

    /// Capture a snapshot before batch execution.
    fn capture_pre_snapshot(&self, spec: &BatchSpec) -> Result<PathBuf> {
        use crate::proof::generation::generate_snapshot;

        let storage = SnapshotStorage::new()?;
        let snapshot = generate_snapshot(&self.db_path)?;

        // Use description from spec or default
        let operation = spec
            .metadata
            .description
            .as_ref()
            .map(|d| format!("batch-{}", d.replace(' ', "-")))
            .unwrap_or_else(|| "batch".to_string());

        let metadata = storage.save_snapshot(&operation, &self.db_path, snapshot)?;
        eprintln!("Pre-batch snapshot: {}", metadata.snapshot_path.display());

        Ok(metadata.snapshot_path)
    }

    /// Perform rollback from a snapshot.
    fn perform_rollback(&self, snapshot_path: &Path) -> Result<()> {
        // Verify backend is native-v3 (restore only works with native-v3)
        let backend = CodeGraph::detect_backend(&self.db_path)?;
        if backend != crate::graph::BackendType::NativeV3 {
            return Err(SpliceError::Other(format!(
                "Rollback requires native-v3 backend, detected: {}",
                backend
            )));
        }

        // Perform restore
        eprintln!("Rolling back from snapshot: {}", snapshot_path.display());
        let _result = SnapshotStorage::restore_from_snapshot(&self.db_path, snapshot_path)?;

        Ok(())
    }

    /// Clean up a snapshot after successful batch (optional).
    pub fn cleanup_snapshot(&self, snapshot_path: &Path) -> Result<()> {
        std::fs::remove_file(snapshot_path)
            .map_err(|e| SpliceError::Other(format!("Failed to cleanup snapshot: {}", e)))?;
        Ok(())
    }
}
