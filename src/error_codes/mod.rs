//! Error code registry for Splice diagnostics.
//!
//! Provides stable error codes in SPL-E### format with severity levels,
//! precise locations, and actionable hints for LLM consumption.

/// Error code explanations following the `rustc --explain` pattern.
pub mod explanations;
/// Error code registry: ErrorCode struct, ErrorSeverity enum, SpliceErrorCode enum.
pub mod registry;

pub use explanations::*;
pub use registry::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_format() {
        assert_eq!(SpliceErrorCode::SymbolNotFound.code(), "SPL-E001");
        assert_eq!(SpliceErrorCode::AmbiguousSymbol.code(), "SPL-E002");
        assert_eq!(SpliceErrorCode::ParseError.code(), "SPL-E011");
        assert_eq!(SpliceErrorCode::InvalidSpan.code(), "SPL-E021");
        assert_eq!(SpliceErrorCode::FileReadError.code(), "SPL-E031");
        assert_eq!(SpliceErrorCode::PreVerificationFailed.code(), "SPL-E041");
        assert_eq!(SpliceErrorCode::InvalidPlanSchema.code(), "SPL-E051");
        assert_eq!(SpliceErrorCode::GraphError.code(), "SPL-E061");
        assert_eq!(SpliceErrorCode::ExecutionLogError.code(), "SPL-E071");
        assert_eq!(SpliceErrorCode::AnalyzerNotAvailable.code(), "SPL-E081");
    }

    #[test]
    fn test_error_code_severity() {
        assert_eq!(SpliceErrorCode::SymbolNotFound.severity(), "error");
        assert_eq!(SpliceErrorCode::ParseError.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidSpan.severity(), "error");
    }

    #[test]
    fn test_error_code_has_hint() {
        let code = SpliceErrorCode::SymbolNotFound;
        let hint = code.hint();
        assert!(!hint.is_empty());
        assert!(hint.contains("symbol") || hint.contains("file"));
    }

    #[test]
    fn test_error_code_from_splice_error() {
        use crate::SpliceError;

        let symbol_error = SpliceError::symbol_not_found("foo", None);
        let code = SpliceErrorCode::from_splice_error(&symbol_error);
        assert_eq!(code, Some(SpliceErrorCode::SymbolNotFound));

        let ambiguous_error = SpliceError::AmbiguousSymbol {
            name: "foo".to_string(),
            files: vec!["a.rs".to_string(), "b.rs".to_string()],
        };
        let code = SpliceErrorCode::from_splice_error(&ambiguous_error);
        assert_eq!(code, Some(SpliceErrorCode::AmbiguousSymbol));
    }

    #[test]
    fn test_error_code_coverage() {
        use crate::SpliceError;
        use std::path::PathBuf;

        // Test all error-level SpliceError variants produce error codes
        let mut mapped_count = 0;

        // Symbol resolution errors
        let error = SpliceError::symbol_not_found("test", None);
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::AmbiguousSymbol {
            name: "foo".to_string(),
            files: vec!["a.rs".to_string()],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::ReferenceFailed {
            name: "foo".to_string(),
            reason: "test".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::AmbiguousReference {
            name: "foo".to_string(),
            file: "test.rs".to_string(),
            line: 1,
            col: 1,
            candidates: vec!["a::foo".to_string()],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Parse/AST errors
        let error = SpliceError::Parse {
            file: PathBuf::from("test.rs"),
            message: "test error".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // InvalidUtf8 - create by converting invalid bytes
        let invalid_bytes = vec![0xffu8, 0xfe];
        let invalid_utf8 = std::str::from_utf8(&invalid_bytes).unwrap_err();
        let error = SpliceError::InvalidUtf8 {
            file: PathBuf::from("test.rs"),
            source: invalid_utf8,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::CompilerError("syntax error".to_string());
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Span errors
        let error = SpliceError::InvalidSpan {
            file: PathBuf::from("test.rs"),
            start: 0,
            end: 10,
            file_size: 100,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::InvalidLineRange {
            file: PathBuf::from("test.rs"),
            line_start: 1,
            line_end: 10,
            total_lines: 20,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::FileExternallyModified {
            file: "test.rs".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // I/O errors
        let error = SpliceError::Io {
            path: PathBuf::from("test.rs"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::IoContext {
            context: "test context".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "test"),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::InsufficientDiskSpace {
            needed: 1000,
            available: 100,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Validation errors
        let error = SpliceError::PreVerificationFailed {
            check: "test check".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::ParseValidationFailed {
            file: PathBuf::from("test.rs"),
            message: "validation failed".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::CompilerValidationFailed {
            file: PathBuf::from("test.rs"),
            language: "rust".to_string(),
            diagnostics: vec![],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Plan execution errors
        let error = SpliceError::InvalidPlanSchema {
            message: "invalid schema".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::PlanExecutionFailed {
            step: 1,
            error: "failed".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::InvalidBatchSchema {
            message: "invalid batch".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Graph/database errors
        let error = SpliceError::Graph(sqlitegraph::SqliteGraphError::connection("test error"));
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Execution log errors
        let error = SpliceError::ExecutionLogError {
            message: "log error".to_string(),
            source: None,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::ExecutionNotFound {
            execution_id: "test-id".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::ExecutionRecordFailed {
            execution_id: "test-id".to_string(),
            source: None,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Analyzer errors
        let error = SpliceError::AnalyzerNotAvailable {
            mode: "path".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::AnalyzerFailed {
            output: "failed".to_string(),
            diagnostics: vec![],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Query errors
        let error = SpliceError::QueryError {
            message: "query failed".to_string(),
            source: None,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Date format errors
        let error = SpliceError::InvalidDateFormat {
            input: "invalid".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Cargo check failed (Rust-specific compiler validation)
        let error = SpliceError::CargoCheckFailed {
            workspace: PathBuf::from("/workspace"),
            output: "check failed".to_string(),
            diagnostics: vec![],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Intentionally unmapped errors
        let error = SpliceError::BrokenPipe;
        assert!(
            SpliceErrorCode::from_splice_error(&error).is_none(),
            "BrokenPipe should not have an error code (terminal state)"
        );

        // Utf8 variant - intentionally unmapped
        let invalid_bytes = vec![0xffu8, 0xfe];
        let invalid_utf8 = std::str::from_utf8(&invalid_bytes).unwrap_err();
        let error = SpliceError::Utf8(invalid_utf8);
        assert!(
            SpliceErrorCode::from_splice_error(&error).is_none(),
            "Utf8 variant should not have an error code (covered by InvalidUtf8)"
        );

        let error = SpliceError::Other("generic error".to_string());
        assert!(
            SpliceErrorCode::from_splice_error(&error).is_none(),
            "Other variant should not have an error code (generic catchall)"
        );

        // Verify we have at least 22 error-level variants mapped
        assert!(
            mapped_count >= 22,
            "Expected at least 22 error-level variants to be mapped, got {}",
            mapped_count
        );

        println!("Total error-level variants mapped: {}", mapped_count);
    }

    #[test]
    fn test_explain_command_generation() {
        use crate::SpliceError;

        // Test that explain_command is generated for errors with error codes
        let symbol_error = SpliceError::symbol_not_found("foo", None);
        let payload = crate::cli::CliErrorPayload::from_error(&symbol_error);

        // Should have error_code
        assert!(payload.error.error_code.is_some());
        // Should have explain_command
        let Some(explain_cmd) = payload.error.explain_command.as_ref() else {
            panic!("explain_command must be present for coded errors");
        };
        assert_eq!(explain_cmd, "splice explain --code SPL-E001");
        assert!(explain_cmd.contains("splice explain --code"));

        // Test that BrokenPipe (no error code) doesn't have explain_command
        let broken_pipe_error = SpliceError::BrokenPipe;
        let payload = crate::cli::CliErrorPayload::from_error(&broken_pipe_error);

        assert!(payload.error.error_code.is_none());
        assert!(payload.error.explain_command.is_none());

        // Test format for various error codes
        let test_cases = vec![
            (
                SpliceError::Parse {
                    file: std::path::PathBuf::from("test.rs"),
                    message: "parse error".to_string(),
                },
                "SPL-E011",
            ),
            (
                SpliceError::InvalidSpan {
                    file: std::path::PathBuf::from("test.rs"),
                    start: 0,
                    end: 10,
                    file_size: 100,
                },
                "SPL-E021",
            ),
            (
                SpliceError::Io {
                    path: std::path::PathBuf::from("test.rs"),
                    source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
                },
                "SPL-E031",
            ),
        ];

        for (error, expected_code) in test_cases {
            let payload = crate::cli::CliErrorPayload::from_error(&error);
            assert!(payload.error.error_code.is_some());
            let Some(explain_cmd) = payload.error.explain_command.as_ref() else {
                panic!("explain_command must be present for coded errors");
            };
            let expected = format!("splice explain --code {}", expected_code);
            assert_eq!(explain_cmd, &expected);
        }
    }

    #[test]
    fn test_error_code_construction() {
        let code = ErrorCode::new(
            "SPL-E001",
            "error",
            "src/main.rs:10:5",
            "Check symbol spelling",
        );

        assert_eq!(code.code, "SPL-E001");
        assert_eq!(code.severity, "error");
        assert_eq!(code.location, Some("src/main.rs:10:5".to_string()));
        assert_eq!(code.hint, "Check symbol spelling");
    }

    #[test]
    fn test_error_code_from_splice_code() {
        let error_code = ErrorCode::from_splice_code(
            SpliceErrorCode::SymbolNotFound,
            Some("src/main.rs"),
            Some(10),
            Some(5),
        );

        assert_eq!(error_code.code, "SPL-E001");
        assert_eq!(error_code.severity, "error");
        assert_eq!(error_code.location, Some("src/main.rs:10:5".to_string()));
        assert!(!error_code.hint.is_empty());
    }

    #[test]
    fn test_error_code_location_formats() {
        // Full location
        let ec = ErrorCode::from_splice_code(
            SpliceErrorCode::InvalidSpan,
            Some("test.rs"),
            Some(5),
            Some(10),
        );
        assert_eq!(ec.location, Some("test.rs:5:10".to_string()));

        // File only
        let ec = ErrorCode::from_splice_code(
            SpliceErrorCode::FileReadError,
            Some("test.rs"),
            None,
            None,
        );
        assert_eq!(ec.location, Some("test.rs".to_string()));

        // Unknown location
        let ec = ErrorCode::from_splice_code(SpliceErrorCode::GraphError, None, None, None);
        assert_eq!(ec.location, None);
    }

    #[test]
    fn test_error_severity_as_str() {
        assert_eq!(ErrorSeverity::Error.as_str(), "error");
        assert_eq!(ErrorSeverity::Warning.as_str(), "warning");
        assert_eq!(ErrorSeverity::Note.as_str(), "note");
    }

    #[test]
    fn test_error_code_severity_error() {
        // Test that error-level codes return "error"
        assert_eq!(SpliceErrorCode::SymbolNotFound.severity(), "error");
        assert_eq!(SpliceErrorCode::ReferenceFailed.severity(), "error");
        assert_eq!(SpliceErrorCode::ParseError.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidUtf8.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidSyntax.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidSpan.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidLineRange.severity(), "error");
        assert_eq!(SpliceErrorCode::SpanOutOfBounds.severity(), "error");
        assert_eq!(SpliceErrorCode::FileReadError.severity(), "error");
        assert_eq!(SpliceErrorCode::FileWriteError.severity(), "error");
        assert_eq!(SpliceErrorCode::FileNotFound.severity(), "error");
        assert_eq!(SpliceErrorCode::PreVerificationFailed.severity(), "error");
        assert_eq!(SpliceErrorCode::ParseValidationFailed.severity(), "error");
        assert_eq!(
            SpliceErrorCode::CompilerValidationFailed.severity(),
            "error"
        );
        assert_eq!(SpliceErrorCode::InvalidPlanSchema.severity(), "error");
        assert_eq!(SpliceErrorCode::PlanExecutionFailed.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidBatchSchema.severity(), "error");
        assert_eq!(SpliceErrorCode::GraphError.severity(), "error");
        assert_eq!(SpliceErrorCode::DatabaseError.severity(), "error");
        assert_eq!(SpliceErrorCode::ExecutionLogError.severity(), "error");
        assert_eq!(SpliceErrorCode::ExecutionNotFound.severity(), "error");
        assert_eq!(SpliceErrorCode::AnalyzerNotAvailable.severity(), "error");
        assert_eq!(SpliceErrorCode::AnalyzerFailed.severity(), "error");
    }

    #[test]
    fn test_error_code_severity_warning() {
        // Test that warning-level codes return "warning"
        assert_eq!(SpliceErrorCode::AmbiguousSymbol.severity(), "warning");
        assert_eq!(SpliceErrorCode::AmbiguousReference.severity(), "warning");
        assert_eq!(
            SpliceErrorCode::FileExternallyModified.severity(),
            "warning"
        );
        assert_eq!(
            SpliceErrorCode::AmbiguousSymbolAsWarning.severity(),
            "warning"
        );
        assert_eq!(SpliceErrorCode::FileSkipped.severity(), "warning");
        assert_eq!(
            SpliceErrorCode::FileExternallyModifiedWarning.severity(),
            "warning"
        );
    }

    #[test]
    fn test_error_code_all_have_severity() {
        // Test that all SpliceErrorCode variants have valid severity
        let all_codes = [
            SpliceErrorCode::SymbolNotFound,
            SpliceErrorCode::AmbiguousSymbol,
            SpliceErrorCode::ReferenceFailed,
            SpliceErrorCode::AmbiguousReference,
            SpliceErrorCode::ParseError,
            SpliceErrorCode::InvalidUtf8,
            SpliceErrorCode::InvalidSyntax,
            SpliceErrorCode::InvalidSpan,
            SpliceErrorCode::InvalidLineRange,
            SpliceErrorCode::SpanOutOfBounds,
            SpliceErrorCode::FileReadError,
            SpliceErrorCode::FileWriteError,
            SpliceErrorCode::FileNotFound,
            SpliceErrorCode::FileExternallyModified,
            SpliceErrorCode::AmbiguousSymbolAsWarning,
            SpliceErrorCode::FileSkipped,
            SpliceErrorCode::FileExternallyModifiedWarning,
            SpliceErrorCode::PreVerificationFailed,
            SpliceErrorCode::ParseValidationFailed,
            SpliceErrorCode::CompilerValidationFailed,
            SpliceErrorCode::InvalidPlanSchema,
            SpliceErrorCode::PlanExecutionFailed,
            SpliceErrorCode::InvalidBatchSchema,
            SpliceErrorCode::GraphError,
            SpliceErrorCode::DatabaseError,
            SpliceErrorCode::ExecutionLogError,
            SpliceErrorCode::ExecutionNotFound,
            SpliceErrorCode::AnalyzerNotAvailable,
            SpliceErrorCode::AnalyzerFailed,
        ];

        for code in all_codes.iter() {
            let severity = code.severity();
            assert!(
                severity == "error" || severity == "warning" || severity == "note",
                "Code {:?} has invalid severity: {}",
                code,
                severity
            );
        }
    }

    #[test]
    fn test_warning_code_format() {
        // Test that warning codes use SPL-W### format
        assert_eq!(SpliceErrorCode::AmbiguousSymbolAsWarning.code(), "SPL-W001");
        assert_eq!(SpliceErrorCode::FileSkipped.code(), "SPL-W002");
        assert_eq!(
            SpliceErrorCode::FileExternallyModifiedWarning.code(),
            "SPL-W003"
        );
    }

    #[test]
    fn test_warning_code_from_splice_code() {
        // Test that warning codes produce proper ErrorCode with warning severity
        let warning_code = ErrorCode::from_splice_code(
            SpliceErrorCode::AmbiguousSymbolAsWarning,
            Some("src/main.rs"),
            Some(10),
            Some(5),
        );

        assert_eq!(warning_code.code, "SPL-W001");
        assert_eq!(warning_code.severity, "warning");
        assert_eq!(warning_code.location, Some("src/main.rs:10:5".to_string()));
        assert!(!warning_code.hint.is_empty());
    }
}
