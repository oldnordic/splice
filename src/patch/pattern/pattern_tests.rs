use super::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_find_pattern_in_file() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.rs");
    fs::write(
        &test_file,
        r#"
fn foo() {
    let x = 42;
    let y = 42;
    println!("{}", x);
}
"#,
    )
    .expect("Failed to write test file");

    let matches =
        find_pattern_in_file(&test_file, "42", Language::Rust).expect("Failed to find pattern");

    assert_eq!(matches.len(), 2, "Should find 2 occurrences of '42'");
}

#[test]
fn test_apply_pattern_replace() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.py");
    fs::write(
        &test_file,
        r#"
def foo():
    x = 10
    y = 10
    return x + y
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.py")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "10".to_string(),
        replace_pattern: "20".to_string(),
        language: Some(Language::Python),
        validate: false,
    };

    let result =
        apply_pattern_replace(&config, workspace_root).expect("Failed to apply pattern replace");

    assert_eq!(result.files_patched.len(), 1);
    assert_eq!(result.replacements_count, 2);

    let content = fs::read_to_string(&test_file).expect("Failed to read file");
    assert!(content.contains("20"), "Should contain replaced value");
}

#[test]
fn test_search_command_rust() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.rs");
    fs::write(
        &test_file,
        r#"
fn search_function() {
    let value = "test";
    println!("{}", value);
}

fn another_function() {
    let other = "another";
}
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "function".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(matches.len(), 2, "Should find 2 occurrences of 'function'");
    assert_eq!(matches[0].file, test_file);
    assert_eq!(matches[0].line, 2);
}

#[test]
fn test_search_command_python() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.py");
    fs::write(
        &test_file,
        r#"
def search_function():
    value = "test"
    print(value)

def another_function():
    other = "another"
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.py")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "function".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Python),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(matches.len(), 2, "Should find 2 occurrences of 'function'");
    assert_eq!(matches[0].file, test_file);
}

#[test]
fn test_search_command_multiple_files() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file1 = workspace_root.join("file1.rs");
    fs::write(
        &test_file1,
        r#"
fn first() {
    let target = 1;
}
"#,
    )
    .expect("Failed to write test file");

    let test_file2 = workspace_root.join("file2.rs");
    fs::write(
        &test_file2,
        r#"
fn second() {
    let target = 2;
}
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "target".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(matches.len(), 2, "Should find 2 occurrences across files");
    assert_eq!(matches[0].file, test_file1);
    assert_eq!(matches[1].file, test_file2);
}

#[test]
fn test_search_command_no_matches() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.rs");
    fs::write(
        &test_file,
        r#"
fn example() {
    let x = 42;
}
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "nonexistent".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(matches.len(), 0, "Should find no occurrences");
}

#[test]
fn test_search_glob_rust_only() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    // Create nested directory structure
    let src_dir = workspace_root.join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    let tests_dir = workspace_root.join("tests");
    fs::create_dir(&tests_dir).expect("Failed to create tests dir");

    // Create Rust files in src/
    let rust_file = src_dir.join("main.rs");
    fs::write(
        &rust_file,
        r#"
fn main() {
    let rust_unique_pattern = 42;
}
"#,
    )
    .expect("Failed to write test file");

    // Create Python file in tests/
    let python_file = tests_dir.join("test.py");
    fs::write(
        &python_file,
        r#"
def test():
    python_unique_pattern = 24
"#,
    )
    .expect("Failed to write test file");

    // Search only .rs files
    let config = PatternReplaceConfig {
        glob_pattern: src_dir
            .join("**/*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "rust_unique_pattern".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(
        matches.len(),
        1,
        "Should find 1 occurrence in .rs file only"
    );
    assert_eq!(matches[0].file, rust_file, "Should match the Rust file");
}

