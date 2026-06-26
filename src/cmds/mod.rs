//! CLI command handlers, split by concern.
//!
//! Each submodule contains one or more `execute_*` functions that
//! adapt parsed CLI arguments into library calls.

pub(crate) mod apply;
pub(crate) mod batch;
pub(crate) mod dead_code;
pub(crate) mod delete;
pub(crate) mod edit;
pub(crate) mod graph;
pub(crate) mod helpers;
pub(crate) mod impact;
pub(crate) mod log;
pub(crate) mod patch;
pub(crate) mod patch_batch;
pub(crate) mod plan;
pub(crate) mod query;
pub(crate) mod rename;
pub(crate) mod search;
pub(crate) mod snapshots;
pub(crate) mod status;
pub(crate) mod suggest;
pub(crate) mod verify;
