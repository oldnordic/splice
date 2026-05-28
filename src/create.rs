//! File creation with validation
//!
//! Provides functionality for creating new Rust files with rust-analyzer validation.

use crate::error::{Result, SpliceError};
use crate::io_ext;
use crate::validate::validate_rust_snippet;
use crate::write::write_atomic;
use std::fs;
use std::path::Path;

/// Create a new file with validation
///
/// # Arguments
/// * `file_path` - Path where file should be created
/// * `code` - Rust source code to write
/// * `workspace_dir` - Workspace directory (for context)
/// * `validate_only` - If true, validate but don't write file
///
/// # Returns
/// * `Ok(ValidationResult)` - Validation result
/// * `Err(SpliceError)` - File creation or validation failed
///
/// # Behavior
/// - If `validate_only`: validates code and returns result, no file written
/// - If not `validate_only`: validates first, then writes file if valid
/// - Never overwrites existing files (returns error)
/// - Uses atomic file writing (via write_atomic)
pub fn create_file_with_validation(
    file_path: &Path,
    code: &str,
    workspace_dir: &Path,
    validate_only: bool,
) -> Result<crate::validate::SnippetValidation> {
    // Step 1: Check if file already exists
    if file_path.exists() {
        return Err(SpliceError::IoContext {
            context: format!("File already exists: {}", file_path.display()),
            source: std::io::Error::new(std::io::ErrorKind::AlreadyExists, "file exists"),
        });
    }

    // Step 2: Validate code with rustc (standalone snippet, not cargo check)
    let validation = validate_rust_snippet(code, Some(workspace_dir))?;

    // Step 3: If validate-only, return result without writing
    if validate_only {
        return Ok(validation);
    }

    // Step 4: If validation failed, return error (don't write file)
    if !validation.is_valid {
        return Err(SpliceError::IoContext {
            context: format!(
                "Code validation failed with {} error(s). Use --validate-only to see errors.",
                validation.errors.len()
            ),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, "validation failed"),
        });
    }

    // Step 5: Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|source| SpliceError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    // Step 6: Write file atomically
    write_atomic(file_path, code.as_bytes(), "create")?;

    Ok(validation)
}

/// Create a new file and add module declaration to parent mod.rs/lib.rs
///
/// # Arguments
/// * `file_path` - Path where file should be created
/// * `code` - Rust source code to write
/// * `workspace_dir` - Workspace directory
/// * `add_mod_declaration` - If true, add `mod filename;` to parent module
///
/// # Returns
/// * `Ok(ValidationResult)` - Validation result
/// * `Err(SpliceError)` - Creation failed
pub fn create_file_with_module(
    file_path: &Path,
    code: &str,
    workspace_dir: &Path,
    add_mod_declaration: bool,
) -> Result<crate::validate::SnippetValidation> {
    // First, create the file
    let validation = create_file_with_validation(file_path, code, workspace_dir, false)?;

    // If requested, add module declaration
    if add_mod_declaration {
        add_module_declaration(file_path, workspace_dir)?;
    }

    Ok(validation)
}

