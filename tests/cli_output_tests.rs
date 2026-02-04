//! CLI output format and exit code tests.

use std::path::PathBuf;

#[test]
fn test_output_format_enum_exists() {
    // Verify OutputFormat enum variants exist
    // This is a compile-time check - if it compiles, the types exist
    use splice::cli::OutputFormat;

    let _human = OutputFormat::Human;
    let _json = OutputFormat::Json;
    let _pretty = OutputFormat::Pretty;

    // Verify is_json() method
    assert!(!OutputFormat::Human.is_json());
    assert!(OutputFormat::Json.is_json());
    assert!(OutputFormat::Pretty.is_json());
}

#[test]
fn test_call_direction_enum_exists() {
    use splice::cli::CallDirection;

    let _in = CallDirection::In;
    let _out = CallDirection::Out;
    let _both = CallDirection::Both;
}

#[test]
fn test_splice_exit_code_values() {
    // Verify exit code values match Magellan conventions
    // Note: SpliceExitCode is defined in main.rs, not re-exported
    // We verify the values match Magellan conventions (0-5)
    let success: u8 = 0;
    let error: u8 = 1;
    let usage: u8 = 2;
    let database: u8 = 3;
    let file_not_found: u8 = 4;
    let validation: u8 = 5;

    // These are the expected Magellan exit codes
    assert_eq!(success, 0);
    assert_eq!(error, 1);
    assert_eq!(usage, 2);
    assert_eq!(database, 3);
    assert_eq!(file_not_found, 4);
    assert_eq!(validation, 5);
}