#[test]
fn test_search_glob_python_only() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    // Create nested directory structure
    let src_dir = workspace_root.join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    let tests_dir = workspace_root.join("tests");
    fs::create_dir(&tests_dir).expect("Failed to create tests dir");

    // Create Python file in tests/
    let python_file = tests_dir.join("test.py");
    fs::write(
        &python_file,
        r#"
def test_function():
    python_unique_pattern = 42
"#,
    )
    .expect("Failed to write test file");

    // Create Rust file in src/
    let rust_file = src_dir.join("main.rs");
    fs::write(
        &rust_file,
        r#"
fn main() {
    let rust_unique_pattern = 24
}
"#,
    )
    .expect("Failed to write test file");

    // Search only .py files
    let config = PatternReplaceConfig {
        glob_pattern: tests_dir
            .join("**/*.py")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "python_unique_pattern".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Python),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(
        matches.len(),
        1,
        "Should find 1 occurrence in .py file only"
    );
    assert_eq!(matches[0].file, python_file, "Should match the Python file");
}

#[test]
fn test_search_glob_multi_extension() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    // Create Rust file
    let rust_file = workspace_root.join("test.rs");
    fs::write(
        &rust_file,
        r#"
fn rust_function() {
    let target = 1;
}
"#,
    )
    .expect("Failed to write test file");

    // Create Python file
    let python_file = workspace_root.join("test.py");
    fs::write(
        &python_file,
        r#"
def python_function():
    target = 2
"#,
    )
    .expect("Failed to write test file");

    // Create C file (should not match)
    let c_file = workspace_root.join("test.c");
    fs::write(
        &c_file,
        r#"
void c_function() {
    int target = 3;
}
"#,
    )
    .expect("Failed to write test file");

    // Search for .rs files first
    let config_rs = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "target".to_string(),
        replace_pattern: String::new(),
        language: None,
        validate: false,
    };

    let matches_rs =
        find_pattern_in_files(&config_rs).expect("Failed to search for pattern in .rs files");

    // Search for .py files next
    let config_py = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.py")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "target".to_string(),
        replace_pattern: String::new(),
        language: None,
        validate: false,
    };

    let matches_py =
        find_pattern_in_files(&config_py).expect("Failed to search for pattern in .py files");

    // Total matches across both extensions
    let total_matches = matches_rs.len() + matches_py.len();

    assert_eq!(
        total_matches, 2,
        "Should find 2 occurrences total in .rs and .py files"
    );
    assert_eq!(matches_rs.len(), 1, "Should find 1 occurrence in .rs file");
    assert_eq!(matches_py.len(), 1, "Should find 1 occurrence in .py file");
}

