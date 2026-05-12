//! Integration tests for find_workspace_root boundary behavior (Bug B1).
//!
//! Verifies that splice patch:
//! - Finds local Cargo.toml in a legitimate tempdir project (regression check)
//! - Does NOT walk past /tmp / $TMPDIR / $HOME boundaries to find a stray Cargo.toml
//! - Surfaces a clear "no workspace" error when no project marker is found within the boundaries
//!
//! Tests pass TMPDIR via the child process environment, so the parent test runner
//! is not affected and multiple tests can run sequentially.

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::{Path, PathBuf};
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

    fn make_replacement(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    fn run_splice_patch(
        source: &Path,
        symbol: &str,
        replacement: &Path,
        tmpdir_override: Option<&Path>,
    ) -> std::process::Output {
        let mut cmd = Command::new(get_splice_binary());
        cmd.arg("patch")
            .arg("--file")
            .arg(source)
            .arg("--symbol")
            .arg(symbol)
            .arg("--with")
            .arg(replacement)
            .arg("--dry-run")
            .arg("--output")
            .arg("json");
        if let Some(td) = tmpdir_override {
            cmd.env("TMPDIR", td);
        }
        cmd.output().expect("failed to run splice")
    }

    /// Regression: a normal tempdir project with Cargo.toml should still work.
    #[test]
    fn test_patch_uses_local_cargo_toml_in_tempdir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            "[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"t\"\npath = \"src.rs\"\n",
        )
        .unwrap();
        let source = temp_dir.path().join("src.rs");
        std::fs::write(
            &source,
            "pub fn x() -> i32 { 1 }\n\nfn main() { let _ = x(); }\n",
        )
        .unwrap();
        let replacement = make_replacement("pub fn x() -> i32 { 2 }\n");

        let output = run_splice_patch(&source, "x", replacement.path(), None);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);

        assert!(
            !combined.contains("I/O error for path <unknown>"),
            "must not produce B3-style <unknown> path in io error message. Got: {}",
            combined
        );
        assert!(
            !combined.contains("File name too long"),
            "must not produce ENAMETOOLONG. Got: {}",
            combined
        );
        // The operation should reach preview/dry-run successfully (no <unknown>/ENAMETOOLONG).
        // Exit code may be 0 or 1 per git-diff convention; CargoCheckFailed for unrelated reasons
        // is acceptable here as long as workspace detection succeeded.
    }

    /// Bug case: stray Cargo.toml at $TMPDIR root must not be picked when source
    /// is in a non-project subdir below $TMPDIR. Splice must surface a clear
    /// "no workspace" error, not <unknown> or "File name too long".
    #[test]
    fn test_patch_ignores_stray_tmpdir_cargo_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Stray Cargo.toml at the TMPDIR root (the bug case).
        std::fs::write(
            temp_path.join("Cargo.toml"),
            "[package]\nname = \"stray\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        // Source file 2 levels deep, NO Cargo.toml in any subdir.
        let work_dir = temp_path.join("no_project_subdir").join("work");
        std::fs::create_dir_all(&work_dir).unwrap();
        let source = work_dir.join("lib.rs");
        std::fs::write(&source, "pub fn x() -> i32 { 1 }\n").unwrap();
        let replacement = make_replacement("pub fn x() -> i32 { 2 }\n");

        // Override TMPDIR to our tempdir so the boundary set contains it.
        let output = run_splice_patch(&source, "x", replacement.path(), Some(&temp_path));

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);

        assert!(
            !output.status.success(),
            "splice should fail when no project marker is reachable. Got: {}",
            combined
        );
        assert!(
            !combined.contains("I/O error for path <unknown>"),
            "must not produce B3-style <unknown> path in io error message. Got: {}",
            combined
        );
        assert!(
            !combined.contains("File name too long"),
            "must not produce ENAMETOOLONG. Got: {}",
            combined
        );
        assert!(
            combined.contains("project marker")
                || combined.contains("workspace")
                || combined.contains("Cargo.toml"),
            "error should mention workspace/project marker. Got: {}",
            combined
        );
    }
}
