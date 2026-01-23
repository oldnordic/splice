//! LLM consumption tests verifying JSON fields are properly structured for agent use.
//!
//! These tests validate that JSON output meets LLM requirements for reliable parsing:
//! - Top-level JSON has required fields (status, error/data)
//! - Optional fields use skip_serializing_if (not present when None)
//! - Rich span objects have proper nested structure
//! - Arrays are never null (use empty arrays instead)
//! - Error codes have required sub-fields (code, severity, location, hint)
//! - Type consistency (no mixed types for same field)
//! - JSON is parseable and round-trips correctly
//!
//! Note: These tests validate JSON schema and structure only.
//! We do not test with actual LLMs - that's out of scope for a refactoring tool.

use serde_json::{json, Value};
use splice::action::{ActionType, Confidence, SuggestedAction};
use splice::cli::{CliErrorPayload, CliSuccessPayload};
use splice::context::extract_context;
use splice::error_codes::ErrorCode;
use splice::hints::ToolHints;
use splice::output::SpanResult;
use splice::relationships::{Relationship, Relationships};
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

// Test 1: Top-level JSON has required fields
#[test]
fn test_llm_json_top_level_fields() {
    let payload = CliSuccessPayload {
        status: "ok",
        message: "Operation completed".to_string(),
        data: None,
        already_emitted: false,
        has_pending_changes: false,
    };

    let json_value = serde_json::to_value(&payload).unwrap();

    // Verify top-level fields
    assert_eq!(json_value["status"], "ok");
    assert_eq!(json_value["message"], "Operation completed");
    assert!(
        json_value.get("data").is_none(),
        "data should be omitted when None"
    );
}

// Test 2: Optional fields use skip_serializing_if (not present when None)
#[test]
fn test_llm_json_optional_fields_omitted_when_none() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "fn test() {{}}").unwrap();

    let file_path = file.path();
    let span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 0, 10);

    let json_value = serde_json::to_value(&span).unwrap();

    // Verify optional fields are NOT present (skip_serializing_if)
    assert!(
        json_value.get("context").is_none(),
        "context should be omitted when None"
    );
    assert!(
        json_value.get("semantics").is_none(),
        "semantics should be omitted"
    );
    assert!(
        json_value.get("checksums").is_none(),
        "checksums should be omitted"
    );
    assert!(
        json_value.get("error_code").is_none(),
        "error_code should be omitted"
    );
    assert!(
        json_value.get("relationships").is_none(),
        "relationships should be omitted"
    );
    assert!(
        json_value.get("tool_hints").is_none(),
        "tool_hints should be omitted"
    );
    assert!(
        json_value.get("suggested_action").is_none(),
        "suggested_action should be omitted"
    );
}

// Test 3: Rich span context object has proper nested structure
#[test]
fn test_llm_json_context_object_structure() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();
    writeln!(file, "selected").unwrap();
    writeln!(file, "line 4").unwrap();

    let file_path = file.path();
    let byte_start = 14; // "selected"
    let byte_end = 22;

    let ctx = extract_context(file_path, byte_start, byte_end, 1).unwrap();

    // Create span with context
    let mut span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        byte_start,
        byte_end,
    );
    span = span.with_context(ctx);

    let json_value = serde_json::to_value(&span).unwrap();

    // Verify context is object (not null)
    assert!(
        json_value.get("context").is_some(),
        "context should be present"
    );
    let context = json_value.get("context").unwrap();
    assert!(context.is_object(), "context should be object, not null");

    // Verify context has before, selected, after arrays
    let ctx_obj = context.as_object().unwrap();
    assert!(
        ctx_obj.contains_key("before"),
        "context should have 'before' field"
    );
    assert!(
        ctx_obj.contains_key("selected"),
        "context should have 'selected' field"
    );
    assert!(
        ctx_obj.contains_key("after"),
        "context should have 'after' field"
    );

    // Verify arrays are arrays (not null)
    let before = ctx_obj.get("before").unwrap();
    let selected = ctx_obj.get("selected").unwrap();
    let after = ctx_obj.get("after").unwrap();

    assert!(
        before.is_array(),
        "context.before should be array, not null"
    );
    assert!(
        selected.is_array(),
        "context.selected should be array, not null"
    );
    assert!(after.is_array(), "context.after should be array, not null");
}

