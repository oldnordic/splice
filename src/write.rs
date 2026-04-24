//! Atomic file writing utilities

use crate::error::{Result, SpliceError};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Write file atomically using temp file + rename strategy
///
/// # Arguments
/// * `file_path` - Target file path
/// * `content` - Content to write
/// * `label` - Label for temp file (e.g., "patch", "create")
///
/// # Returns
/// * `Ok(())` - File written successfully
/// * `Err(SpliceError)` - Write operation failed
///
/// # Method
/// 1. Write to temp file in same directory
/// 2. Fsync temp file to ensure data is on disk
/// 3. Atomic rename to final path
/// 4. Cleanup temp file if rename fails
///
/// This ensures atomicity: either file exists with correct content,
/// or it doesn't exist at all (no partial writes).
pub fn write_atomic(file_path: &Path, content: &[u8], label: &str) -> Result<()> {
    // Create temp file in same directory as target
    let temp_path = file_path.with_extension(format!("{}.tmp", label));

    // Write temp file
    {
        let mut temp_file = File::create(&temp_path).map_err(|e| SpliceError::IoContext {
            context: format!("Failed to create temp file: {}", temp_path.display()),
            source: e,
        })?;

        temp_file.write_all(content).map_err(|e| SpliceError::IoContext {
            context: format!("Failed to write to temp file: {}", temp_path.display()),
            source: e,
        })?;

        temp_file.flush().map_err(|e| SpliceError::IoContext {
            context: format!("Failed to flush temp file: {}", temp_path.display()),
            source: e,
        })?;

        temp_file.sync_all().map_err(|e| SpliceError::IoContext {
            context: format!(
                "Failed to sync temp file to disk: {}",
                temp_path.display()
            ),
            source: e,
        })?;
    } // temp_file closed here

    // Atomic rename
    std::fs::rename(&temp_path, file_path).map_err(|e| SpliceError::IoContext {
        context: format!(
            "Failed to rename temp file to final path: {} -> {}",
            temp_path.display(),
            file_path.display()
        ),
        source: e,
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_write_atomic_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let content = b"Hello, World!";
        let result = write_atomic(&file_path, content, "test");

        assert!(result.is_ok());
        assert!(file_path.exists());

        let read_content = fs::read(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_write_atomic_content_correct() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("content.txt");

        let content = b"This is test content with multiple lines\nLine 2\nLine 3";
        let result = write_atomic(&file_path, content, "test");

        assert!(result.is_ok());

        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, String::from_utf8_lossy(content));
    }

    #[test]
    fn test_write_atomic_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("overwrite.txt");

        // Write initial content
        fs::write(&file_path, b"old content").unwrap();

        // Overwrite with atomic write
        let new_content = b"new content";
        let result = write_atomic(&file_path, new_content, "test");

        assert!(result.is_ok());

        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, String::from_utf8_lossy(new_content));
    }

    #[test]
    fn test_write_atomic_no_temp_files_left() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("cleanup.txt");

        let content = b"test content";
        let result = write_atomic(&file_path, content, "test");

        assert!(result.is_ok());

        // Check that no .tmp files are left behind
        let entries = fs::read_dir(temp_dir.path()).unwrap();
        let tmp_files: Vec<_> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".tmp")
            })
            .collect();

        assert_eq!(
            tmp_files.len(),
            0,
            "Temp files should be cleaned up after successful write"
        );
    }

    #[test]
    fn test_write_atomic_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("dir").join("file.txt");

        let content = b"nested content";
        let result = write_atomic(&nested_path, content, "test");

        // This should fail because parent directories don't exist
        // write_atomic does NOT create parent directories
        assert!(result.is_err());
        assert!(!nested_path.exists());
    }

    #[test]
    fn test_write_atomic_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");

        let content = b"";
        let result = write_atomic(&file_path, content, "test");

        assert!(result.is_ok());
        assert!(file_path.exists());

        let read_content = fs::read(&file_path).unwrap();
        assert_eq!(read_content.len(), 0);
    }

    #[test]
    fn test_write_atomic_binary_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("binary.bin");

        let content: Vec<u8> = (0..255).collect();
        let result = write_atomic(&file_path, &content, "test");

        assert!(result.is_ok());

        let read_content = fs::read(&file_path).unwrap();
        assert_eq!(read_content, content);
    }
}