/// Add module declaration to parent module file
///
/// If file_path is `src/commands/new.rs`, adds `mod new;` to `src/commands/mod.rs`
/// or `src/commands.rs` if it exists.
fn add_module_declaration(file_path: &Path, workspace_dir: &Path) -> Result<()> {
    // Extract module name from file path
    let file_stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| SpliceError::IoContext {
            context: "Cannot extract module name from file path".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"),
        })?;

    // Find parent module file (mod.rs or same name as directory)
    let parent_dir = file_path.parent().ok_or_else(|| SpliceError::IoContext {
        context: "File has no parent directory".to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent"),
    })?;

    // Check for mod.rs first
    let mod_rs = parent_dir.join("mod.rs");
    let parent_module = if mod_rs.exists() {
        mod_rs
    } else {
        // Check for {dirname}.rs
        if let Some(dir_name) = parent_dir.file_name().and_then(|n| n.to_str()) {
            parent_dir
                .parent()
                .expect("invariant: parent_dir always has a parent when file_name is Some")
                .join(format!("{}.rs", dir_name))
        } else {
            // Fallback to lib.rs (if in src/)
            if parent_dir.ends_with("src") {
                workspace_dir.join("src").join("lib.rs")
            } else {
                return Err(SpliceError::IoContext {
                    context: "Cannot determine parent module file".to_string(),
                    source: std::io::Error::new(std::io::ErrorKind::NotFound, "no parent module"),
                });
            }
        }
    };

    // Check if module declaration already exists
    if parent_module.exists() {
        let content = io_ext::read_to_string(&parent_module)?;
        let mod_decl = format!("mod {};", file_stem);

        if content.contains(&mod_decl) {
            // Already exists, nothing to do
            return Ok(());
        }

        // Add module declaration
        let mod_line = format!("mod {};", file_stem);
        let new_content = if content.trim_end().ends_with('}') {
            // Insert before last closing brace
            if let Some(pos) = content.rfind('}') {
                let mut updated = content.clone();
                updated.insert_str(pos, &format!("{}\n", mod_line));
                updated
            } else {
                format!("{}\n{}\n", content.trim_end(), mod_line)
            }
        } else {
            format!("{}\n{}\n", content.trim_end(), mod_line)
        };

        io_ext::write(&parent_module, new_content)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_file_with_valid_code() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_module.rs");

        let code = r#"
        pub fn test_function() -> i32 {
            42
        }
        "#;

        let result = create_file_with_validation(
            &file_path,
            code,
            temp_dir.path(),
            false, // validate_only = false
        );

        assert!(result.is_ok());
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content.trim(), code.trim());
    }

    #[test]
    fn test_create_file_rejects_invalid_code() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid.rs");

        let code = r#"
        pub fn invalid() {
            let x: NonExistentType = 5;
        }
        "#;

        let result = create_file_with_validation(&file_path, code, temp_dir.path(), false);

        assert!(result.is_err());
        assert!(!file_path.exists());
    }

    #[test]
    fn test_validate_only_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let code = r#"
        pub fn test() -> i32 { 42 }
        "#;

        let result = create_file_with_validation(
            &file_path,
            code,
            temp_dir.path(),
            true, // validate_only = true
        );

        assert!(result.is_ok());
        assert!(!file_path.exists()); // File should NOT exist
    }

    #[test]
    fn test_prevent_overwrite_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.rs");

        // Create file first
        fs::write(&file_path, "old content").unwrap();

        let code = r#"
        pub fn new() -> i32 { 42 }
        "#;

        let result = create_file_with_validation(&file_path, code, temp_dir.path(), false);

        assert!(result.is_err()); // Should fail

        // File should still have old content
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "old content");
    }

    #[test]
    fn test_create_file_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("dir").join("file.rs");

        let code = r#"
        pub fn nested() -> i32 { 42 }
        "#;

        let result = create_file_with_validation(&nested_path, code, temp_dir.path(), false);

        assert!(result.is_ok());
        assert!(nested_path.exists());
        assert!(nested_path.parent().unwrap().exists());
    }

    #[test]
    fn test_create_file_with_empty_code() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.rs");

        let code = "";

        let result = create_file_with_validation(&file_path, code, temp_dir.path(), false);

        assert!(result.is_ok());
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_add_module_declaration_to_mod_rs() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let commands_dir = src_dir.join("commands");
        fs::create_dir_all(&commands_dir).unwrap();

        // Create mod.rs
        let mod_rs = commands_dir.join("mod.rs");
        fs::write(&mod_rs, "// Module file\n").unwrap();

        // Create new file
        let new_file = commands_dir.join("new.rs");
        fs::write(&new_file, "pub fn new() {}").unwrap();

        // Add module declaration
        let result = add_module_declaration(&new_file, temp_dir.path());

        assert!(result.is_ok());

        // Check mod.rs contains the declaration
        let content = fs::read_to_string(&mod_rs).unwrap();
        assert!(content.contains("mod new;"));
    }

    #[test]
    fn test_add_module_declaration_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        // Create mod.rs with existing declaration
        let mod_rs = src_dir.join("mod.rs");
        fs::write(&mod_rs, "mod test;\n").unwrap();

        // Create test.rs
        let test_file = src_dir.join("test.rs");
        fs::write(&test_file, "pub fn test() {}").unwrap();

        // Add module declaration (should not duplicate)
        let result = add_module_declaration(&test_file, temp_dir.path());
        assert!(result.is_ok());

        let content = fs::read_to_string(&mod_rs).unwrap();
        // Count occurrences of "mod test;"
        let count = content.matches("mod test;").count();
        assert_eq!(count, 1, "Module declaration should not be duplicated");
    }
}