// Test 4: Error code object has required sub-fields
#[test]
fn test_llm_json_error_code_structure() {
    let error_code = ErrorCode::new(
        "SPL-E001",
        "error",
        "/path/to/file.rs:10:5".to_string(),
        "Symbol not found".to_string(),
    );

    let mut span = SpanResult::from_byte_span("/path/to/file.rs".to_string(), 0, 10);
    span = span.with_error_code(error_code);

    let json_value = serde_json::to_value(&span).unwrap();

    // Verify error_code is object (not null)
    assert!(
        json_value.get("error_code").is_some(),
        "error_code should be present"
    );
    let error_code_json = json_value.get("error_code").unwrap();
    assert!(
        error_code_json.is_object(),
        "error_code should be object, not null"
    );

    // Verify error_code has required fields
    let ec_obj = error_code_json.as_object().unwrap();
    assert!(
        ec_obj.contains_key("code"),
        "error_code should have 'code' field"
    );
    assert!(
        ec_obj.contains_key("severity"),
        "error_code should have 'severity' field"
    );
    assert!(
        ec_obj.contains_key("location"),
        "error_code should have 'location' field"
    );
    assert!(
        ec_obj.contains_key("hint"),
        "error_code should have 'hint' field"
    );

    // Verify field types
    assert_eq!(ec_obj.get("code").unwrap().as_str(), Some("SPL-E001"));
    assert_eq!(ec_obj.get("severity").unwrap().as_str(), Some("error"));
    assert_eq!(
        ec_obj.get("location").unwrap().as_str(),
        Some("/path/to/file.rs:10:5")
    );
    assert_eq!(
        ec_obj.get("hint").unwrap().as_str(),
        Some("Symbol not found")
    );
}

// Test 5: Relationships object has proper structure
#[test]
fn test_llm_json_relationships_structure() {
    let relationships = Relationships {
        callers: vec![Relationship {
            rel_type: "caller".to_string(),
            name: "caller_func".to_string(),
            kind: "function".to_string(),
            file_path: "/path/to/caller.rs".to_string(),
            line_start: 5,
            byte_start: 50,
            byte_end: 60,
        }],
        callees: vec![],
        imports: vec![Relationship {
            rel_type: "import".to_string(),
            name: "std::collections".to_string(),
            kind: "module".to_string(),
            file_path: "/path/to/file.rs".to_string(),
            line_start: 1,
            byte_start: 0,
            byte_end: 20,
        }],
        exports: vec![],
        cycle_detected: false,
        error_code: None,
    };

    let span = SpanResult::from_byte_span("/path/to/file.rs".to_string(), 0, 10)
        .with_relationships(relationships);

    let json_value = serde_json::to_value(&span).unwrap();

    // Verify relationships is object
    assert!(
        json_value.get("relationships").is_some(),
        "relationships should be present"
    );
    let rel_json = json_value.get("relationships").unwrap();
    assert!(rel_json.is_object(), "relationships should be object");

    // Verify structure - note: empty arrays are omitted due to skip_serializing_if
    let rel_obj = rel_json.as_object().unwrap();
    assert!(
        rel_obj.contains_key("callers"),
        "should have 'callers' field"
    );
    assert!(
        rel_obj.contains_key("imports"),
        "should have 'imports' field"
    );

    // Empty arrays are omitted (not serialized) due to skip_serializing_if = "Vec::is_empty"
    // This is actually good for LLM consumption - reduces payload size
    assert!(
        !rel_obj.contains_key("callees"),
        "callees should be omitted when empty"
    );
    assert!(
        !rel_obj.contains_key("exports"),
        "exports should be omitted when empty"
    );

    // Verify present relationships have proper structure
    let callers = rel_obj.get("callers").unwrap();
    assert!(callers.is_array(), "callers should be array");
    assert!(
        !callers.as_array().unwrap().is_empty(),
        "callers should have one entry"
    );

    let callers_array = callers.as_array().unwrap();
    let first_caller = &callers_array[0];
    assert!(first_caller.is_object(), "caller entry should be object");
    assert!(
        first_caller.get("rel_type").is_some(),
        "caller should have rel_type"
    );
    assert!(
        first_caller.get("name").is_some(),
        "caller should have name"
    );
    assert!(
        first_caller.get("kind").is_some(),
        "caller should have kind"
    );
    assert!(
        first_caller.get("file_path").is_some(),
        "caller should have file_path"
    );
}