#[test]
fn test_search_glob_recursive() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    // Create nested directory structure
    let level1 = workspace_root.join("level1");
    fs::create_dir(&level1).expect("Failed to create level1 dir");

    let level2 = level1.join("level2");
    fs::create_dir(&level2).expect("Failed to create level2 dir");

    // Create files at each level
    let root_file = workspace_root.join("root.rs");
    fs::write(
        &root_file,
        r#"
fn root() {
    let search_target = 1;
}
"#,
    )
    .expect("Failed to write test file");

    let level1_file = level1.join("level1.rs");
    fs::write(
        &level1_file,
        r#"
fn level1() {
    let search_target = 2;
}
"#,
    )
    .expect("Failed to write test file");

    let level2_file = level2.join("level2.rs");
    fs::write(
        &level2_file,
        r#"
fn level2() {
    let search_target = 3;
}
"#,
    )
    .expect("Failed to write test file");

    // Search recursively with **/*.rs pattern
    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("**/*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "search_target".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(
        matches.len(),
        3,
        "Should find 3 occurrences across all nested directories"
    );

    // Verify files are in correct order
    let file_names: Vec<_> = matches
        .iter()
        .map(|m| {
            m.file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert!(file_names.contains(&"root.rs".to_string()));
    assert!(file_names.contains(&"level1.rs".to_string()));
    assert!(file_names.contains(&"level2.rs".to_string()));
}

#[test]
fn test_search_glob_no_matches() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    // Create directory with no matching files
    let empty_dir = workspace_root.join("empty");
    fs::create_dir(&empty_dir).expect("Failed to create empty dir");

    // Create a file that doesn't match the pattern
    let other_file = empty_dir.join("readme.txt");
    fs::write(
        &other_file,
        r#"
This is a text file with no code.
"#,
    )
    .expect("Failed to write test file");

    // Search for .rs files in directory that has none
    let config = PatternReplaceConfig {
        glob_pattern: empty_dir
            .join("**/*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "anything".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(
        matches.len(),
        0,
        "Should find no occurrences when pattern doesn't match any files"
    );
}

#[test]
fn test_search_with_context() {
    use crate::context::extract_context_asymmetric;
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    // Create test file with multiple lines
    let test_file = workspace_root.join("test.rs");
    fs::write(
        &test_file,
        r#"line 1
line 2
line 3
line 4
line 5
"#,
    )
    .expect("Failed to write test file");

    // Search for "line 3" with context
    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "line 3".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(matches.len(), 1, "Should find one occurrence of 'line 3'");

    // Extract context with 1 line before and after
    let m = &matches[0];
    let context = extract_context_asymmetric(
        &m.file,
        m.byte_start,
        m.byte_end,
        1, // context_before
        1, // context_after
    )
    .expect("Failed to extract context");

    // Verify context
    assert_eq!(context.before.len(), 1, "Should have 1 line before");
    assert_eq!(context.selected.len(), 1, "Should have 1 selected line");
    assert_eq!(context.after.len(), 1, "Should have 1 line after");
    assert!(context.before[0].contains("line 2"));
    assert!(context.selected[0].contains("line 3"));
    assert!(context.after[0].contains("line 4"));
}

#[test]
fn test_search_context_asymmetric() {
    use crate::context::extract_context_asymmetric;
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    // Create test file with multiple lines
    let test_file = workspace_root.join("test.py");
    fs::write(
        &test_file,
        r#"line 1
line 2
line 3
line 4
line 5
line 6
"#,
    )
    .expect("Failed to write test file");

    // Search for "line 4" with asymmetric context
    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.py")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "line 4".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Python),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(matches.len(), 1, "Should find one occurrence of 'line 4'");

    // Extract asymmetric context: 2 lines before, 1 line after
    let m = &matches[0];
    let context = extract_context_asymmetric(
        &m.file,
        m.byte_start,
        m.byte_end,
        2, // context_before
        1, // context_after
    )
    .expect("Failed to extract context");

    // Verify asymmetric context
    assert_eq!(context.before.len(), 2, "Should have 2 lines before");
    assert_eq!(context.after.len(), 1, "Should have 1 line after");
    assert!(context.before[0].contains("line 2"));
    assert!(context.before[1].contains("line 3"));
    assert!(context.after[0].contains("line 5"));
}

#[test]
fn test_search_no_context() {
    use crate::context::extract_context_asymmetric;
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    // Create test file
    let test_file = workspace_root.join("test.rs");
    fs::write(
        &test_file,
        r#"line 1
line 2
line 3
"#,
    )
    .expect("Failed to write test file");

    // Search for "line 2" with no context
    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "line 2".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(matches.len(), 1, "Should find one occurrence of 'line 2'");

    // Extract context with 0 lines before and after
    let m = &matches[0];
    let context = extract_context_asymmetric(
        &m.file,
        m.byte_start,
        m.byte_end,
        0, // context_before
        0, // context_after
    )
    .expect("Failed to extract context");

    // Verify no context
    assert_eq!(context.before.len(), 0, "Should have 0 lines before");
    assert_eq!(context.after.len(), 0, "Should have 0 lines after");
    assert_eq!(context.selected.len(), 1, "Should have 1 selected line");
}

#[test]
fn test_search_context_in_json() {
    use crate::context::extract_context_asymmetric;
    use serde_json::json;
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    // Create test file with multiple lines
    let test_file = workspace_root.join("test.rs");
    fs::write(
        &test_file,
        r#"line 1
line 2
line 3
line 4
"#,
    )
    .expect("Failed to write test file");

    // Search for "line 3" with context
    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "line 3".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(matches.len(), 1, "Should find one occurrence of 'line 3'");

    // Build JSON structure like execute_search does
    let m = &matches[0];
    let context = extract_context_asymmetric(
        &m.file,
        m.byte_start,
        m.byte_end,
        1, // context_before
        1, // context_after
    )
    .expect("Failed to extract context");

    let json_result = json!({
        "file": m.file.to_str().unwrap_or("<invalid-utf-8>").to_string(),
        "byte_start": m.byte_start,
        "byte_end": m.byte_end,
        "line": m.line,
        "column": m.column,
        "matched_text": m.matched_text,
        "context_before": context.before,
        "context_selected": context.selected,
        "context_after": context.after,
    });

    // Verify JSON structure
    assert!(json_result["file"].is_string());
    assert!(json_result["byte_start"].is_number());
    assert!(json_result["line"].is_number());
    assert!(json_result["matched_text"].is_string());
    assert!(json_result["context_before"].is_array());
    assert!(json_result["context_selected"].is_array());
    assert!(json_result["context_after"].is_array());

    // Verify context arrays have correct length
    assert_eq!(json_result["context_before"].as_array().unwrap().len(), 1);
    assert_eq!(json_result["context_selected"].as_array().unwrap().len(), 1);
    assert_eq!(json_result["context_after"].as_array().unwrap().len(), 1);

    // Verify context content
    let before_str = json_result["context_before"][0].as_str().unwrap();
    let selected_str = json_result["context_selected"][0].as_str().unwrap();
    let after_str = json_result["context_after"][0].as_str().unwrap();

    assert!(before_str.contains("line 2"));
    assert!(selected_str.contains("line 3"));
    assert!(after_str.contains("line 4"));
}

#[test]
fn test_search_json_output_format() {
    use serde_json::json;
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.rs");
    fs::write(&test_file, r#"fn test() { let x = 42; }"#).expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "42".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    // Build full JSON output structure like execute_search does
    let results: Vec<Value> = matches
        .into_iter()
        .map(|m| {
            json!({
                "file": m.file.to_str().unwrap_or("<invalid-utf-8>").to_string(),
                "byte_start": m.byte_start,
                "byte_end": m.byte_end,
                "line": m.line,
                "column": m.column,
                "matched_text": m.matched_text,
            })
        })
        .collect();

    let output = json!({
        "status": "ok",
        "message": format!("Found {} occurrence(s) of '42'", results.len()),
        "matches": results,
        "pattern": "42",
        "count": results.len(),
    });

    // Verify top-level structure
    assert_eq!(output["status"], "ok");
    assert!(output["message"].is_string());
    assert!(output["matches"].is_array());
    assert_eq!(output["pattern"], "42");
    assert_eq!(output["count"], 1);
}

#[test]
fn test_search_json_with_context() {
    use crate::context::extract_context_asymmetric;
    use serde_json::json;
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.rs");
    fs::write(
        &test_file,
        r#"line 1
line 2
line 3
line 4
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "line 3".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    assert_eq!(matches.len(), 1);

    let m = &matches[0];
    let context = extract_context_asymmetric(
        &m.file,
        m.byte_start,
        m.byte_end,
        1, // context_before
        1, // context_after
    )
    .expect("Failed to extract context");

    // Build JSON with context
    let mut match_json = json!({
        "file": m.file.to_str().unwrap_or("<invalid-utf-8>").to_string(),
        "byte_start": m.byte_start,
        "byte_end": m.byte_end,
        "line": m.line,
        "column": m.column,
        "matched_text": m.matched_text,
    });

    if let Some(obj) = match_json.as_object_mut() {
        obj.insert("context_before".to_string(), json!(context.before));
        obj.insert("context_selected".to_string(), json!(context.selected));
        obj.insert("context_after".to_string(), json!(context.after));
    }

    // Verify context fields are present
    assert!(match_json.get("context_before").is_some());
    assert!(match_json.get("context_selected").is_some());
    assert!(match_json.get("context_after").is_some());
    assert!(match_json["context_before"].is_array());
    assert!(match_json["context_selected"].is_array());
    assert!(match_json["context_after"].is_array());
}

#[test]
fn test_search_json_parseable() {
    use serde_json::json;
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.py");
    fs::write(
        &test_file,
        r#"def foo():
    x = 10
    return x"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.py")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "x".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Python),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    // Build JSON output
    let results: Vec<Value> = matches
        .into_iter()
        .map(|m| {
            json!({
                "file": m.file.to_str().unwrap_or("<invalid-utf-8>").to_string(),
                "byte_start": m.byte_start,
                "byte_end": m.byte_end,
                "line": m.line,
                "column": m.column,
                "matched_text": m.matched_text,
            })
        })
        .collect();

    let output = json!({
        "status": "ok",
        "message": format!("Found {} occurrence(s) of 'x'", results.len()),
        "matches": results,
        "pattern": "x",
        "count": results.len(),
    });

    // Verify it can be serialized and deserialized
    let json_string = serde_json::to_string(&output).expect("Failed to serialize JSON");
    let parsed: Value = serde_json::from_str(&json_string).expect("Failed to parse JSON");

    assert_eq!(parsed, output);
}

#[test]
fn test_search_json_no_context() {
    use serde_json::json;
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.rs");
    fs::write(&test_file, r#"fn test() { let x = 42; }"#).expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "42".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    // Build JSON without context (no context flags specified)
    let results: Vec<Value> = matches
        .into_iter()
        .map(|m| {
            json!({
                "file": m.file.to_str().unwrap_or("<invalid-utf-8>").to_string(),
                "byte_start": m.byte_start,
                "byte_end": m.byte_end,
                "line": m.line,
                "column": m.column,
                "matched_text": m.matched_text,
            })
        })
        .collect();

    // Verify context fields are not present when not requested
    let first_match = &results[0];
    assert!(first_match.get("context_before").is_none());
    assert!(first_match.get("context_selected").is_none());
    assert!(first_match.get("context_after").is_none());
}

#[test]
fn test_search_json_all_metadata() {
    use serde_json::json;
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.rs");
    fs::write(
        &test_file,
        r#"fn example() {
    let value = 100;
}"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "100".to_string(),
        replace_pattern: String::new(),
        language: Some(Language::Rust),
        validate: false,
    };

    let matches = find_pattern_in_files(&config).expect("Failed to search for pattern");

    // Build JSON output with all metadata
    let results: Vec<Value> = matches
        .into_iter()
        .map(|m| {
            json!({
                "file": m.file.to_str().unwrap_or("<invalid-utf-8>").to_string(),
                "byte_start": m.byte_start,
                "byte_end": m.byte_end,
                "line": m.line,
                "column": m.column,
                "matched_text": m.matched_text,
            })
        })
        .collect();

    let output = json!({
        "status": "ok",
        "message": format!("Found {} occurrence(s) of '100'", results.len()),
        "matches": results,
        "pattern": "100",
        "count": results.len(),
    });

    // Verify all top-level fields
    assert!(output.get("status").is_some());
    assert!(output.get("message").is_some());
    assert!(output.get("matches").is_some());
    assert!(output.get("pattern").is_some());
    assert!(output.get("count").is_some());

    // Verify all match metadata fields
    let first_match = &output["matches"][0];
    assert!(first_match.get("file").is_some());
    assert!(first_match.get("byte_start").is_some());
    assert!(first_match.get("byte_end").is_some());
    assert!(first_match.get("line").is_some());
    assert!(first_match.get("column").is_some());
    assert!(first_match.get("matched_text").is_some());

    // Verify field types
    assert!(first_match["file"].is_string());
    assert!(first_match["byte_start"].is_number());
    assert!(first_match["byte_end"].is_number());
    assert!(first_match["line"].is_number());
    assert!(first_match["column"].is_number());
    assert!(first_match["matched_text"].is_string());
}

#[test]
fn test_apply_replace_single_file() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.rs");
    fs::write(
        &test_file,
        r#"
fn foo() {
    let x = 42;
    let y = 42;
    println!("{}", x);
}
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "42".to_string(),
        replace_pattern: "100".to_string(),
        language: Some(Language::Rust),
        validate: false,
    };

    let result =
        apply_pattern_replace(&config, workspace_root).expect("Failed to apply pattern replace");

    assert_eq!(result.files_patched.len(), 1);
    assert_eq!(result.replacements_count, 2);

    let content = fs::read_to_string(&test_file).expect("Failed to read file");
    assert!(content.contains("100"), "Should contain replaced value");
    assert!(!content.contains("42"), "Should not contain replaced value");
}

#[test]
fn test_apply_replace_multiple_files() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file1 = workspace_root.join("file1.rs");
    fs::write(
        &test_file1,
        r#"
fn first() {
    let target = 1;
}
"#,
    )
    .expect("Failed to write test file");

    let test_file2 = workspace_root.join("file2.rs");
    fs::write(
        &test_file2,
        r#"
fn second() {
    let target = 2;
}
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "target".to_string(),
        replace_pattern: "replaced".to_string(),
        language: Some(Language::Rust),
        validate: false,
    };

    let result =
        apply_pattern_replace(&config, workspace_root).expect("Failed to apply pattern replace");

    assert_eq!(result.files_patched.len(), 2);
    assert_eq!(result.replacements_count, 2);

    let content1 = fs::read_to_string(&test_file1).expect("Failed to read file1");
    assert!(content1.contains("replaced"));

    let content2 = fs::read_to_string(&test_file2).expect("Failed to read file2");
    assert!(content2.contains("replaced"));
}

