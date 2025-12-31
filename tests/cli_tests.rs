//! Integration tests for CLI wiring.
//!
//! These tests validate that the CLI is a thin adapter over existing APIs
//! with proper error handling and exit codes.

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use splice::ingest::rust::extract_rust_symbols;
    use std::collections::HashMap;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::{NamedTempFile, TempDir};

    /// Get the path to the splice binary.
    fn get_splice_binary() -> PathBuf {
        // During testing, use cargo to build/run the binary
        let mut path = std::env::current_exe().unwrap();
        // This test binary is in target/debug/deps/
        // The splice binary is in target/debug/
        path.pop(); // deps
        path.pop(); // debug
        path.push("splice");
        path
    }

    /// Test A: Successful CLI patch.
    ///
    /// This test creates a temp Rust workspace, calls the CLI via std::process::Command,
    /// and verifies:
    /// - Exit code == 0
    /// - File content updated exactly at resolved span
    /// - cargo check still passes
    #[test]
    fn test_cli_successful_patch() {
        // Create temporary workspace directory
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create Cargo.toml
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        // Create src directory
        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        // Create lib.rs with function to patch
        let lib_rs_path = src_dir.join("lib.rs");
        let source = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#;

        std::fs::write(&lib_rs_path, source).expect("Failed to write lib.rs");

        // Create patch file with replacement
        let patch_path = workspace_path.join("patch.rs");
        let patch_content = r#"
pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
}
"#;

        std::fs::write(&patch_path, patch_content).expect("Failed to write patch file");

        // Create graph database path
        let _graph_db_path = workspace_path.join("splice_graph.db");

        // First, we need to ingest symbols into the graph
        // TODO: This will require implementing the ingest command
        // For now, we'll skip this and just verify the CLI can be invoked

        // Call CLI: splice patch --file src/lib.rs --symbol greet --with patch.rs
        let splice_binary = get_splice_binary();

        let output = Command::new(&splice_binary)
            .arg("patch")
            .arg("--file")
            .arg(&lib_rs_path)
            .arg("--symbol")
            .arg("greet")
            .arg("--with")
            .arg(&patch_path)
            .current_dir(workspace_path)
            .output();

        // For now, the CLI isn't implemented, so we expect it to fail
        // After implementation, this should succeed (exit code 0)
        match output {
            Ok(result) => {
                // After implementation, this should be:
                // assert_eq!(result.status.code(), Some(0), "CLI should succeed");
                // assert!(result.status.success(), "CLI should exit with success");

                // For now, just verify the binary can be invoked
                println!("CLI exit code: {:?}", result.status.code());
                println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
                println!("stderr: {}", String::from_utf8_lossy(&result.stderr));
            }
            Err(e) => {
                // Binary doesn't exist yet - that's OK for this test
                println!("Failed to execute splice binary: {}", e);
            }
        }
    }

    /// Test B: Ambiguous symbol fails.
    ///
    /// This test verifies that when two files define the same symbol name,
    /// the CLI fails with proper error message.
    ///
    /// Expected behavior:
    /// - Exit code != 0
    /// - stderr contains "AmbiguousSymbol"
    #[test]
    fn test_cli_ambiguous_symbol_fails() {
        // TODO: Implement after CLI patch command is functional
        // This test will:
        // 1. Create workspace with 2 files defining same symbol
        // 2. Ingest both files into graph
        // 3. Call: splice patch --symbol foo --with patch.rs (without --file)
        // 4. Assert exit code != 0
        // 5. Assert stderr contains "AmbiguousSymbol"
    }

    /// Test C: Syntax failure propagates.
    ///
    /// This test verifies that when a replacement introduces a syntax error,
    /// the CLI fails and rolls back the original file.
    ///
    /// Expected behavior:
    /// - Exit code != 0
    /// - Original file unchanged
    #[test]
    fn test_cli_syntax_failure_propagates() {
        // TODO: Implement after CLI patch command is functional
        // This test will:
        // 1. Create workspace with valid function
        // 2. Ingest symbols into graph
        // 3. Call: splice patch --file src/lib.rs --symbol foo --with invalid_patch.rs
        // 4. Assert exit code != 0
        // 5. Assert original file unchanged (atomic rollback)
    }

    /// Test D: Analyzer off by default.
    ///
    /// This test verifies that the CLI succeeds with rust-analyzer OFF by default.
    ///
    /// Expected behavior:
    /// - Exit code == 0
    /// - No rust-analyzer invocation
    /// - Patch succeeds with tree-sitter + cargo check only
    #[test]
    fn test_analyzer_off_by_default() {
        // Create temporary workspace directory
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create Cargo.toml
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        // Create src directory
        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        // Create lib.rs with function to patch
        let lib_rs_path = src_dir.join("lib.rs");
        let source = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#;

        std::fs::write(&lib_rs_path, source).expect("Failed to write lib.rs");

        // Create patch file with replacement
        let patch_path = workspace_path.join("patch.rs");
        let patch_content = r#"
pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
}
"#;

        std::fs::write(&patch_path, patch_content).expect("Failed to write patch file");

        // Call CLI WITHOUT --analyzer flag (should default to off)
        let splice_binary = get_splice_binary();

        let output = Command::new(&splice_binary)
            .arg("patch")
            .arg("--file")
            .arg(&lib_rs_path)
            .arg("--symbol")
            .arg("greet")
            .arg("--with")
            .arg(&patch_path)
            .current_dir(workspace_path)
            .output();

        match output {
            Ok(result) => {
                // Should succeed with exit code 0
                assert_eq!(
                    result.status.code(),
                    Some(0),
                    "CLI should succeed with analyzer off by default"
                );
                assert!(result.status.success(), "CLI should exit with success");
            }
            Err(e) => {
                panic!("Failed to execute splice binary: {}", e);
            }
        }
    }

    /// Test E: Analyzer required but missing.
    ///
    /// This test verifies that when --analyzer os is specified but rust-analyzer
    /// is not available, the CLI fails with proper error.
    ///
    /// Expected behavior:
    /// - Exit code != 0
    /// - stderr contains "AnalyzerNotAvailable"
    #[test]
    fn test_analyzer_required_but_missing() {
        // TODO: This test requires --analyzer CLI flag first
        // Will implement in STEP 3 after CLI wiring is complete
    }

    /// Test F: Analyzer failure causes rollback.
    ///
    /// This test verifies that when rust-analyzer reports diagnostics,
    /// the CLI fails and rolls back the original file.
    ///
    /// Expected behavior:
    /// - Exit code != 0
    /// - stderr contains "AnalyzerFailed"
    /// - Original file unchanged
    #[test]
    fn test_analyzer_failure_causes_rollback() {
        // TODO: This test requires rust-analyzer gate implementation
        // Will implement in STEP 2-3 after analyzer gate is complete
    }

    /// Test G: Successful plan execution.
    ///
    /// This test verifies that a plan with multiple valid steps
    /// executes all patches sequentially and succeeds.
    ///
    /// Expected behavior:
    /// - Exit code == 0
    /// - All patches applied
    /// - cargo check passes
    #[test]
    fn test_plan_execution_success() {
        // Create temporary workspace directory
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Create Cargo.toml
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        // Create src directory
        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        // Create lib.rs with two functions to patch
        let lib_rs_path = src_dir.join("lib.rs");
        let source = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

pub fn farewell(name: &str) -> String {
    format!("Goodbye, {}!", name)
}
"#;

        std::fs::write(&lib_rs_path, source).expect("Failed to write lib.rs");

        // Create patches directory
        let patches_dir = workspace_path.join("patches");
        std::fs::create_dir(&patches_dir).expect("Failed to create patches directory");

        // Create first patch file
        let patch1_path = patches_dir.join("greet.rs");
        let patch1_content = r#"
pub fn greet(name: &str) -> String {
    format!("Greetings, {}!", name)
}
"#;

        std::fs::write(&patch1_path, patch1_content).expect("Failed to write patch1");

        // Create second patch file
        let patch2_path = patches_dir.join("farewell.rs");
        let patch2_content = r#"
pub fn farewell(name: &str) -> String {
    format!("Farewell, {}!", name)
}
"#;

        std::fs::write(&patch2_path, patch2_content).expect("Failed to write patch2");

        // Create plan.json
        let plan_path = workspace_path.join("plan.json");
        let plan_content = r#"{
  "steps": [
    {
      "file": "src/lib.rs",
      "symbol": "greet",
      "kind": "function",
      "with": "patches/greet.rs"
    },
    {
      "file": "src/lib.rs",
      "symbol": "farewell",
      "kind": "function",
      "with": "patches/farewell.rs"
    }
  ]
}
"#;

        std::fs::write(&plan_path, plan_content).expect("Failed to write plan.json");

        // TODO: This test requires plan CLI command
        // Will implement in STEP 3 after CLI wiring is complete
    }

    /// Test H: Failure stops execution.
    ///
    /// This test verifies that when a step fails, execution stops
    /// and previous successful steps remain applied.
    ///
    /// Expected behavior:
    /// - Exit code != 0
    /// - First patch applied
    /// - Second patch NOT applied
    /// - File state is correct (first patch applied, second not)
    #[test]
    fn test_plan_failure_stops_execution() {
        // TODO: This test requires plan execution implementation
        // Will implement in STEP 2-3 after plan execution is complete
    }

    /// Test I: Invalid plan schema.
    ///
    /// This test verifies that invalid plan schemas are rejected
    /// with clear error messages.
    ///
    /// Expected behavior:
    /// - Exit code != 0
    /// - Clear error message about schema issue
    #[test]
    fn test_plan_invalid_schema() {
        // TODO: This test requires plan parsing implementation
        // Will implement in STEP 2 after plan parsing is complete
    }

    /// Test J: Symbol not found returns structured JSON payload.
    #[test]
    fn test_cli_symbol_not_found_returns_structured_json() {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Minimal Cargo.toml
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        // Source tree
        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(
            &lib_rs_path,
            r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#,
        )
        .expect("Failed to write lib.rs");

        // Replacement file for CLI invocation
        let patch_path = workspace_path.join("patch.rs");
        std::fs::write(
            &patch_path,
            r#"
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
"#,
        )
        .expect("Failed to write patch.rs");

        let splice_binary = get_splice_binary();
        let output = Command::new(&splice_binary)
            .arg("patch")
            .arg("--file")
            .arg(&lib_rs_path)
            .arg("--symbol")
            .arg("missing_symbol")
            .arg("--with")
            .arg(&patch_path)
            .current_dir(workspace_path)
            .output()
            .expect("Failed to run splice CLI");

        assert!(
            !output.status.success(),
            "CLI should fail when symbol cannot be resolved"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        let payload: Value =
            serde_json::from_str(&stderr).expect("stderr should contain JSON payload");

        assert_eq!(
            payload.get("status").and_then(|v| v.as_str()),
            Some("error"),
            "status should be error"
        );

        let error = payload
            .get("error")
            .and_then(|v| v.as_object())
            .expect("error object missing");

        assert_eq!(
            error.get("kind").and_then(|v| v.as_str()),
            Some("SymbolNotFound"),
            "kind should be SymbolNotFound"
        );

        assert_eq!(
            error.get("symbol").and_then(|v| v.as_str()),
            Some("missing_symbol"),
            "symbol field should echo missing symbol"
        );

        assert_eq!(
            error.get("file").and_then(|v| v.as_str()),
            lib_rs_path.to_str(),
            "file should reference requested source file"
        );

        assert!(
            error.get("hint").and_then(|v| v.as_str()).is_some(),
            "hint should be populated for guidance"
        );
    }

    /// Test K: Syntax errors emit diagnostics in the JSON payload.
    #[test]
    fn test_cli_patch_syntax_error_emits_diagnostics() {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Cargo manifest
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        // Valid source file
        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");
        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(
            &lib_rs_path,
            r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#,
        )
        .expect("Failed to write lib.rs");

        // Replacement patch missing closing brace to trigger syntax failure
        let patch_path = workspace_path.join("patch.rs");
        std::fs::write(
            &patch_path,
            r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
"#,
        )
        .expect("Failed to write patch.rs");

        let splice_binary = get_splice_binary();
        let output = Command::new(&splice_binary)
            .arg("patch")
            .arg("--file")
            .arg(&lib_rs_path)
            .arg("--symbol")
            .arg("greet")
            .arg("--with")
            .arg(&patch_path)
            .current_dir(workspace_path)
            .output()
            .expect("Failed to execute splice CLI");

        assert!(
            !output.status.success(),
            "CLI should fail when patch introduces syntax errors"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        let payload: Value =
            serde_json::from_str(&stderr).expect("stderr should contain JSON payload");

        let error = payload
            .get("error")
            .and_then(|v| v.as_object())
            .expect("error object missing");

        let diagnostics = error
            .get("diagnostics")
            .and_then(|v| v.as_array())
            .expect("diagnostics array missing from payload");

        assert!(
            !diagnostics.is_empty(),
            "diagnostics array should contain at least one entry"
        );

        let first = diagnostics[0]
            .as_object()
            .expect("diagnostic entry should be an object");

        assert_eq!(
            first.get("tool").and_then(|v| v.as_str()),
            Some("tree-sitter"),
            "tree-sitter should report syntax errors"
        );
        assert!(
            first
                .get("message")
                .and_then(|v| v.as_str())
                .map(|m| m.contains("syntax"))
                .unwrap_or(false),
            "diagnostic message should mention syntax issues"
        );
    }

    /// Test L: Cargo check failures emit file-specific diagnostics.
    #[test]
    fn test_cli_cargo_check_failure_emits_diagnostics() {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Cargo manifest
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        // Source file
        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");
        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(
            &lib_rs_path,
            r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#,
        )
        .expect("Failed to write lib.rs");

        // Replacement patch referencing missing function (compile error, but syntax OK)
        let patch_path = workspace_path.join("patch.rs");
        std::fs::write(
            &patch_path,
            r#"
pub fn greet(name: &str) -> String {
    missing_helper(name)
}
"#,
        )
        .expect("Failed to write patch.rs");

        let splice_binary = get_splice_binary();
        let output = Command::new(&splice_binary)
            .arg("patch")
            .arg("--file")
            .arg(&lib_rs_path)
            .arg("--symbol")
            .arg("greet")
            .arg("--with")
            .arg(&patch_path)
            .current_dir(workspace_path)
            .output()
            .expect("Failed to execute splice CLI");

        assert!(
            !output.status.success(),
            "CLI should fail when cargo check reports errors"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        let payload: Value =
            serde_json::from_str(&stderr).expect("stderr should contain JSON payload");

        let error = payload
            .get("error")
            .and_then(|v| v.as_object())
            .expect("error object missing");

        assert_eq!(
            error.get("kind").and_then(|v| v.as_str()),
            Some("CargoCheckFailed"),
            "expected CargoCheckFailed error kind"
        );

        let diagnostics = error
            .get("diagnostics")
            .and_then(|v| v.as_array())
            .expect("diagnostics array missing");
        assert!(
            !diagnostics.is_empty(),
            "diagnostics should not be empty for cargo failures"
        );

        let first = diagnostics[0]
            .as_object()
            .expect("diagnostic entry should be an object");
        assert_eq!(
            first.get("tool").and_then(|v| v.as_str()),
            Some("cargo-check"),
            "cargo-check diagnostics expected"
        );
        let file_value = first
            .get("file")
            .and_then(|v| v.as_str())
            .expect("diagnostic should include file path");
        assert!(
            file_value.ends_with("src/lib.rs"),
            "diagnostic should point to the patched file"
        );
        assert!(
            first.get("line").and_then(|v| v.as_u64()).is_some(),
            "diagnostic should contain a line number"
        );
        assert_eq!(
            first.get("code").and_then(|v| v.as_str()),
            Some("E0425"),
            "diagnostic should expose compiler error code"
        );
        let tool_version = first
            .get("tool_version")
            .and_then(|v| v.as_str())
            .expect("tool_version should be present");
        assert!(
            tool_version.to_lowercase().contains("cargo"),
            "tool_version should describe cargo"
        );
        assert!(
            first
                .get("tool_path")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false),
            "tool_path should contain the detected binary path"
        );
        assert_eq!(
            first.get("remediation").and_then(|v| v.as_str()),
            Some("https://doc.rust-lang.org/error-index.html#E0425"),
            "remediation link should point to the Rust error index"
        );
    }

    /// Test M: Batch patch CLI rolls back when validation fails.
    #[test]
    fn test_cli_batch_patch_rolls_back_on_failure() {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        // Cargo manifest
        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(
            &lib_rs_path,
            r#"
pub fn helper(x: i32) -> i32 {
    x + 1
}

pub mod a;
pub mod b;
"#,
        )
        .expect("Failed to write lib.rs");

        let file_a = src_dir.join("a.rs");
        std::fs::write(
            &file_a,
            r#"
use crate::helper;

pub fn value() -> i32 {
    helper(10)
}
"#,
        )
        .expect("Failed to write a.rs");

        let file_b = src_dir.join("b.rs");
        std::fs::write(
            &file_b,
            r#"
use crate::helper;

pub fn broken() -> i32 {
    helper(5)
}
"#,
        )
        .expect("Failed to write b.rs");

        let symbols_a = extract_rust_symbols(
            &file_a,
            std::fs::read(&file_a).expect("read a.rs").as_slice(),
        )
        .expect("parse a.rs");
        let span_a = symbols_a
            .iter()
            .find(|s| s.name == "value")
            .expect("value span");

        let symbols_b = extract_rust_symbols(
            &file_b,
            std::fs::read(&file_b).expect("read b.rs").as_slice(),
        )
        .expect("parse b.rs");
        let span_b = symbols_b
            .iter()
            .find(|s| s.name == "broken")
            .expect("broken span");

        let relative_a = file_a
            .strip_prefix(workspace_path)
            .expect("a.rs relative path");
        let relative_b = file_b
            .strip_prefix(workspace_path)
            .expect("b.rs relative path");

        let batch_path = workspace_path.join("batch.json");
        let batch_json = json!({
            "batches": [
                {
                    "replacements": [
                        {
                            "file": relative_a,
                            "start": span_a.byte_start,
                            "end": span_a.byte_end,
                            "content": r#"
pub fn value() -> i32 {
    helper(42)
}
"#
                        },
                        {
                            "file": relative_b,
                            "start": span_b.byte_start,
                            "end": span_b.byte_end,
                            "content": r#"
pub fn broken() -> i32 {
    helper("oops")
}
"#
                        }
                    ]
                }
            ]
        });
        std::fs::write(
            &batch_path,
            serde_json::to_string_pretty(&batch_json).unwrap(),
        )
        .expect("write batch.json");

        let original_a = std::fs::read_to_string(&file_a).expect("read original a.rs");
        let original_b = std::fs::read_to_string(&file_b).expect("read original b.rs");

        let splice_binary = get_splice_binary();
        let output = Command::new(&splice_binary)
            .arg("patch")
            .arg("--batch")
            .arg(&batch_path)
            .arg("--language")
            .arg("rust")
            .current_dir(workspace_path)
            .output()
            .expect("Failed to run splice CLI");

        assert!(
            !output.status.success(),
            "CLI should fail because the second replacement introduces a type error"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        let payload: Value = serde_json::from_str(&stderr).expect("stderr should contain JSON");
        assert_eq!(
            payload.get("status").and_then(|v| v.as_str()),
            Some("error")
        );
        assert_eq!(
            payload
                .get("error")
                .and_then(|v| v.get("kind"))
                .and_then(|v| v.as_str()),
            Some("CargoCheckFailed"),
            "batch failures should return CargoCheckFailed errors"
        );

        assert_eq!(
            original_a,
            std::fs::read_to_string(&file_a).expect("read patched a.rs"),
            "a.rs should remain unchanged when batch fails"
        );
        assert_eq!(
            original_b,
            std::fs::read_to_string(&file_b).expect("read patched b.rs"),
            "b.rs should remain unchanged when batch fails"
        );
    }

    /// Test N: Batch patch success emits per-file metadata.
    #[test]
    fn test_cli_batch_patch_success_returns_metadata() {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(
            &lib_rs_path,
            r#"
pub fn helper(x: i32) -> i32 {
    x + 1
}

pub mod a;
"#,
        )
        .expect("Failed to write lib.rs");

        let file_a = src_dir.join("a.rs");
        std::fs::write(
            &file_a,
            r#"
use crate::helper;

pub fn value() -> i32 {
    helper(10)
}
"#,
        )
        .expect("Failed to write a.rs");

        let file_b = src_dir.join("b.rs");
        std::fs::write(
            &file_b,
            r#"
use crate::helper;

pub fn broken() -> i32 {
    helper(5)
}
"#,
        )
        .expect("Failed to write b.rs");

        let symbols_a = extract_rust_symbols(
            &file_a,
            std::fs::read(&file_a).expect("read a.rs").as_slice(),
        )
        .expect("parse a.rs");
        let span_a = symbols_a
            .iter()
            .find(|s| s.name == "value")
            .expect("value span");

        let symbols_b = extract_rust_symbols(
            &file_b,
            std::fs::read(&file_b).expect("read b.rs").as_slice(),
        )
        .expect("parse b.rs");
        let span_b = symbols_b
            .iter()
            .find(|s| s.name == "broken")
            .expect("broken span");

        let relative_a = file_a
            .strip_prefix(workspace_path)
            .expect("a.rs relative path");
        let relative_b = file_b
            .strip_prefix(workspace_path)
            .expect("b.rs relative path");

        let batch_path = workspace_path.join("batch-success.json");
        let batch_json = json!({
            "batches": [
                {
                    "replacements": [
                        {
                            "file": relative_a,
                            "start": span_a.byte_start,
                            "end": span_a.byte_end,
                            "content": r#"
pub fn value() -> i32 {
    helper(42)
}
"#
                        },
                        {
                            "file": relative_b,
                            "start": span_b.byte_start,
                            "end": span_b.byte_end,
                            "content": r#"
pub fn broken() -> i32 {
    helper(7)
}
"#
                        }
                    ]
                }
            ]
        });
        std::fs::write(
            &batch_path,
            serde_json::to_string_pretty(&batch_json).unwrap(),
        )
        .expect("write batch-success.json");

        let before_hash_a = hash_file(&file_a);
        let before_hash_b = hash_file(&file_b);

        let splice_binary = get_splice_binary();
        let output = Command::new(&splice_binary)
            .arg("patch")
            .arg("--batch")
            .arg(&batch_path)
            .arg("--language")
            .arg("rust")
            .current_dir(workspace_path)
            .output()
            .expect("Failed to run splice CLI");

        if !output.status.success() {
            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        assert!(
            output.status.success(),
            "CLI should succeed when both replacements are valid"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let payload: Value = serde_json::from_str(&stdout).expect("stdout should be JSON payload");
        assert_eq!(
            payload.get("status").and_then(|v| v.as_str()),
            Some("ok"),
            "success payload should set status=ok"
        );

        let data = payload
            .get("data")
            .and_then(|v| v.as_object())
            .expect("success payload should include metadata");

        assert_eq!(
            data.get("batches_applied").and_then(|v| v.as_u64()),
            Some(1),
            "metadata should report number of batches"
        );

        let files = data
            .get("files")
            .and_then(|v| v.as_array())
            .expect("metadata should include per-file entries");
        assert_eq!(files.len(), 2, "two files should be reported");

        let after_hash_a = hash_file(&file_a);
        let after_hash_b = hash_file(&file_b);

        let mut expected = HashMap::new();
        expected.insert(
            file_a.to_string_lossy().to_string(),
            (before_hash_a.clone(), after_hash_a.clone()),
        );
        expected.insert(
            file_b.to_string_lossy().to_string(),
            (before_hash_b.clone(), after_hash_b.clone()),
        );

        for entry in files {
            let obj = entry
                .as_object()
                .expect("file metadata entries should be JSON objects");
            let file = obj
                .get("file")
                .and_then(|v| v.as_str())
                .expect("file entry should be a string");
            let before = obj
                .get("before_hash")
                .and_then(|v| v.as_str())
                .expect("before_hash should be a string");
            let after = obj
                .get("after_hash")
                .and_then(|v| v.as_str())
                .expect("after_hash should be a string");

            let (expected_before, expected_after) = expected
                .get(file)
                .unwrap_or_else(|| panic!("unexpected file in metadata: {}", file));
            assert_eq!(
                before, expected_before,
                "before hash mismatch for file {}",
                file
            );
            assert_eq!(
                after, expected_after,
                "after hash mismatch for file {}",
                file
            );
        }

        let final_a = std::fs::read_to_string(&file_a).expect("read final a.rs");
        assert!(
            final_a.contains("helper(42)"),
            "file a.rs should reflect the batch replacement"
        );
        let final_b = std::fs::read_to_string(&file_b).expect("read final b.rs");
        assert!(
            final_b.contains("helper(7)"),
            "file b.rs should reflect the batch replacement"
        );
    }

    #[test]
    fn test_cli_patch_preview() {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(
            &lib_rs_path,
            r#"
pub fn helper(x: i32) -> i32 {
    x + 1
}

pub mod a;
"#,
        )
        .expect("Failed to write lib.rs");

        let a_rs_path = src_dir.join("a.rs");
        std::fs::write(
            &a_rs_path,
            r#"
use crate::helper;

pub fn value() -> i32 {
    helper(10)
}
"#,
        )
        .expect("Failed to write a.rs");

        let patch_path = workspace_path.join("patch.rs");
        std::fs::write(
            &patch_path,
            r#"
pub fn value() -> i32 {
    helper(20)
}
"#,
        )
        .expect("Failed to write patch file");

        let original_content =
            std::fs::read_to_string(&a_rs_path).expect("Failed to read original file");

        let splice_binary = get_splice_binary();
        let output = Command::new(&splice_binary)
            .arg("patch")
            .arg("--file")
            .arg(&a_rs_path)
            .arg("--symbol")
            .arg("value")
            .arg("--with")
            .arg(&patch_path)
            .arg("--preview")
            .current_dir(workspace_path)
            .output()
            .expect("Failed to run splice CLI");

        assert!(
            output.status.success(),
            "CLI preview should exit successfully"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let payload: Value = serde_json::from_str(&stdout).expect("stdout should be JSON payload");

        let data = payload
            .get("data")
            .and_then(|v| v.as_object())
            .expect("data missing");

        let preview_report = data
            .get("preview_report")
            .and_then(|v| v.as_object())
            .expect("preview_report missing");

        assert_eq!(
            preview_report
                .get("file")
                .and_then(|v| v.as_str())
                .expect("file missing in preview_report"),
            a_rs_path.to_string_lossy()
        );

        assert!(
            preview_report
                .get("lines_added")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );

        assert!(
            preview_report
                .get("bytes_removed")
                .and_then(|v| v.as_u64())
                .is_some(),
            "preview_report must include bytes_removed"
        );

        assert_eq!(
            original_content,
            std::fs::read_to_string(&a_rs_path).expect("file unchanged after preview")
        );

        let files = data
            .get("files")
            .and_then(|v| v.as_array())
            .expect("files array missing");
        assert_eq!(files.len(), 1);

        assert_eq!(
            files[0]
                .get("file")
                .and_then(|v| v.as_str())
                .expect("file entry missing"),
            a_rs_path.to_string_lossy()
        );
    }

    /// Test O: Backup creation and undo restores files.
    #[test]
    fn test_cli_backup_and_undo() {
        let workspace_dir = TempDir::new().expect("Failed to create temp workspace");
        let workspace_path = workspace_dir.path();

        let cargo_toml_path = workspace_path.join("Cargo.toml");
        let mut cargo_toml = NamedTempFile::new().expect("Failed to create Cargo.toml");
        write!(
            cargo_toml,
            r#"[package]
name = "temp-test"
version = "0.1.0"
edition = "2021"

[lib]
name = "temp_test"
path = "src/lib.rs"
"#
        )
        .expect("Failed to write Cargo.toml");
        std::fs::rename(cargo_toml.path(), &cargo_toml_path).expect("Failed to move Cargo.toml");

        let src_dir = workspace_path.join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src directory");

        let lib_rs_path = src_dir.join("lib.rs");
        let original_content = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#;
        std::fs::write(&lib_rs_path, original_content)
            .expect("Failed to write lib.rs");

        let patch_path = workspace_path.join("patch.rs");
        std::fs::write(
            &patch_path,
            r#"
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
"#,
        )
        .expect("Failed to write patch.rs");

        // Extract symbol span
        let symbols = extract_rust_symbols(
            &lib_rs_path,
            std::fs::read(&lib_rs_path).expect("read lib.rs").as_slice(),
        )
        .expect("parse lib.rs");
        let span = symbols
            .iter()
            .find(|s| s.name == "greet")
            .expect("greet span");

        // Create batch JSON with the replacement
        let batch_path = workspace_path.join("batch.json");
        let batch_json = json!({
            "batches": [
                {
                    "replacements": [
                        {
                            "file": lib_rs_path.strip_prefix(workspace_path).unwrap(),
                            "start": span.byte_start,
                            "end": span.byte_end,
                            "content": r#"
pub fn greet(name: &str) -> String {
    format!("Hi, {}!", name)
}
"#
                        }
                    ]
                }
            ]
        });
        std::fs::write(
            &batch_path,
            serde_json::to_string_pretty(&batch_json).unwrap(),
        )
        .expect("write batch.json");

        let splice_binary = get_splice_binary();

        // Run patch with --create-backup
        let output = Command::new(&splice_binary)
            .arg("patch")
            .arg("--batch")
            .arg(&batch_path)
            .arg("--language")
            .arg("rust")
            .arg("--create-backup")
            .current_dir(workspace_path)
            .output()
            .expect("Failed to run splice CLI");

        assert!(
            output.status.success(),
            "CLI should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify file was modified
        let modified_content =
            std::fs::read_to_string(&lib_rs_path).expect("read modified lib.rs");
        assert!(
            modified_content.contains("Hi, "),
            "File should be patched"
        );
        assert!(
            !modified_content.contains("Hello, "),
            "Old content should be gone"
        );

        // Get backup manifest path from response
        let stdout = String::from_utf8_lossy(&output.stdout);
        let payload: Value = serde_json::from_str(&stdout).expect("stdout should be JSON");
        let backup_manifest = payload
            .get("data")
            .and_then(|v| v.get("backup_manifest"))
            .and_then(|v| v.as_str())
            .expect("backup_manifest should be in response");

        let manifest_path = std::path::PathBuf::from(backup_manifest);

        // Run undo command
        let undo_output = Command::new(&splice_binary)
            .arg("undo")
            .arg("--manifest")
            .arg(&manifest_path)
            .current_dir(workspace_path)
            .output()
            .expect("Failed to run splice undo");

        assert!(
            undo_output.status.success(),
            "Undo should succeed: {}",
            String::from_utf8_lossy(&undo_output.stderr)
        );

        // Verify file was restored
        let restored_content =
            std::fs::read_to_string(&lib_rs_path).expect("read restored lib.rs");
        assert_eq!(
            restored_content, original_content,
            "File should be restored to original content"
        );
    }

    fn hash_file(path: &std::path::Path) -> String {
        let bytes = std::fs::read(path).expect("Failed to read file for hashing");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    }
}
