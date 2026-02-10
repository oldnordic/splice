//! Batch operation support.
//!
//! Provides multi-file refactoring with YAML specification
//! and transaction-based rollback support.

pub mod spec;
pub mod transaction;

pub use spec::{BatchSpec, BatchOperation, PatchOp, DeleteOp, RenameOp, ExecutionMode, parse_batch_spec};
pub use spec::BatchSpecError as BatchError;
pub use transaction::{BatchTransaction, RollbackMode, TransactionResult};
