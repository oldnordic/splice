//! Integration tests for io error path preservation (Bug B3).
//!
//! Verifies that `splice patch` errors include the real file path in the
//! error message, not the literal `<unknown>` placeholder produced by
//! the bare `?` propagation of `std::io::Error`.

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::NamedTempFile;

    fn get_splice_binary() -> PathBuf {
        if let Ok(path) = std::env::var("SPLICE_TEST_BIN") {
            return PathBuf::from(path);
        }
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_splice") {
            return PathBuf::from(path);
        }
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // deps
        path.pop(); // debug
        path.push("splice");
        path
    }

    fn make_temp_replacement() -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        file.write_all(b"pub fn x() {}\n").unwrap();
        file
    }

    #[test]
    fn test_patch_missing_source_file_reports_real_path() {
        let replacement = make_temp_replacement();
        let missing_source = "/nonexistent_splice_test_dir/missing_source_file.rs";

        let output = Command::new(get_splice_binary())
            .arg("patch")
            .arg("--file")
            .arg(missing_source)
            .arg("--symbol")
            .arg("x")
            .arg("--with")
            .arg(replacement.path())
            .arg("--output")
            .arg("json")
            .output()
            .expect("failed to run splice");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);

        assert!(
            !output.status.success(),
            "splice should fail when --file is missing"
        );
        assert!(
            !combined.contains("I/O error for path <unknown>"),
            "error message should not contain B3-style 'I/O error for path <unknown>'. Got: {}",
            combined
        );
        assert!(
            combined.contains(missing_source),
            "error message should mention the real missing path '{}'. Got: {}",
            missing_source,
            combined
        );
    }

    #[test]
    fn test_patch_missing_replacement_file_reports_real_path() {
        // Create a real source file so the source-read step succeeds.
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            "[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        let source = temp_dir.path().join("src.rs");
        std::fs::write(&source, "pub fn x() {}\n").unwrap();

        let missing_replacement = "/nonexistent_splice_test_dir/missing_replacement.rs";

        let output = Command::new(get_splice_binary())
            .arg("patch")
            .arg("--file")
            .arg(&source)
            .arg("--symbol")
            .arg("x")
            .arg("--with")
            .arg(missing_replacement)
            .arg("--output")
            .arg("json")
            .output()
            .expect("failed to run splice");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);

        assert!(
            !output.status.success(),
            "splice should fail when --with is missing"
        );
        assert!(
            !combined.contains("I/O error for path <unknown>"),
            "error message should not contain B3-style 'I/O error for path <unknown>'. Got: {}",
            combined
        );
        assert!(
            combined.contains(missing_replacement),
            "error message should mention the real missing path '{}'. Got: {}",
            missing_replacement,
            combined
        );
    }
}
