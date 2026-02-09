//! Batch operation support.
//!
//! Provides multi-file refactoring with YAML specification.

pub mod spec;

pub use spec::{BatchSpec, BatchOperation, PatchOp, DeleteOp, RenameOp, parse_batch_spec};
pub use spec::BatchSpecError as BatchError;
