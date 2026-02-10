//! Batch operation executor.
//!
//! Executes multi-file refactoring operations from a BatchSpec.

use crate::batch::spec::{BatchSpec, BatchOperation, ExecutionMode, PatchOp, DeleteOp, RenameOp};
use crate::batch::transaction::{BatchTransaction, RollbackMode, TransactionResult};
use crate::error::{Result, SpliceError};
use crate::graph::{CodeGraph, MagellanIntegration};
use crate::symbol::Symbol;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Result of executing a single operation.
#[derive(Debug, Clone)]
pub struct OperationResult {
    /// Operation index (1-based)
    pub index: usize,
    /// Operation type
    pub op_type: String,
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Duration of operation
    pub duration_ms: u64,
}

/// Result of executing a batch spec.
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Path to the batch spec file
    pub spec_path: PathBuf,
    /// Total operations in spec
    pub total_operations: usize,
    /// Number of successful operations
    pub successful: usize,
    /// Number of failed operations
    pub failed: usize,
    /// Individual operation results
    pub operations: Vec<OperationResult>,
    /// Total duration
    pub total_duration_ms: u64,
    /// Whether execution was stopped early
    pub stopped_early: bool,
}

/// Executor for batch refactoring operations.
pub struct BatchExecutor {
    /// Dry-run mode (preview only, no changes)
    dry_run: bool,
    /// Database path for graph operations
    db_path: Option<PathBuf>,
}

impl BatchExecutor {
    /// Create a new batch executor.
    pub fn new(dry_run: bool, db_path: Option<PathBuf>) -> Self {
        Self { dry_run, db_path }
    }

