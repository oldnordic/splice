//! Integration tests for Splice.

mod backend_compatibility_tests;
mod backend_specific_tests;
mod cli_output_tests;
mod snapshot_tests;
mod migration_integration_tests;
mod compiler_error_tests;
mod cross_language_tests;
mod e2e_refactor_tests;
mod magellan_integration_tests;
mod rich_span_tests;

// Graph algorithm tests (Phase 30)
mod integration;