// Test 6: Arrays are never null (empty arrays instead)
#[test]
fn test_llm_json_arrays_not_null() {
    let mut file = NamedTempFile::new().unwrap();
    // Create a multi-line file to get context lines
    writeln!(file, "// before line 1").unwrap();
    writeln!(file, "// before line 2").unwrap();
    writeln!(file, "fn test() {{").unwrap();
    writeln!(file, "}}").unwrap();
    writeln!(file, "// after line 1").unwrap();

    let file_path = file.path();
    // The "fn test() {" is at bytes 34-45
    let byte_start = 34;
    let byte_end = 45;

    // Create span with context
    let ctx = extract_context(file_path, byte_start, byte_end, 1).unwrap();
    let span = SpanResult::from_byte_span(
        file_path.to_string_lossy().to_string(),
        byte_start,
        byte_end,
    )
    .with_context(ctx);

    let json_value = serde_json::to_value(&span).unwrap();

    // Verify context arrays are arrays (not null)
    let context = json_value.get("context").unwrap();
    let ctx_obj = context.as_object().unwrap();

    let before = ctx_obj.get("before").unwrap();
    let selected = ctx_obj.get("selected").unwrap();
    let after = ctx_obj.get("after").unwrap();

    assert!(before.is_array(), "before should be array, not null");
    assert!(selected.is_array(), "selected should be array, not null");
    assert!(after.is_array(), "after should be array, not null");

    // The key LLM requirement: arrays are never null (they may be empty)
    // When fields are within an object that's present, they should be arrays
    let _before_arr = before.as_array().unwrap();
    let _selected_arr = selected.as_array().unwrap();
    let _after_arr = after.as_array().unwrap();

    // Verify at minimum they are arrays (even if empty would be acceptable)
    // The important thing is they're not null
    assert!(!before.is_null(), "before array should never be null");
    assert!(!selected.is_null(), "selected array should never be null");
    assert!(!after.is_null(), "after array should never be null");
}

// Test 7: JSON type consistency (no mixed types)
#[test]
fn test_llm_json_type_consistency() {
    // Create multiple spans with different field populations
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "fn test() {{}}").unwrap();

    let file_path = file.path();

    let spans = vec![
        SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 0, 10),
        SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 0, 10)
            .with_semantic_info("function", "rust"),
        SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 0, 10)
            .with_semantic_info("variable", "rust"),
    ];

    // All spans should serialize to same structure
    for span in spans {
        let json_value = serde_json::to_value(&span).unwrap();

        // file_path should always be string
        assert!(
            json_value
                .get("file_path")
                .and_then(|v| v.as_str())
                .is_some(),
            "file_path should always be string"
        );

        // byte_start should always be number
        assert!(
            json_value
                .get("byte_start")
                .and_then(|v| v.as_i64())
                .is_some(),
            "byte_start should always be number"
        );

        // semantics should be object when present
        if let Some(semantics) = json_value.get("semantics") {
            assert!(semantics.is_object(), "semantics should be object");
            let sem_obj = semantics.as_object().unwrap();
            if let Some(kind) = sem_obj.get("kind") {
                assert!(kind.is_string(), "semantics.kind should be string");
            }
            if let Some(language) = sem_obj.get("language") {
                assert!(language.is_string(), "semantics.language should be string");
            }
        }

        // context should be object or null
        if let Some(ctx) = json_value.get("context") {
            assert!(
                ctx.is_object() || ctx.is_null(),
                "context should be object or null"
            );
        }
    }
}

// Test 8: CLI error payload JSON structure
#[test]
fn test_llm_cli_error_payload_structure() {
    use splice::SpliceError;

    let error = SpliceError::SymbolNotFound {
        message: "Symbol 'missing_function' not found".to_string(),
        symbol: "missing_function".to_string(),
        file: Some(PathBuf::from("/path/to/file.rs")),
        hint: "Check that the symbol name is spelled correctly".to_string(),
    };

    let payload = CliErrorPayload::from_error(&error);

    let json_value = serde_json::to_value(&payload).unwrap();

    // Verify top-level structure
    assert_eq!(json_value["status"], "error");
    assert!(
        json_value.get("error").is_some(),
        "should have 'error' object"
    );

    let error_obj = json_value.get("error").unwrap().as_object().unwrap();
    assert!(
        error_obj.contains_key("kind"),
        "error should have 'kind' field"
    );
    assert!(
        error_obj.contains_key("message"),
        "error should have 'message' field"
    );

    // Verify error code if present
    if let Some(error_code) = error_obj.get("error_code") {
        assert!(error_code.is_object(), "error_code should be object");
        let ec = error_code.as_object().unwrap();
        assert!(ec.contains_key("code"), "error_code should have 'code'");
        assert!(
            ec.contains_key("severity"),
            "error_code should have 'severity'"
        );
    }
}