    /// Execute a batch specification.
    pub fn execute(&mut self, spec: &BatchSpec) -> Result<BatchResult> {
        let start = Instant::now();
        let total_operations = spec.operations.len();
        let mut operations = Vec::with_capacity(total_operations);
        let mut successful = 0;
        let mut failed = 0;
        let mut stopped_early = false;

        for (index, op) in spec.operations.iter().enumerate() {
            let op_start = Instant::now();
            let op_index = index + 1; // 1-based for user display
            let op_type = self.operation_type_name(op);

            let result = self.execute_operation(op, op_index, &spec.mode);

            let duration_ms = op_start.elapsed().as_millis() as u64;
            let is_success = result.is_ok();
            let error_msg = result.err().map(|e| e.to_string());

            let op_result = OperationResult {
                index: op_index,
                op_type: op_type.clone(),
                success: is_success,
                error: error_msg,
                duration_ms,
            };

            // Report progress
            self.report_progress(&op_result);

            match (is_success, spec.mode) {
                (true, _) => successful += 1,
                (false, ExecutionMode::StopOnError) => {
                    failed += 1;
                    stopped_early = true;
                }
                (false, ExecutionMode::ContinueOnError) => {
                    failed += 1;
                }
            }

            operations.push(op_result);

            if stopped_early {
                break;
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;

        Ok(BatchResult {
            spec_path: PathBuf::from("<unknown>"), // Set by caller
            total_operations,
            successful,
            failed,
            operations,
            total_duration_ms,
            stopped_early,
        })
    }

    /// Execute a batch as a transaction with automatic rollback.
    ///
    /// This is a convenience method that creates a BatchTransaction
    /// and executes the spec with rollback support.
    pub fn execute_transaction(
        &mut self,
        spec: &BatchSpec,
        dry_run: bool,
        rollback_mode: RollbackMode,
    ) -> Result<TransactionResult>
    where
        Self: Sized,
    {
        let db_path = self.db_path.as_ref().ok_or_else(|| {
            SpliceError::Other("Transaction requires database path for snapshots".to_string())
        })?;

        let transaction = BatchTransaction::new(
            db_path.clone(),
            rollback_mode,
            true, // Always snapshot before transaction
        );

        transaction.execute(spec, dry_run)
    }

    fn execute_operation(
        &mut self,
        op: &BatchOperation,
        index: usize,
        _mode: &ExecutionMode,
    ) -> Result<()> {
        match op {
            BatchOperation::Patch(patch_op) => self.execute_patch(patch_op, index),
            BatchOperation::Delete(delete_op) => self.execute_delete(delete_op, index),
            BatchOperation::Rename(rename_op) => self.execute_rename(rename_op, index),
        }
    }

    fn execute_patch(&mut self, op: &PatchOp, index: usize) -> Result<()> {
        // Check if snapshot before is requested
        if op.snapshot_before {
            if let Some(db_path) = &self.db_path {
                self.capture_snapshot(db_path, &format!("batch-patch-{}", index))?;
            }
        }

        // Read the replacement content
        let replacement = std::fs::read_to_string(&op.with)
            .map_err(|e| SpliceError::Other(format!("Failed to read replacement file '{}': {}",
                op.with.display(), e)))?;

        // We need to resolve the symbol to get its span
        let db_path = self.db_path.as_ref().ok_or_else(|| {
            SpliceError::Other("Batch patch operations require --db flag for symbol resolution".to_string())
        })?;

        let mut code_graph = CodeGraph::open(db_path)?;

        // Read source file to extract symbols
        let source = std::fs::read(&op.file)?;

        // Detect language
        let language = crate::symbol::Language::from_path(&op.file)
            .ok_or_else(|| SpliceError::Parse {
                file: op.file.clone(),
                message: "Cannot detect language - unknown file extension".to_string(),
            })?;

        // Extract symbols using language-aware dispatcher
        let symbols = crate::ingest::extract_symbols_with_language(&op.file, &source, language)?;

        // Store symbols in graph
        for symbol in &symbols {
            code_graph.store_symbol_with_file_and_language(
                &op.file,
                symbol.name(),
                symbol.kind(),
                symbol.language(),
                symbol.byte_start(),
                symbol.byte_end(),
                symbol.line_start(),
                symbol.line_end(),
                symbol.col_start(),
                symbol.col_end(),
            )?;
        }

        // Resolve symbol to get span
        let kind_str = op.kind.as_deref();
        let resolved = crate::resolve::resolve_symbol(&code_graph, Some(&op.file), kind_str, &op.symbol)?;

        // Get workspace directory
        let workspace_dir = op.file.parent().ok_or_else(|| {
            SpliceError::Other("Cannot determine workspace directory".to_string())
        })?;

        // Apply patch (with dry-run support)
        if self.dry_run {
            // Preview mode - just show what would change
            eprintln!("[PREVIEW] Would patch {}::{} in file: {}",
                     kind_str.unwrap_or("symbol"), op.symbol, op.file.display());
            Ok(())
        } else {
            // Actual patch
            crate::patch::apply_patch_with_validation(
                &op.file,
                resolved.byte_start,
                resolved.byte_end,
                &replacement,
                workspace_dir,
                language,
                crate::validate::AnalyzerMode::Off,
            )?;
            Ok(())
        }
    }

    fn execute_delete(&mut self, op: &DeleteOp, index: usize) -> Result<()> {
        if op.snapshot_before {
            if let Some(db_path) = &self.db_path {
                self.capture_snapshot(db_path, &format!("batch-delete-{}", index))?;
            }
        }

        let db_path = self.db_path.as_ref().ok_or_else(|| {
            SpliceError::Other("Batch delete operations require --db flag for symbol resolution".to_string())
        })?;

        let mut code_graph = CodeGraph::open(db_path)?;

        // Read source file to extract symbols
        let source = std::fs::read(&op.file)?;

        // Detect language
        let language = crate::symbol::Language::from_path(&op.file)
            .ok_or_else(|| SpliceError::Parse {
                file: op.file.clone(),
                message: "Cannot detect language - unknown file extension".to_string(),
            })?;

        // Extract symbols using language-aware dispatcher
        let symbols = crate::ingest::extract_symbols_with_language(&op.file, &source, language)?;

        // Store symbols in graph
        for symbol in &symbols {
            code_graph.store_symbol_with_file_and_language(
                &op.file,
                symbol.name(),
                symbol.kind(),
                symbol.language(),
                symbol.byte_start(),
                symbol.byte_end(),
                symbol.line_start(),
                symbol.line_end(),
                symbol.col_start(),
                symbol.col_end(),
            )?;
        }

        // Resolve symbol to get span
        let kind_str = op.kind.as_deref();
        let resolved = crate::resolve::resolve_symbol(&code_graph, Some(&op.file), kind_str, &op.symbol)?;

        // Delete means replacing with empty string
        let workspace_dir = op.file.parent().ok_or_else(|| {
            SpliceError::Other("Cannot determine workspace directory".to_string())
        })?;

        if self.dry_run {
            eprintln!("[PREVIEW] Would delete {}::{} in file: {}",
                     kind_str.unwrap_or("symbol"), op.symbol, op.file.display());
            Ok(())
        } else {
            crate::patch::apply_patch_with_validation(
                &op.file,
                resolved.byte_start,
                resolved.byte_end,
                "", // Empty replacement for delete
                workspace_dir,
                language,
                crate::validate::AnalyzerMode::Off,
            )?;
            Ok(())
        }
    }

    fn execute_rename(&mut self, op: &RenameOp, index: usize) -> Result<()> {
        if op.snapshot_before {
            if let Some(db_path) = &self.db_path {
                self.capture_snapshot(db_path, &format!("batch-rename-{}", index))?;
            }
        }

        // For rename, we use MagellanIntegration to find references
        let db_path = self.db_path.as_ref().ok_or_else(|| {
            SpliceError::Other("Batch rename operations require --db flag".to_string())
        })?;

        let mut magellan = MagellanIntegration::open(db_path)?;

        // Find all symbols with the given name in the file
        let mut matches = magellan.find_symbol_by_name(&op.from, true)?;

        // Filter to the specified file
        let file_path_str = op.file.to_string_lossy().to_string();
        matches.retain(|s| s.file_path == file_path_str);

        if matches.is_empty() {
            return Err(SpliceError::Other(format!(
                "Symbol '{}' not found in file '{}'",
                op.from,
                op.file.display()
            )));
        }

        // Use the first match
        let symbol_info = matches.into_iter().next().unwrap();

        // Get references using entity_id
        let references = magellan.get_all_references(symbol_info.entity_id)?;

        if references.is_empty() {
            eprintln!("Warning: No references found for symbol '{}'", op.from);
        }

        // Filter by files if specified
        let filtered_refs = if let Some(ref files) = op.files {
            let file_set: std::collections::HashSet<String> = files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            references.into_iter()
                .filter(|r| file_set.contains(&r.file_path.to_string_lossy().to_string()))
                .collect()
        } else {
            references
        };

        if self.dry_run {
            eprintln!("[PREVIEW] Would rename '{}' to '{}' in {} file(s)",
                     op.from, op.to, filtered_refs.len());
            Ok(())
        } else {
            // Group references by file
            let grouped = crate::graph::rename::group_references_by_file(&filtered_refs);

            // Apply replacements in each file
            for (file_path, refs) in grouped {
                crate::graph::rename::apply_replacements_in_file(
                    &file_path,
                    &op.from,
                    &op.to,
                    &refs,
                )?;
            }

            Ok(())
        }
    }

    fn capture_snapshot(&self, db_path: &Path, operation: &str) -> Result<()> {
        use crate::proof::storage::SnapshotStorage;
        use crate::proof::generation::generate_snapshot;

        let storage = SnapshotStorage::new()?;
        let snapshot = generate_snapshot(db_path)?;
        storage.save_snapshot(operation, db_path, snapshot)?;
        Ok(())
    }

    fn operation_type_name(&self, op: &BatchOperation) -> String {
        match op {
            BatchOperation::Patch(_) => "patch".to_string(),
            BatchOperation::Delete(_) => "delete".to_string(),
            BatchOperation::Rename(_) => "rename".to_string(),
        }
    }

    fn report_progress(&self, result: &OperationResult) {
        let status = if result.success { "+" } else { "x" };
        eprintln!(
            "[{}] Op {}/{}: {} ({})",
            status,
            result.index,
            "?", // Total not known yet
            result.op_type,
            result.duration_ms
        );
        if let Some(error) = &result.error {
            eprintln!("     Error: {}", error);
        }
    }
}