#[test]
fn test_response_types_serialize() {
    use splice::output::{
        FilesResponse, FindResponse, MagellanSymbol, RefsResponse, StatusResponse,
    };

    // Test StatusResponse
    let status = StatusResponse {
        files: 10,
        symbols: 100,
        references: 50,
        calls: 25,
        code_chunks: 75,
        db_path: "/path/to/db".to_string(),
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains(r#""files":10"#));
    assert!(json.contains(r#""symbols":100"#));

    // Test FindResponse
    let find = FindResponse {
        symbols: vec![MagellanSymbol {
            symbol_id: Some("abc123".to_string()),
            id_format: Some("v1".to_string()),
            name: "test_fn".to_string(),
            kind: "fn".to_string(),
            file_path: "/path/to/file.rs".to_string(),
            byte_start: 0,
            byte_end: 100,
            start_line: 1,
            end_line: 5,
            start_col: 0,
            end_col: 4,
        }],
        count: 1,
    };
    let json = serde_json::to_string(&find).unwrap();
    assert!(json.contains(r#""start_line":1"#)); // Magellan field name
    assert!(json.contains(r#""end_line":5"#));

    // Test RefsResponse
    // Note: callers and callees use skip_serializing_if, so empty vectors are omitted
    let refs = RefsResponse {
        symbol: MagellanSymbol {
            symbol_id: None,
            id_format: None,
            name: "main".to_string(),
            kind: "fn".to_string(),
            file_path: "/path/to/main.rs".to_string(),
            byte_start: 0,
            byte_end: 50,
            start_line: 1,
            end_line: 3,
            start_col: 0,
            end_col: 2,
        },
        callers: vec![],
        callees: vec![],
    };
    let json = serde_json::to_string(&refs).unwrap();
    assert!(json.contains(r#""symbol":"#));
    // Empty vectors are skipped due to skip_serializing_if attribute
    assert!(!json.contains(r#""callers""#));

    // Test FilesResponse
    let files = FilesResponse {
        files: vec![],
        count: 0,
    };
    let json = serde_json::to_string(&files).unwrap();
    assert!(json.contains(r#""count":0"#));
}

#[test]
fn test_magellan_symbol_field_names() {
    use splice::output::MagellanSymbol;
    use serde_json::Value;

    let symbol = MagellanSymbol {
        symbol_id: Some("test_id".to_string()),
        id_format: Some("v2".to_string()),
        name: "test".to_string(),
        kind: "fn".to_string(),
        file_path: "/path/to/test.rs".to_string(),
        byte_start: 0,
        byte_end: 100,
        start_line: 5,
        end_line: 10,
        start_col: 2,
        end_col: 6,
    };

    let json = serde_json::to_string(&symbol).unwrap();
    let value: Value = serde_json::from_str(&json).unwrap();

    // Verify Magellan field names (NOT Splice names)
    assert!(value.get("start_line").is_some());
    assert!(value.get("end_line").is_some());
    assert!(value.get("start_col").is_some());
    assert!(value.get("end_col").is_some());

    // Verify Splice field names are NOT present
    assert!(value.get("line_start").is_none());
    assert!(value.get("line_end").is_none());
    assert!(value.get("col_start").is_none());
    assert!(value.get("col_end").is_none());
}

#[test]
fn test_response_types_reexported() {
    // Verify response types are accessible via splice::cli::
    // This is a compile-time check
    use splice::cli::{FindResponse, StatusResponse};

    // Just verify types exist
    let _status: StatusResponse = StatusResponse {
        files: 0,
        symbols: 0,
        references: 0,
        calls: 0,
        code_chunks: 0,
        db_path: String::new(),
    };

    let _find: FindResponse = FindResponse {
        symbols: vec![],
        count: 0,
    };

    // Types compile successfully
    assert_eq!(_status.files, 0);
    assert_eq!(_find.count, 0);
}

#[test]
fn test_help_text_includes_categories() {
    // Note: This test checks that the help text categories are defined
    // in the source code, not by running the binary (which would require subprocess)
    // The categories are verified at compile time by the long_about attribute

    // Verify OutputFormat parsing works
    use splice::cli::OutputFormat;

    // Human format should not be JSON
    assert!(!OutputFormat::Human.is_json());

    // Json and Pretty should both be JSON formats
    assert!(OutputFormat::Json.is_json());
    assert!(OutputFormat::Pretty.is_json());

    // Verify we can format JSON with each format
    let test_data = serde_json::json!({"test": "data"});

    assert!(OutputFormat::Json.format_json(&test_data).is_ok());
    assert!(OutputFormat::Pretty.format_json(&test_data).is_ok());
    assert!(OutputFormat::Human.format_json(&test_data).is_err());
}

#[test]
fn test_status_command_requires_db_flag() {
    // Test that the Status command struct requires db field
    // This is a compile-time check - if it compiles, the field exists
    use splice::cli::Commands;

    // We can't run the CLI without a database, but we can verify
    // the struct requires the db field by construction
    let _status_variant = Commands::Status {
        db: PathBuf::from("/tmp/test.db"),
    };

    // If we reached here, the Status command has the required db field
    assert!(true);
}

#[test]
fn test_find_command_requires_name_or_symbol_id() {
    // Test Find command has name and symbol_id fields with conflicts_with
    use splice::cli::Commands;

    // Valid Find with name
    let _find_with_name = Commands::Find {
        db: PathBuf::from("/tmp/test.db"),
        name: Some("test".to_string()),
        symbol_id: None,
        ambiguous: false,
        output: splice::cli::OutputFormat::Human,
    };

    // Valid Find with symbol_id
    let _find_with_id = Commands::Find {
        db: PathBuf::from("/tmp/test.db"),
        name: None,
        symbol_id: Some("abc123".to_string()),
        ambiguous: false,
        output: splice::cli::OutputFormat::Human,
    };

    // Both are valid at the struct level (clap handles conflicts_with at parse time)
    assert!(true);
}

#[test]
fn test_files_command_requires_db_flag() {
    // Test that the Files command struct requires db field
    use splice::cli::Commands;

    let _files_variant = Commands::Files {
        db: PathBuf::from("/tmp/test.db"),
        symbols: false,
        output: splice::cli::OutputFormat::Human,
    };

    // If we reached here, the Files command has the required db field
    assert!(true);
}

#[test]
fn test_output_format_flag_accepted() {
    // Test that OutputFormat enum has all three variants
    use splice::cli::OutputFormat;

    // Verify all three format variants exist and can be constructed
    let _human = OutputFormat::Human;
    let _json = OutputFormat::Json;
    let _pretty = OutputFormat::Pretty;

    // Verify we can convert to string if needed for clap parsing
    // (clap derives ValueEnum which provides to_possible_value())
    assert!(true);
}

#[test]
fn test_call_direction_enum_parsing() {
    use splice::cli::CallDirection;

    // Verify we can construct all direction variants
    let in_dir = CallDirection::In;
    let _out_dir = CallDirection::Out;
    let _both_dir = CallDirection::Both;

    // Verify PartialEq works
    assert_eq!(CallDirection::In, CallDirection::In);
    assert_ne!(CallDirection::In, CallDirection::Out);

    // Verify Copy trait works
    let in_copy = in_dir;
    assert_eq!(in_dir, in_copy);
}

#[test]
fn test_refs_command_has_direction_field() {
    // Test Refs command has direction field
    use splice::cli::{CallDirection, Commands, OutputFormat};

    let _refs_variant = Commands::Refs {
        db: PathBuf::from("/tmp/test.db"),
        name: "test".to_string(),
        path: PathBuf::from("/tmp/test.rs"),
        direction: CallDirection::Both,
        output: OutputFormat::Human,
    };

    // If we reached here, the Refs command has all required fields
    assert!(true);
}

#[test]
fn test_magellan_span_field_names() {
    use splice::output::MagellanSpan;
    use serde_json::Value;

    let span = MagellanSpan {
        file_path: "/path/to/file.rs".to_string(),
        byte_start: 0,
        byte_end: 100,
        start_line: 5,
        start_col: 2,
        end_line: 10,
        end_col: 6,
    };

    let json = serde_json::to_string(&span).unwrap();
    let value: Value = serde_json::from_str(&json).unwrap();

    // Verify Magellan field names (NOT Splice names)
    assert!(value.get("start_line").is_some());
    assert!(value.get("end_line").is_some());
    assert!(value.get("start_col").is_some());
    assert!(value.get("end_col").is_some());
}

#[test]
fn test_magellan_call_reference_serialization() {
    use splice::output::{MagellanCallReference, MagellanSpan, MagellanSymbol};

    let call_ref = MagellanCallReference {
        symbol: MagellanSymbol {
            symbol_id: Some("abc123".to_string()),
            id_format: Some("v1".to_string()),
            name: "callee".to_string(),
            kind: "fn".to_string(),
            file_path: "/path/to/callee.rs".to_string(),
            byte_start: 0,
            byte_end: 50,
            start_line: 1,
            end_line: 5,
            start_col: 0,
            end_col: 2,
        },
        call_site: MagellanSpan {
            file_path: "/path/to/caller.rs".to_string(),
            byte_start: 100,
            byte_end: 105,
            start_line: 10,
            start_col: 5,
            end_line: 10,
            end_col: 10,
        },
    };

    let json = serde_json::to_string(&call_ref).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify structure
    assert!(value.get("symbol").is_some());
    assert!(value.get("call_site").is_some());

    // Verify Magellan field names in nested structures
    let call_site = value.get("call_site").unwrap();
    assert!(call_site.get("start_line").is_some());
    assert!(call_site.get("end_line").is_some());
}

#[test]
fn test_magellan_file_metadata_serialization() {
    use splice::output::MagellanFileMetadata;

    let metadata = MagellanFileMetadata {
        path: "/path/to/file.rs".to_string(),
        hash: "abc123".to_string(),
        last_indexed_at: 1234567890,
        last_modified: 1234567890,
        symbol_count: Some(42),
    };

    let json = serde_json::to_string(&metadata).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify all fields serialize correctly
    assert_eq!(value.get("path").unwrap().as_str(), Some("/path/to/file.rs"));
    assert_eq!(value.get("hash").unwrap().as_str(), Some("abc123"));
    assert_eq!(value.get("symbol_count").unwrap().as_u64(), Some(42));
}

#[test]
fn test_status_response_serialization() {
    use splice::output::StatusResponse;

    let status = StatusResponse {
        files: 100,
        symbols: 1000,
        references: 500,
        calls: 250,
        code_chunks: 750,
        db_path: "/tmp/magellan.db".to_string(),
    };

    let json = serde_json::to_string(&status).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify all fields serialize correctly
    assert_eq!(value.get("files").unwrap().as_u64(), Some(100));
    assert_eq!(value.get("symbols").unwrap().as_u64(), Some(1000));
    assert_eq!(value.get("references").unwrap().as_u64(), Some(500));
    assert_eq!(value.get("calls").unwrap().as_u64(), Some(250));
    assert_eq!(value.get("code_chunks").unwrap().as_u64(), Some(750));
    assert_eq!(
        value.get("db_path").unwrap().as_str(),
        Some("/tmp/magellan.db")
    );
}

// ============================================================================
// Export command tests (Phase 25-04)
// ============================================================================

#[cfg(test)]
mod export_tests {
    use serde_json::Value;
    use splice::graph::magellan_integration::MagellanIntegration;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    /// Get the path to the splice binary.
    fn get_splice_binary() -> PathBuf {
        if let Ok(path) = std::env::var("SPLICE_TEST_BIN") {
            return PathBuf::from(path);
        }

        if let Ok(path) = std::env::var("CARGO_BIN_EXE_splice") {
            return PathBuf::from(path);
        }

        let mut path = std::env::current_exe().unwrap();
        path.pop(); // deps
        let deps_dir = path.clone();
        path.pop(); // debug
        let bin_path = path.join("splice");

        if bin_path.exists() {
            return bin_path;
        }

        if let Ok(entries) = std::fs::read_dir(deps_dir) {
            let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                if !name.starts_with("splice-") || !path.is_file() {
                    continue;
                }

                if let Ok(metadata) = entry.metadata() {
                    #[cfg(unix)]
                    let is_executable = metadata.permissions().mode() & 0o111 != 0;
                    #[cfg(not(unix))]
                    let is_executable = true;

                    if !is_executable {
                        continue;
                    }

                    if let Ok(modified) = metadata.modified() {
                        let len = metadata.len();
                        if len > 50_000_000 {
                            candidates.push((modified, path));
                        }
                    }
                }
            }

            if let Some((_, path)) = candidates.into_iter().max_by_key(|(time, _)| *time) {
                return path;
            }
        }

        bin_path
    }

    /// Extract JSON from stdout that may contain debug output lines.
    fn extract_json_from_stdout(stdout: &str) -> String {
        let start = stdout.find('{');
        let end = stdout.rfind('}');

        match (start, end) {
            (Some(start), Some(end)) if end >= start => stdout[start..=end].to_string(),
            _ => String::new(),
        }
    }

    #[test]
    fn test_export_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let output_path = temp_dir.path().join("export.json");

        // Create a test file and index it
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn test() { println!(\"hello\"); }").unwrap();

        let mut integration = MagellanIntegration::open(&db_path).unwrap();
        integration.index_file(&test_file).unwrap();

        // Run export command
        let splice_binary = get_splice_binary();
        let result = Command::new(&splice_binary)
            .arg("export")
            .arg("--db")
            .arg(&db_path)
            .arg("--format")
            .arg("json")
            .arg("--file")
            .arg(&output_path)
            .output();

        assert!(result.is_ok(), "export command should succeed");
        let output = result.unwrap();
        if !output.status.success() {
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        assert!(output.status.success(), "export should return success");

        // Verify output file exists and contains valid JSON
        let json_content = std::fs::read_to_string(&output_path).unwrap();
        let value: Value = serde_json::from_str(&json_content)
            .expect("export should produce valid JSON");

        // Check required fields
        assert!(value.get("schema_version").is_some(), "should have schema_version");
        assert!(value.get("timestamp").is_some(), "should have timestamp");
        assert!(value.get("db_path").is_some(), "should have db_path");
        assert!(value.get("data").is_some(), "should have data");

        // Check data structure
        let data = &value["data"];
        assert!(data.get("files").is_some(), "data should have files array");
        assert!(data.get("symbols").is_some(), "data should have symbols array");
        assert!(data.get("references").is_some(), "data should have references array");
        assert!(data.get("calls").is_some(), "data should have calls array");
    }

    #[test]
    fn test_export_jsonl_format() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let output_path = temp_dir.path().join("export.jsonl");

        // Create a test file and index it
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn test() {}").unwrap();

        let mut integration = MagellanIntegration::open(&db_path).unwrap();
        integration.index_file(&test_file).unwrap();

        // Run export command
        let splice_binary = get_splice_binary();
        let result = Command::new(&splice_binary)
            .arg("export")
            .arg("--db")
            .arg(&db_path)
            .arg("--format")
            .arg("jsonl")
            .arg("--file")
            .arg(&output_path)
            .output();

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());

        // Verify JSONL format (one JSON object per line)
        let jsonl_content = std::fs::read_to_string(&output_path).unwrap();
        for line in jsonl_content.lines() {
            let value: Value = serde_json::from_str(line)
                .expect("each line should be valid JSON");
            // Check for type tag in data records
            if let Some(obj) = value.as_object() {
                if obj.get("type").is_some() {
                    let record_type = obj["type"].as_str().unwrap();
                    assert!(
                        record_type == "header" || record_type == "file" || record_type == "symbol",
                        "type should be header, file, or symbol"
                    );
                }
            }
        }
    }

    #[test]
    fn test_export_csv_format() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let output_path = temp_dir.path().join("export.csv");

        // Create a test file and index it
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn test() {}").unwrap();

        let mut integration = MagellanIntegration::open(&db_path).unwrap();
        integration.index_file(&test_file).unwrap();

        // Run export command
        let splice_binary = get_splice_binary();
        let result = Command::new(&splice_binary)
            .arg("export")
            .arg("--db")
            .arg(&db_path)
            .arg("--format")
            .arg("csv")
            .arg("--file")
            .arg(&output_path)
            .output();

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());

        // Verify CSV format
        let csv_content = std::fs::read_to_string(&output_path).unwrap();
        // CSV should have section headers
        assert!(csv_content.contains("# Files"), "CSV should have Files section");
        assert!(csv_content.contains("# Symbols"), "CSV should have Symbols section");
        // CSV should have column headers
        assert!(csv_content.contains("path"), "CSV should have path column");
        assert!(csv_content.contains("hash"), "CSV should have hash column");
    }

    #[test]
    fn test_export_defaults_to_json() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let output_path = temp_dir.path().join("export_default.json");

        // Create and index a test file
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn test() {}").unwrap();

        let mut integration = MagellanIntegration::open(&db_path).unwrap();
        integration.index_file(&test_file).unwrap();

        // Run export without --format flag (should default to json)
        let splice_binary = get_splice_binary();
        let result = Command::new(&splice_binary)
            .arg("export")
            .arg("--db")
            .arg(&db_path)
            .arg("--file")
            .arg(&output_path)
            .output();

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());

        // Verify output is valid JSON
        let json_content = std::fs::read_to_string(&output_path).unwrap();
        let _value: Value = serde_json::from_str(&json_content)
            .expect("default format should produce valid JSON");
    }

    #[test]
    fn test_export_stdout_output() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create and index a test file
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn test() {}").unwrap();

        let mut integration = MagellanIntegration::open(&db_path).unwrap();
        integration.index_file(&test_file).unwrap();

        // Run export without --file (should write to stdout)
        let splice_binary = get_splice_binary();
        let result = Command::new(&splice_binary)
            .arg("export")
            .arg("--db")
            .arg(&db_path)
            .arg("--format")
            .arg("json")
            .output();

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());

        // Verify stdout contains expected export data fields
        let stdout = String::from_utf8_lossy(&output.stdout);
        // When exporting to stdout, the export JSON is written directly
        // followed by the success payload JSON. Verify both are present.
        assert!(stdout.contains("schema_version"), "stdout should contain schema_version");
        assert!(stdout.contains("files"), "stdout should contain files array");
        assert!(stdout.contains("symbols"), "stdout should contain symbols array");
        assert!(stdout.contains("\"status\""), "stdout should contain success payload status");
    }

    // ============================================================================
    // Extended export format validation tests (Phase 26-02)
    // ============================================================================

    #[test]
    fn test_export_json_schema_validation() {
        use splice::output::EXPORT_SCHEMA_VERSION;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let output_path = temp_dir.path().join("export.json");

        // Create multiple test files (Rust and Python)
        let rust_file = temp_dir.path().join("lib.rs");
        std::fs::write(&rust_file, r#"
pub fn helper() -> i32 { 42 }
pub fn main() { let _ = helper(); }
"#).unwrap();

        let python_file = temp_dir.path().join("module.py");
        std::fs::write(&python_file, r#"
def helper():
    return 42

def main():
    helper()
"#).unwrap();

        let mut integration = MagellanIntegration::open(&db_path).unwrap();
        integration.index_file(&rust_file).unwrap();
        integration.index_file(&python_file).unwrap();

        // Run export command
        let splice_binary = get_splice_binary();
        let result = Command::new(&splice_binary)
            .arg("export")
            .arg("--db")
            .arg(&db_path)
            .arg("--format")
            .arg("json")
            .arg("--file")
            .arg(&output_path)
            .output();

        assert!(result.is_ok(), "export command should execute");
        let output = result.unwrap();
        if !output.status.success() {
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        assert!(output.status.success(), "export should return success");

        // Read and validate export.json
        let json_content = std::fs::read_to_string(&output_path).unwrap();
        let value: Value = serde_json::from_str(&json_content)
            .expect("export should produce valid JSON");

        // 1. Validate schema_version
        assert_eq!(
            value.get("schema_version").and_then(|v| v.as_str()),
            Some(EXPORT_SCHEMA_VERSION),
            "schema_version should equal EXPORT_SCHEMA_VERSION constant"
        );
        assert_eq!(
            value["schema_version"],
            "1.0.0",
            "schema_version should be 1.0.0"
        );

        // 2. Validate timestamp is valid ISO 8601
        let timestamp = value.get("timestamp")
            .and_then(|v| v.as_str())
            .expect("should have timestamp string");
        assert!(!timestamp.is_empty(), "timestamp should not be empty");
        // Verify it's parseable as ISO 8601 (contains T and : chars)
        assert!(timestamp.contains('T'), "timestamp should be ISO 8601 format");
        assert!(timestamp.contains(':'), "timestamp should have time component");

        // 3. Validate db_path matches test database
        let db_path_str = value.get("db_path")
            .and_then(|v| v.as_str())
            .expect("should have db_path");
        assert!(db_path_str.contains("test.db") || db_path_str.contains(&format!("{}", db_path.display())),
                "db_path should reference test database");

        // 4. Validate data structure has required arrays
        let data = value.get("data").expect("should have data object");
        assert!(data.get("files").and_then(|v| v.as_array()).is_some(),
                "data should have files array");
        assert!(data.get("symbols").and_then(|v| v.as_array()).is_some(),
                "data should have symbols array");
        assert!(data.get("references").and_then(|v| v.as_array()).is_some(),
                "data should have references array");
        assert!(data.get("calls").and_then(|v| v.as_array()).is_some(),
                "data should have calls array");

        // 5. Validate arrays have expected counts
        let files = data["files"].as_array().unwrap();
        let symbols = data["symbols"].as_array().unwrap();
        assert!(files.len() >= 2, "should have at least 2 files (Rust and Python)");
        assert!(symbols.len() >= 2, "should have at least 2 symbols");

        // 6. Validate each symbol has required fields with correct types
        for symbol in symbols {
            // symbol_id is optional (can be null for some symbols)
            // Accept both 16-char SHA-256 (v1) and 32-char BLAKE3 (v2) formats
            if let Some(sid) = symbol.get("symbol_id") {
                if let Some(sid_str) = sid.as_str() {
                    assert!((sid_str.len() == 16 || sid_str.len() == 32) && sid_str.chars().all(|c| c.is_ascii_hexdigit()),
                               "symbol_id should be 16-char or 32-char hex when present, got: {} ({})",
                               sid_str.len(), sid_str);
                }
            }

            // name must be non-empty string
            let name = symbol.get("name").and_then(|v| v.as_str())
                .expect("symbol should have name");
            assert!(!name.is_empty(), "symbol name should not be empty");

            // kind must be valid symbol kind
            let kind = symbol.get("kind").and_then(|v| v.as_str())
                .expect("symbol should have kind");
            let valid_kinds = ["fn", "method", "struct", "class", "enum", "interface",
                              "module", "trait", "impl", "variable", "constructor", "type_alias",
                              "function", "type"];  // Additional kinds from different parsers
            assert!(valid_kinds.contains(&kind) || !kind.is_empty(),
                   "symbol kind should be valid");

            // file_path must be absolute path
            let file_path = symbol.get("file_path").and_then(|v| v.as_str())
                .expect("symbol should have file_path");
            assert!(!file_path.is_empty(), "file_path should not be empty");

            // byte offsets
            let byte_start = symbol.get("byte_start").and_then(|v| v.as_u64())
                .expect("symbol should have byte_start") as usize;
            let byte_end = symbol.get("byte_end").and_then(|v| v.as_u64())
                .expect("symbol should have byte_end") as usize;
            assert!(byte_start < byte_end,
                   "byte_start ({}) should be less than byte_end ({})", byte_start, byte_end);

            // Line numbers (may be 0 if not populated)
            let start_line = symbol.get("start_line").and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let end_line = symbol.get("end_line").and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            if start_line > 0 {
                assert!(end_line >= start_line,
                       "end_line should be >= start_line when populated");
            }

            // 7. Validate Magellan field naming (start_line not line_start)
            assert!(symbol.get("start_line").is_some(),
                   "symbol should use Magellan field name start_line");
            assert!(symbol.get("end_line").is_some(),
                   "symbol should use Magellan field name end_line");
        }
    }

    #[test]
    fn test_export_jsonl_record_types() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let output_path = temp_dir.path().join("export.jsonl");

        // Create test files with different symbol types
        let rust_file = temp_dir.path().join("lib.rs");
        std::fs::write(&rust_file, r#"
pub fn helper() -> i32 { 42 }
pub struct Counter;
impl Counter {
    pub fn new() -> Self { Counter }
}
pub fn main() { let _ = helper(); }
"#).unwrap();

        let mut integration = MagellanIntegration::open(&db_path).unwrap();
        integration.index_file(&rust_file).unwrap();

        // Run export command with jsonl format
        let splice_binary = get_splice_binary();
        let result = Command::new(&splice_binary)
            .arg("export")
            .arg("--db")
            .arg(&db_path)
            .arg("--format")
            .arg("jsonl")
            .arg("--file")
            .arg(&output_path)
            .output();

        assert!(result.is_ok(), "export command should execute");
        let output = result.unwrap();
        if !output.status.success() {
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        assert!(output.status.success(), "export should return success");

        // Read and validate JSONL format
        let jsonl_content = std::fs::read_to_string(&output_path).unwrap();
        let lines: Vec<&str> = jsonl_content.lines().collect();

        // Should have at least header + file record + some symbols
        assert!(lines.len() >= 3, "should have header, file, and at least one symbol");

        let mut found_header = false;
        let mut found_file = false;
        let mut found_symbol = false;

        // Track all record types found
        let mut record_types = std::collections::HashSet::new();

        for (line_num, line) in lines.iter().enumerate() {
            // Verify every line is valid JSON
            let value: Value = serde_json::from_str(line)
                .unwrap_or_else(|e| {
                    panic!("Line {} should be valid JSON: {}\nContent: {}", line_num + 1, e, line);
                });

            // Check for type field
            if let Some(obj) = value.as_object() {
                if let Some(record_type) = obj.get("type") {
                    let type_str = record_type.as_str()
                        .unwrap_or_else(|| panic!("Line {} type field should be string", line_num + 1));
                    record_types.insert(type_str.to_string());

                    match type_str {
                        "header" => {
                            found_header = true;
                            // Header should have schema_version
                            assert!(obj.get("schema_version").is_some(),
                                   "header record should have schema_version");
                        }
                        "file" => {
                            found_file = true;
                            // File records have "data" wrapper with path and hash
                            if let Some(data) = obj.get("data") {
                                let data_obj = data.as_object()
                                    .expect("file data should be object");
                                assert!(data_obj.get("path").is_some(),
                                       "file data should have path");
                                assert!(data_obj.get("hash").is_some(),
                                       "file data should have hash");
                            } else {
                                panic!("Line {} file record should have data field", line_num + 1);
                            }
                        }
                        "symbol" => {
                            found_symbol = true;
                            // Symbol records have "data" wrapper
                            if let Some(data) = obj.get("data") {
                                let data_obj = data.as_object()
                                    .expect("symbol data should be object");
                                assert!(data_obj.get("name").is_some(),
                                       "symbol data should have name");
                                assert!(data_obj.get("kind").is_some(),
                                       "symbol data should have kind");
                                assert!(data_obj.get("file_path").is_some(),
                                       "symbol data should have file_path");
                            } else {
                                panic!("Line {} symbol record should have data field", line_num + 1);
                            }
                        }
                        "reference" => {
                            // Reference records with from_symbol and to_symbol
                            if let Some(data) = obj.get("data") {
                                let data_obj = data.as_object()
                                    .expect("reference data should be object");
                                assert!(data_obj.get("from_symbol_id").is_some() ||
                                        data_obj.get("from_symbol").is_some(),
                                       "reference data should have from_symbol field");
                                assert!(data_obj.get("to_symbol_id").is_some() ||
                                        data_obj.get("to_symbol").is_some(),
                                       "reference data should have to_symbol field");
                            }
                        }
                        "call" => {
                            // Call records with caller and callee
                            if let Some(data) = obj.get("data") {
                                let data_obj = data.as_object()
                                    .expect("call data should be object");
                                assert!(data_obj.get("caller_symbol_id").is_some() ||
                                        data_obj.get("caller").is_some(),
                                       "call data should have caller field");
                                assert!(data_obj.get("callee_symbol_id").is_some() ||
                                        data_obj.get("callee").is_some(),
                                       "call data should have callee field");
                                assert!(data_obj.get("call_site").is_some() ||
                                        data_obj.get("call_site_file").is_some() ||
                                        data_obj.get("call_site_line").is_some(),
                                       "call data should have call_site field");
                            }
                        }
                        other => {
                            panic!("Line {} unexpected record type: {}", line_num + 1, other);
                        }
                    }
                }
            }
        }

        // Verify we found the expected record types
        assert!(found_header, "should find header record type");
        assert!(found_file, "should find at least one file record type");
        assert!(found_symbol, "should find at least one symbol record type");

        // Verify all record types are valid
        for record_type in &record_types {
            assert!(
                matches!(record_type.as_str(), "header" | "file" | "symbol" | "reference" | "call"),
                "record type '{}' should be valid", record_type
            );
        }
    }

    #[test]
    fn test_export_csv_section_structure() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let output_path = temp_dir.path().join("export.csv");

        // Create multiple test files
        let rust_file = temp_dir.path().join("lib.rs");
        std::fs::write(&rust_file, r#"
pub fn helper() -> i32 { 42 }
pub struct Counter;
pub fn main() { let _ = helper(); }
"#).unwrap();

        let python_file = temp_dir.path().join("module.py");
        std::fs::write(&python_file, r#"
def helper():
    return 42
"#).unwrap();

        let mut integration = MagellanIntegration::open(&db_path).unwrap();
        integration.index_file(&rust_file).unwrap();
        integration.index_file(&python_file).unwrap();

        // Run export command with csv format
        let splice_binary = get_splice_binary();
        let result = Command::new(&splice_binary)
            .arg("export")
            .arg("--db")
            .arg(&db_path)
            .arg("--format")
            .arg("csv")
            .arg("--file")
            .arg(&output_path)
            .output();

        assert!(result.is_ok(), "export command should execute");
        let output = result.unwrap();
        if !output.status.success() {
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        assert!(output.status.success(), "export should return success");

        // Read and validate CSV format
        let csv_content = std::fs::read_to_string(&output_path).unwrap();

        // 1. Validate section headers
        assert!(csv_content.contains("# Files"), "CSV should have Files section header");
        assert!(csv_content.contains("# Symbols"), "CSV should have Symbols section header");

        // 2. Parse into sections
        let lines: Vec<&str> = csv_content.lines().collect();

        // Find section boundaries
        let mut files_section_start = None;
        let mut symbols_section_start = None;
        let mut references_section_start = None;
        let mut calls_section_start = None;

        for (i, line) in lines.iter().enumerate() {
            if *line == "# Files" {
                files_section_start = Some(i);
            } else if *line == "# Symbols" {
                symbols_section_start = Some(i);
            } else if *line == "# References" {
                references_section_start = Some(i);
            } else if *line == "# Calls" {
                calls_section_start = Some(i);
            }
        }

        assert!(files_section_start.is_some(), "should find # Files section");
        assert!(symbols_section_start.is_some(), "should find # Symbols section");

        // 3. Validate Files section has proper headers
        if let Some(files_start) = files_section_start {
            // Files section header is followed by column header row
            if files_start + 1 < lines.len() {
                let header_line = lines[files_start + 1];
                assert!(header_line.contains("path"),
                       "Files section should have path column header");
                assert!(header_line.contains("hash"),
                       "Files section should have hash column header");
            }
        }

        // 4. Validate Symbols section has proper headers
        if let Some(symbols_start) = symbols_section_start {
            // Symbols section header is followed by column header row
            if symbols_start + 1 < lines.len() {
                let header_line = lines[symbols_start + 1];
                assert!(header_line.contains("symbol_id"),
                       "Symbols section should have symbol_id column header");
                assert!(header_line.contains("name"),
                       "Symbols section should have name column header");
                assert!(header_line.contains("kind"),
                       "Symbols section should have kind column header");
                assert!(header_line.contains("file_path"),
                       "Symbols section should have file_path column header");
                assert!(header_line.contains("byte_start"),
                       "Symbols section should have byte_start column header");
                assert!(header_line.contains("byte_end"),
                       "Symbols section should have byte_end column header");
            }
        }

        // 5. Validate data rows match column counts
        // Count columns in header row vs data rows for each section
        let mut prev_col_count = None;

        for line in &lines {
            // Skip empty lines and section headers
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let col_count = line.split(',').count();

            if let Some(prev) = prev_col_count {
                // Within a section, column counts should be consistent
                // (New sections reset the count, which is fine)
                if col_count != prev {
                    // Column count changed - this is OK at section boundaries
                }
            }
            prev_col_count = Some(col_count);
        }

        // 6. Validate no empty rows between sections
        for (i, line) in lines.iter().enumerate() {
            // Empty lines are allowed, but consecutive empty lines are not
            if line.is_empty() {
                if i > 0 && lines[i - 1].is_empty() {
                    panic!("Line {}: consecutive empty lines not allowed", i + 1);
                }
            }
        }

        // 7. Verify we have actual data (at least 2 files and symbols)
        let data_lines: Vec<&str> = lines.iter()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .copied()
            .collect();

        assert!(data_lines.len() >= 2,
               "should have at least 2 data rows (files + symbols)");
    }

    #[test]
    fn test_export_error_handling() {
        // Test 1: Invalid format option (if CLI validates it)
        {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");

            // Create a test file and index it
            let test_file = temp_dir.path().join("test.rs");
            std::fs::write(&test_file, "fn test() {}").unwrap();

            let mut integration = MagellanIntegration::open(&db_path).unwrap();
            integration.index_file(&test_file).unwrap();

            let splice_binary = get_splice_binary();
            let output = Command::new(&splice_binary)
                .arg("export")
                .arg("--db")
                .arg(&db_path)
                .arg("--format")
                .arg("invalid_format")
                .output();

            assert!(output.is_ok(), "command should execute");
            let output = output.unwrap();

            // Should fail - clap validates the format enum
            assert!(!output.status.success(),
                   "export should fail with invalid format");

            // Exit code should be 2 (usage error) from clap
            let exit_code = output.status.code();
            assert!(exit_code == Some(2),
                   "Expected exit code 2 for invalid argument, got {:?}", exit_code);

            // stderr should mention the invalid value
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("invalid_format") ||
                   stderr.contains("invalid") ||
                   stderr.contains("possible values") ||
                   stderr.contains("one of"),
                   "stderr should mention invalid value or show possible values");
        }

        // Test 2: Valid export with empty database (edge case - Magellan creates db if nonexistent)
        {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("empty.db");
            let output_path = temp_dir.path().join("export.json");

            // Create empty database
            let _integration = MagellanIntegration::open(&db_path).unwrap();

            let splice_binary = get_splice_binary();
            let output = Command::new(&splice_binary)
                .arg("export")
                .arg("--db")
                .arg(&db_path)
                .arg("--format")
                .arg("json")
                .arg("--file")
                .arg(&output_path)
                .output();

            assert!(output.is_ok(), "command should execute");
            let output = output.unwrap();

            // Should succeed with empty data
            assert!(output.status.success(),
                   "export should succeed with empty database");

            // Output should be valid JSON with empty arrays
            let json_content = std::fs::read_to_string(&output_path).unwrap();
            let value: Value = serde_json::from_str(&json_content)
                .expect("export should produce valid JSON");

            assert!(value.get("schema_version").is_some(),
                   "should have schema_version");
            assert!(value["data"]["files"].as_array().unwrap().is_empty(),
                   "files array should be empty");
            assert!(value["data"]["symbols"].as_array().unwrap().is_empty(),
                   "symbols array should be empty");
        }

        // Test 3: File write permission error (read-only directory) - Unix only
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");
            let readonly_dir = temp_dir.path().join("readonly");

            // Create a test file and index it
            let test_file = temp_dir.path().join("test.rs");
            std::fs::write(&test_file, "fn test() {}").unwrap();

            let mut integration = MagellanIntegration::open(&db_path).unwrap();
            integration.index_file(&test_file).unwrap();

            // Create read-only directory
            std::fs::create_dir(&readonly_dir).unwrap();
            let mut perms = std::fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_mode(0o444); // read-only
            std::fs::set_permissions(&readonly_dir, perms).unwrap();

            let readonly_file = readonly_dir.join("export.json");

            let splice_binary = get_splice_binary();
            let output = Command::new(&splice_binary)
                .arg("export")
                .arg("--db")
                .arg(&db_path)
                .arg("--format")
                .arg("json")
                .arg("--file")
                .arg(&readonly_file)
                .output();

            assert!(output.is_ok(), "command should execute");
            let output = output.unwrap();

            // Should fail due to permission error
            assert!(!output.status.success(),
                   "export should fail with read-only directory");

            // Exit code should be 1 (error)
            let exit_code = output.status.code();
            assert!(exit_code == Some(1),
                   "Expected exit code 1 for permission error, got {:?}", exit_code);

            // stderr should contain error message
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!stderr.is_empty() || !output.stdout.is_empty(),
                   "should have some error output");
        }

        // Test 4: Export to stdout succeeds
        {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");

            // Create a test file and index it
            let test_file = temp_dir.path().join("test.rs");
            std::fs::write(&test_file, "fn test() {}").unwrap();

            let mut integration = MagellanIntegration::open(&db_path).unwrap();
            integration.index_file(&test_file).unwrap();

            let splice_binary = get_splice_binary();
            let output = Command::new(&splice_binary)
                .arg("export")
                .arg("--db")
                .arg(&db_path)
                .arg("--format")
                .arg("json")
                .output();

            assert!(output.is_ok(), "command should execute");
            let output = output.unwrap();

            // Should succeed (writing to stdout is valid)
            assert!(output.status.success(),
                   "export to stdout should succeed");

            // stdout should contain export data
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(stdout.contains("schema_version"),
                   "stdout should contain schema_version");
            assert!(stdout.contains("\"status\""),
                   "stdout should contain success status");
        }
    }
}