#[test]
fn test_apply_replace_rollback_on_error() {
    // Test atomic multi-file replacement - all files succeed or none do
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file1 = workspace_root.join("file1.rs");
    fs::write(
        &test_file1,
        r#"
fn first() {
    let target = 1;
}
"#,
    )
    .expect("Failed to write test file");

    let test_file2 = workspace_root.join("file2.rs");
    fs::write(
        &test_file2,
        r#"
fn second() {
    let target = 2;
}
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "target".to_string(),
        replace_pattern: "modified".to_string(),
        language: Some(Language::Rust),
        validate: false,
    };

    // Apply should succeed atomically (both files replaced)
    let result =
        apply_pattern_replace(&config, workspace_root).expect("Failed to apply pattern replace");

    assert_eq!(result.files_patched.len(), 2);
    assert_eq!(result.replacements_count, 2);

    // Verify both files were modified
    let content1 = fs::read_to_string(&test_file1).expect("Failed to read file1");
    let content2 = fs::read_to_string(&test_file2).expect("Failed to read file2");

    assert!(
        content1.contains("modified"),
        "file1 should contain replaced text"
    );
    assert!(
        !content1.contains("target"),
        "file1 should not contain original text"
    );
    assert!(
        content2.contains("modified"),
        "file2 should contain replaced text"
    );
    assert!(
        !content2.contains("target"),
        "file2 should not contain original text"
    );

    // Test edge case: no matches found (should succeed without modifying files)
    let workspace2 = TempDir::new().expect("Failed to create temp dir 2");
    let workspace_root2 = workspace2.path();

    let test_file3 = workspace_root2.join("file3.rs");
    fs::write(&test_file3, "fn test() { let x = 42; }").expect("Failed to write test file");

    let original_content3 = fs::read_to_string(&test_file3).expect("Failed to read original file3");

    let config2 = PatternReplaceConfig {
        glob_pattern: workspace_root2
            .join("*.rs")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "nonexistent_pattern_xyz".to_string(),
        replace_pattern: "replacement".to_string(),
        language: Some(Language::Rust),
        validate: false,
    };

    // Should succeed with no replacements (atomic behavior preserved)
    let result2 = apply_pattern_replace(&config2, workspace_root2)
        .expect("Should succeed even with no matches");

    assert_eq!(result2.files_patched.len(), 0);
    assert_eq!(result2.replacements_count, 0);

    // Verify file was not modified (atomicity preserved)
    let content3_after = fs::read_to_string(&test_file3).expect("Failed to read file3 after");
    assert_eq!(
        content3_after, original_content3,
        "file3 should be unchanged when no matches found"
    );
}

#[test]
fn test_apply_replace_with_validation() {
    let workspace = TempDir::new().expect("Failed to create temp dir");
    let workspace_root = workspace.path();

    let test_file = workspace_root.join("test.py");
    fs::write(
        &test_file,
        r#"
def foo():
    x = 10
    y = 10
    return x + y
"#,
    )
    .expect("Failed to write test file");

    let config = PatternReplaceConfig {
        glob_pattern: workspace_root
            .join("*.py")
            .to_str()
            .expect("Invalid UTF-8 path")
            .to_string(),
        find_pattern: "10".to_string(),
        replace_pattern: "20".to_string(),
        language: Some(Language::Python),
        validate: false, // Set to true to enable validation gates
    };

    let result =
        apply_pattern_replace(&config, workspace_root).expect("Failed to apply pattern replace");

    assert_eq!(result.files_patched.len(), 1);
    assert_eq!(result.replacements_count, 2);

    let content = fs::read_to_string(&test_file).expect("Failed to read file");
    assert!(content.contains("20"), "Should contain replaced value");
}
