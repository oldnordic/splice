//! Batch operation support.
//!
//! Provides multi-file refactoring with YAML specification
//! and transaction-based rollback support.

pub mod executor;
pub mod spec;
pub mod transaction;

pub use executor::{BatchExecutor, BatchResult, OperationResult};
pub use spec::BatchSpecError as BatchError;
pub use spec::{
    parse_batch_spec, BatchOperation, BatchSpec, DeleteOp, ExecutionMode, PatchOp, RenameOp,
};
pub use transaction::{BatchTransaction, RollbackMode, TransactionResult};
