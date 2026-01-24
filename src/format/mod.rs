//! Format utilities for cross-tool compatibility.
//!
//! This module provides translation and formatting utilities for interoperability
//! with external tools, particularly Magellan.
//!
//! ## Field Name Translation
//!
//! The primary focus is translating between different field naming conventions:
//! - Magellan uses `start_line`/`end_line` and `start_col`/`end_col`
//! - Splice uses `line_start`/`line_end` and `col_start`/`col_end`
//!
//! ## Modules
//!
//! - [`magellan`]: Bidirectional field translation with Magellan format

pub mod magellan;

/// Re-exports for commonly used types and functions.
pub use magellan::{from_magellan, to_magellan, translate_field_name, MagellanSpan, SpliceSpan};
