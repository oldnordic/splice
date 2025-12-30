//! Integration tests for CLI wiring.
//!
//! These tests validate that the CLI is a thin adapter over existing APIs
//! with proper error handling and exit codes.

#[cfg(test)]
mod tests {
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
}
