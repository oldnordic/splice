//! Module path to file path mapping.
//!
//! Builds an index from fully qualified names to file paths, enabling
//! resolution of imported symbols to their source files.

use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;

/// Maps module paths to file paths
///
/// This index enables resolution of imported symbols by mapping
/// module paths (e.g., "crate::completion::types") to their
/// corresponding file paths (e.g., "/path/to/completion/types.rs").
///
/// # Example
///
/// ```rust
/// use splice::completion::module_index::ModulePathIndex;
///
/// let index = ModulePathIndex::build(&db_path)?;
/// if let Some(file_path) = index.resolve("splice::completion::types") {
///     println!("Module found at: {}", file_path.display());
/// }
/// ```
pub struct ModulePathIndex {
    /// Module path → file path (e.g., "splice::completion::types" → "/path/to/types.rs")
    module_to_file: HashMap<String, PathBuf>,
}

impl ModulePathIndex {
    /// Build index from Magellan database
    ///
    /// Queries the database for Symbol entities and extracts module paths
    /// from their `display_fqn` field. Each fully qualified name like
    /// "splice::completion::types::CompletionRequest" is parsed to extract
    /// the module path "splice::completion::types", which is then mapped
    /// to the file path where the symbol is defined.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the Magellen SQLite database
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the built `ModulePathIndex` or an error
    ///
    /// # Example
    ///
    /// ```rust
    /// let index = ModulePathIndex::build(&PathBuf::from(".magellan/splice.db"))?;
    /// ```
    pub fn build(db_path: &PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Query Symbol entities with display_fqn field
        let query = r#"
            SELECT file_path, data
            FROM graph_entities
            WHERE kind = 'Symbol'
              AND data LIKE '%display_fqn%'
            LIMIT 10000
        "#;

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut module_to_file = HashMap::new();

        // Process each Symbol entity
        for row_result in rows {
            let (file_path, data_str) = row_result?;

            // Parse JSON data to extract display_fqn
            if let Ok(data) = serde_json::from_str::<JsonValue>(&data_str) {
                if let Some(display_fqn) = data.get("display_fqn").and_then(|v| v.as_str()) {
                    // Parse display_fqn to extract module path
                    // "splice::completion::types::CompletionRequest" → "splice::completion::types"
                    let parts: Vec<&str> = display_fqn.split("::").collect();

                    // Only process if we have at least module::symbol structure
                    if parts.len() > 1 {
                        // Extract module path by removing the last element (symbol name)
                        let module_path = parts[..parts.len() - 1].join("::");

                        // Map module path to file path
                        // Use absolute path from database
                        module_to_file.insert(module_path, PathBuf::from(&file_path));
                    }
                }
            }
        }

        Ok(Self { module_to_file })
    }

    /// Resolve module path to file path
    ///
    /// Given a module path like "splice::completion::types", returns the
    /// file path where that module is defined (e.g., "/path/to/completion/types.rs").
    ///
    /// # Arguments
    ///
    /// * `module_path` - Module path to resolve (e.g., "crate::completion::types")
    ///
    /// # Returns
    ///
    /// Returns `Some(&PathBuf)` if the module path is found in the index,
    /// or `None` if the module path is unknown
    ///
    /// # Example
    ///
    /// ```rust
    /// let index = ModulePathIndex::build(&db_path)?;
    ///
    /// if let Some(file_path) = index.resolve("splice::completion::types") {
    ///     println!("Found module at: {}", file_path.display());
    /// } else {
    ///     println!("Module not found in index");
    /// }
    /// ```
    pub fn resolve(&self, module_path: &str) -> Option<&PathBuf> {
        self.module_to_file.get(module_path)
    }

    /// Get the number of indexed modules
    ///
    /// Returns the total number of unique module paths in the index.
    /// Useful for debugging and validation.
    ///
    /// # Example
    ///
    /// ```rust
    /// let index = ModulePathIndex::build(&db_path)?;
    /// println!("Indexed {} modules", index.len());
    /// ```
    pub fn len(&self) -> usize {
        self.module_to_file.len()
    }

    /// Check if the index is empty
    ///
    /// Returns `true` if no modules are indexed, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.module_to_file.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_index() {
        // This test will be implemented once we have a test database setup
        // For now, we just verify the code compiles
        let db_path = PathBuf::from(".magellan/splice.db");

        if !db_path.exists() {
            return; // Skip test if database not available
        }

        let result = ModulePathIndex::build(&db_path);
        assert!(result.is_ok(), "Failed to build index: {:?}", result.err());

        let index = result.unwrap();
        println!("Indexed {} modules", index.len());

        // Should have indexed some modules from the codebase
        if !index.is_empty() {
            // Try to resolve a known module path
            // This will depend on what's actually in the database
            if let Some(file_path) = index.resolve("splice::completion::types") {
                println!("Resolved module to: {}", file_path.display());
                assert!(file_path.exists(), "File path should exist");
            }
        }
    }

    #[test]
    fn test_resolve_unknown_module() {
        let index = ModulePathIndex {
            module_to_file: HashMap::new(),
        };

        // Resolving unknown module should return None
        assert!(index.resolve("unknown::module").is_none());
    }

    #[test]
    fn test_len_and_is_empty() {
        let empty_index = ModulePathIndex {
            module_to_file: HashMap::new(),
        };

        assert_eq!(empty_index.len(), 0);
        assert!(empty_index.is_empty());

        let mut populated = HashMap::new();
        populated.insert("test::module".to_string(), PathBuf::from("/test/path.rs"));

        let populated_index = ModulePathIndex {
            module_to_file: populated,
        };

        assert_eq!(populated_index.len(), 1);
        assert!(!populated_index.is_empty());
    }
}
