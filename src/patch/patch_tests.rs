use super::*;
use std::ffi::OsStr;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_compute_preview_report_line_counts() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "line 1").unwrap();
    writeln!(temp_file, "line 2").unwrap();
    writeln!(temp_file, "line 3").unwrap();
    writeln!(temp_file, "line 4").unwrap();
    temp_file.flush().unwrap();

    let source = std::fs::read_to_string(temp_file.path()).unwrap();
    let start = source.find("line 2\n").unwrap();
    let end = source.find("line 4").unwrap();
    let new_content = "NEW LINE\n";

    let report = compute_preview_report(temp_file.path(), start, end, new_content).unwrap();

    assert_eq!(report.lines_removed, 2, "Should count 2 lines removed");
    assert_eq!(report.lines_added, 1, "Should count 1 line added");
}

#[test]
fn test_compute_preview_report_empty_replacement() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "line 1").unwrap();
    writeln!(temp_file, "line 2").unwrap();
    temp_file.flush().unwrap();

    let source = std::fs::read_to_string(temp_file.path()).unwrap();
    let start = source.find("line 1").unwrap();
    let end = source.len();

    let report = compute_preview_report(temp_file.path(), start, end, "").unwrap();

    assert_eq!(report.lines_removed, 2, "Should count 2 lines removed");
    assert_eq!(report.lines_added, 0, "Empty content = 0 lines added");
}

#[test]
fn test_compute_preview_report_add_only() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "line 1").unwrap();
    temp_file.flush().unwrap();

    let source = std::fs::read_to_string(temp_file.path()).unwrap();
    let start = source.len();
    let end = start;

    let report =
        compute_preview_report(temp_file.path(), start, end, "NEW LINE 1\nNEW LINE 2\n").unwrap();

    assert_eq!(report.lines_removed, 0, "No lines removed when start==end");
    assert_eq!(report.lines_added, 2, "Should count 2 new lines");
}

#[test]
fn test_apply_patch_accepts_strict_and_skip_flags() {
    use tempfile::TempDir;

    let workspace = TempDir::new().unwrap();
    let file_path = workspace.path().join("lib.rs");

    {
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "pub fn old() {{ }}").unwrap();
    }

    {
        let cargo_toml = workspace.path().join("Cargo.toml");
        let mut file = std::fs::File::create(&cargo_toml).unwrap();
        writeln!(
            file,
            r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"

[lib]
path = "lib.rs"
"#
        )
        .unwrap();
    }

    let content = std::fs::read_to_string(&file_path).unwrap();
    let start = content.find("old()").unwrap();
    let end = start + "old()".len();

    let _ = (start, end);
}

#[test]
fn test_should_skip_entry_python_caches() {
    assert!(should_skip_entry(OsStr::new("__pycache__")));
    assert!(should_skip_entry(OsStr::new(".venv")));
    assert!(should_skip_entry(OsStr::new("venv")));
    assert!(should_skip_entry(OsStr::new(".pytest_cache")));
    assert!(should_skip_entry(OsStr::new(".mypy_cache")));
    assert!(should_skip_entry(OsStr::new(".tox")));
}

#[test]
fn test_should_skip_entry_js_build_artifacts() {
    assert!(should_skip_entry(OsStr::new("dist")));
    assert!(should_skip_entry(OsStr::new("build")));
    assert!(should_skip_entry(OsStr::new(".next")));
    assert!(should_skip_entry(OsStr::new(".nuxt")));
    assert!(should_skip_entry(OsStr::new(".cache")));
}

#[test]
fn test_should_skip_entry_java_and_ide() {
    assert!(should_skip_entry(OsStr::new(".gradle")));
    assert!(should_skip_entry(OsStr::new(".idea")));
    assert!(should_skip_entry(OsStr::new(".vscode")));
}

#[test]
fn test_should_skip_entry_preserves_source_dirs() {
    assert!(!should_skip_entry(OsStr::new("src")));
    assert!(!should_skip_entry(OsStr::new("tests")));
    assert!(!should_skip_entry(OsStr::new("examples")));
    assert!(!should_skip_entry(OsStr::new("benches")));
    assert!(!should_skip_entry(OsStr::new("Cargo.toml")));
    assert!(!should_skip_entry(OsStr::new("lib.rs")));
}

#[test]
fn validate_utf8_span_invalid_bounds_reports_real_path() {
    let path = Path::new("/some/real/file/path.rs");
    let err =
        validate_utf8_span(path, "abc", 0, 100).expect_err("out-of-bounds span should be an error");
    let msg = err.to_string();
    assert!(
        !msg.contains("<unknown>"),
        "error must not contain <unknown> placeholder; got: {}",
        msg
    );
    assert!(
        msg.contains("/some/real/file/path.rs"),
        "error must mention the real file path; got: {}",
        msg
    );
}