// Test 9: CLI success payload with results array
#[test]
fn test_llm_cli_success_payload_with_data() {
    let data = json!({
        "results": [
            {
                "file_path": "/path/to/file1.rs",
                "byte_start": 0,
                "byte_end": 10,
                "semantics": {
                    "kind": "function",
                    "language": "rust"
                }
            },
            {
                "file_path": "/path/to/file2.rs",
                "byte_start": 20,
                "byte_end": 30,
                "semantics": {
                    "kind": "variable",
                    "language": "rust"
                }
            }
        ]
    });

    let payload = CliSuccessPayload {
        status: "ok",
        message: "Found 2 symbols".to_string(),
        data: Some(data),
        already_emitted: false,
        has_pending_changes: false,
    };

    let json_value = serde_json::to_value(&payload).unwrap();

    // Verify structure
    assert_eq!(json_value["status"], "ok");
    assert_eq!(json_value["message"], "Found 2 symbols");

    let data_obj = json_value.get("data").unwrap().as_object().unwrap();
    let results_arr = data_obj.get("results").unwrap().as_array().unwrap();
    assert_eq!(results_arr.len(), 2);

    // Verify each result has consistent structure
    for result in results_arr {
        assert!(result.is_object(), "result should be object");
        assert!(
            result.get("file_path").is_some(),
            "result should have file_path"
        );
        assert!(
            result.get("byte_start").is_some(),
            "result should have byte_start"
        );
        assert!(
            result.get("byte_end").is_some(),
            "result should have byte_end"
        );
    }
}

// Test 10: JSON is parseable (no syntax errors)
#[test]
fn test_llm_json_parseable() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "fn test() {{}}").unwrap();

    let file_path = file.path();
    let ctx = extract_context(file_path, 0, 10, 1).unwrap();

    let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 0, 10);
    span = span.with_context(ctx);
    span = span.with_semantic_info("function", "rust");

    // Serialize to JSON string
    let json_string = serde_json::to_string(&span).unwrap();

    // Verify it's parseable
    let parsed: Value = serde_json::from_str(&json_string).expect("JSON should be parseable");

    // Verify parsed structure matches original
    assert_eq!(parsed["file_path"], span.file_path);
    assert_eq!(parsed["byte_start"], span.byte_start);
    assert_eq!(parsed["byte_end"], span.byte_end);
}

// Test 11: Tool hints object structure
#[test]
fn test_llm_json_tool_hints_structure() {
    let hints = ToolHints {
        requires_full_context: true,
        apply_atomically: true,
        may_break_tests: false,
        requires_compilation: true,
    };

    let mut span = SpanResult::from_byte_span("/path/to/file.rs".to_string(), 0, 10);
    span = span.with_tool_hints(hints);

    let json_value = serde_json::to_value(&span).unwrap();

    // Verify tool_hints is object
    assert!(
        json_value.get("tool_hints").is_some(),
        "tool_hints should be present"
    );
    let hints_json = json_value.get("tool_hints").unwrap();
    assert!(hints_json.is_object(), "tool_hints should be object");

    // Verify required fields
    let hints_obj = hints_json.as_object().unwrap();
    assert!(
        hints_obj.contains_key("requires_full_context"),
        "should have requires_full_context"
    );
    assert!(
        hints_obj.contains_key("apply_atomically"),
        "should have apply_atomically"
    );
    assert!(
        hints_obj.contains_key("may_break_tests"),
        "should have may_break_tests"
    );
    assert!(
        hints_obj.contains_key("requires_compilation"),
        "should have requires_compilation"
    );

    // Verify types
    assert_eq!(
        hints_obj.get("requires_full_context").unwrap().as_bool(),
        Some(true)
    );
    assert_eq!(
        hints_obj.get("apply_atomically").unwrap().as_bool(),
        Some(true)
    );
    assert_eq!(
        hints_obj.get("may_break_tests").unwrap().as_bool(),
        Some(false)
    );
    assert_eq!(
        hints_obj.get("requires_compilation").unwrap().as_bool(),
        Some(true)
    );
}

// Test 12: Suggested action object structure
#[test]
fn test_llm_json_suggested_action_structure() {
    let action = SuggestedAction {
        action_type: ActionType::Replace,
        confidence: Confidence::High,
        reason: "Symbol uniquely resolved with complete metadata".to_string(),
        params: None,
    };

    let mut span = SpanResult::from_byte_span("/path/to/file.rs".to_string(), 0, 10);
    span = span.with_suggested_action(action);

    let json_value = serde_json::to_value(&span).unwrap();

    // Verify suggested_action is object
    assert!(
        json_value.get("suggested_action").is_some(),
        "suggested_action should be present"
    );
    let action_json = json_value.get("suggested_action").unwrap();
    assert!(action_json.is_object(), "suggested_action should be object");

    // Verify required fields
    let action_obj = action_json.as_object().unwrap();
    assert!(
        action_obj.contains_key("action_type"),
        "should have action_type"
    );
    assert!(
        action_obj.contains_key("confidence"),
        "should have confidence"
    );
    assert!(action_obj.contains_key("reason"), "should have reason");

    // Verify values
    assert_eq!(
        action_obj.get("action_type").unwrap().as_str(),
        Some("replace")
    );
    assert_eq!(action_obj.get("confidence").unwrap().as_str(), Some("high"));
    assert!(
        action_obj.get("reason").unwrap().as_str().is_some(),
        "reason should be string"
    );
}

// Test 13: Complete rich span with all optional fields
#[test]
fn test_llm_json_complete_rich_span() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "fn greet() {{}}").unwrap();

    let file_path = file.path();
    let ctx = extract_context(file_path, 0, 13, 0).unwrap();
    let error_code = ErrorCode::new("SPL-E001", "error", "test.rs:1:1", "Test hint");

    let relationships = Relationships {
        callers: vec![],
        callees: vec![],
        imports: vec![],
        exports: vec![],
        cycle_detected: false,
        error_code: None,
    };

    let hints = ToolHints {
        requires_full_context: false,
        apply_atomically: true,
        may_break_tests: false,
        requires_compilation: true,
    };

    let action = SuggestedAction {
        action_type: ActionType::Replace,
        confidence: Confidence::High,
        reason: "Test action".to_string(),
        params: None,
    };

    let span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 0, 13)
        .with_context(ctx)
        .with_semantic_info("function", "rust")
        .with_checksum_before("abc123")
        .with_file_checksum_before("def456")
        .with_error_code(error_code)
        .with_relationships(relationships)
        .with_tool_hints(hints)
        .with_suggested_action(action);

    let json_value = serde_json::to_value(&span).unwrap();

    // Verify all rich fields are present
    assert!(
        json_value.get("context").is_some(),
        "context should be present"
    );
    assert!(
        json_value.get("semantics").is_some(),
        "semantics should be present"
    );
    assert!(
        json_value.get("checksums").is_some(),
        "checksums should be present"
    );
    assert!(
        json_value.get("error_code").is_some(),
        "error_code should be present"
    );
    assert!(
        json_value.get("relationships").is_some(),
        "relationships should be present"
    );
    assert!(
        json_value.get("tool_hints").is_some(),
        "tool_hints should be present"
    );
    assert!(
        json_value.get("suggested_action").is_some(),
        "suggested_action should be present"
    );

    // Verify nested object structures
    assert!(json_value["context"].is_object());
    assert!(json_value["error_code"].is_object());
    assert!(json_value["relationships"].is_object());
    assert!(json_value["tool_hints"].is_object());
    assert!(json_value["suggested_action"].is_object());
    assert!(json_value["semantics"].is_object());
    assert!(json_value["checksums"].is_object());
}

// Test 14: JSON round-trip preserves all data
#[test]
fn test_llm_json_round_trip_preserves_data() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "fn example() {{}}").unwrap();

    let file_path = file.path();
    let error_code = ErrorCode::new("SPL-E002", "warning", "example.rs:5:10", "Example warning");

    let original = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 0, 15)
        .with_symbol("example".to_string(), "function".to_string())
        .with_semantic_info("function", "rust")
        .with_error_code(error_code);

    // Serialize and deserialize
    let json_string = serde_json::to_string(&original).unwrap();
    let deserialized: SpanResult = serde_json::from_str(&json_string).unwrap();

    // Verify all fields preserved
    assert_eq!(deserialized.file_path, original.file_path);
    assert_eq!(deserialized.symbol, original.symbol);
    assert_eq!(deserialized.kind, original.kind);
    assert_eq!(deserialized.byte_start, original.byte_start);
    assert_eq!(deserialized.byte_end, original.byte_end);
    assert_eq!(
        deserialized.semantics.as_ref().map(|s| s.kind.clone()),
        original.semantics.as_ref().map(|s| s.kind.clone())
    );
    assert_eq!(
        deserialized.semantics.as_ref().map(|s| s.language.clone()),
        original.semantics.as_ref().map(|s| s.language.clone())
    );

    // Verify error_code preserved
    assert!(deserialized.error_code.is_some());
    let ec = deserialized.error_code.as_ref().unwrap();
    assert_eq!(ec.code, "SPL-E002");
    assert_eq!(ec.severity, "warning");
    assert_eq!(ec.location, "example.rs:5:10");
}

// Test 15: CLI payload status field consistency
#[test]
fn test_llm_cli_status_field_consistency() {
    // Test success payload
    let success = CliSuccessPayload {
        status: "ok",
        message: "Success".to_string(),
        data: None,
        already_emitted: false,
        has_pending_changes: false,
    };
    let success_json = serde_json::to_value(&success).unwrap();
    assert_eq!(success_json["status"], "ok");
    assert!(
        success_json["status"].is_string(),
        "status should be string"
    );

    // Test error payload
    use splice::SpliceError;
    let error = SpliceError::SymbolNotFound {
        message: "Symbol 'test' not found".to_string(),
        symbol: "test".to_string(),
        file: None,
        hint: "Check that the symbol name is spelled correctly".to_string(),
    };
    let error_payload = CliErrorPayload::from_error(&error);
    let error_json = serde_json::to_value(&error_payload).unwrap();
    assert_eq!(error_json["status"], "error");
    assert!(error_json["status"].is_string(), "status should be string");
}

// Test 16: Nested objects never contain null values for optional subfields
#[test]
fn test_llm_nested_objects_no_null_pollution() {
    // Test 1: Empty relationships uses skip_serializing_if (empty fields omitted)
    let relationships_empty = Relationships {
        callers: vec![],
        callees: vec![],
        imports: vec![],
        exports: vec![],
        cycle_detected: false,
        error_code: None,
    };

    let span_empty = SpanResult::from_byte_span("/path/to/file.rs".to_string(), 0, 10)
        .with_relationships(relationships_empty);

    let json_value_empty = serde_json::to_value(&span_empty).unwrap();
    let json_str_empty = json_value_empty.to_string();

    // When all relationship arrays are empty, the entire relationships object may be omitted
    // or only cycle_detected may be present (false values also use skip_serializing_if)
    // Verify no null values for fields that ARE present
    assert!(
        !json_str_empty.contains("\"callers\":null"),
        "callers should never be null"
    );
    assert!(
        !json_str_empty.contains("\"callees\":null"),
        "callees should never be null"
    );
    assert!(
        !json_str_empty.contains("\"imports\":null"),
        "imports should never be null"
    );
    assert!(
        !json_str_empty.contains("\"exports\":null"),
        "exports should never be null"
    );
    assert!(
        !json_str_empty.contains("\"error_code\":null"),
        "error_code should not be null"
    );

    // Test 2: With actual relationship data
    let relationships_populated = Relationships {
        callers: vec![Relationship {
            rel_type: "caller".to_string(),
            name: "foo".to_string(),
            kind: "function".to_string(),
            file_path: "/path/to/foo.rs".to_string(),
            line_start: 1,
            byte_start: 0,
            byte_end: 10,
        }],
        callees: vec![],
        imports: vec![],
        exports: vec![],
        cycle_detected: false,
        error_code: None,
    };

    let span_populated = SpanResult::from_byte_span("/path/to/file.rs".to_string(), 0, 10)
        .with_relationships(relationships_populated);

    let json_value_populated = serde_json::to_value(&span_populated).unwrap();
    let json_str_populated = json_value_populated.to_string();

    // When relationships has data, it should be serialized as array, not null
    assert!(
        json_str_populated.contains("\"callers\":["),
        "callers should be array"
    );
    assert!(
        !json_str_populated.contains("\"callers\":null"),
        "callers should never be null"
    );
}
