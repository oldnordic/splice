//! Import resolution for cross-file completion.
//!
//! Queries Magellan database for Import entities and resolves them to target files.

use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value as JsonValue;
use std::path::PathBuf;

/// Import entity from database
#[derive(Debug, Clone)]
pub struct ImportEntity {
    pub id: String,
    pub file_path: String,
    pub import_kind: String,
    pub import_path: Vec<String>,
    pub imported_names: Vec<String>,
    pub is_glob: bool,
    pub is_reexport: bool,
}

/// Import resolver using Magellan database
pub struct ImportResolver {
    db_path: PathBuf,
}

impl ImportResolver {
    /// Create new import resolver
    pub fn new(db_path: &PathBuf) -> Self {
        Self {
            db_path: db_path.clone(),
        }
    }

    /// Get all imports for a file from database
    pub fn get_file_imports(&self, file_path: &PathBuf) -> Result<Vec<ImportEntity>> {
        let conn = Connection::open(&self.db_path)?;

        let query = r#"
            SELECT ge.id, ge.file_path, ge.data
            FROM graph_entities ge
            JOIN graph_edges e ON ge.id = e.to_id
            WHERE e.from_id = (
                SELECT id FROM graph_entities
                WHERE file_path = ?1 AND kind = 'File'
                LIMIT 1
            )
            AND e.edge_type = 'IMPORTS'
            AND ge.kind = 'Import'
        "#;

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map(
            [file_path.to_string_lossy().as_ref()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            }
        )?;

        let mut imports = Vec::new();
        for row_result in rows {
            let (id, file_path, data_str) = row_result?;
            let id = id.to_string();

            if let Ok(data) = serde_json::from_str::<JsonValue>(&data_str) {
                let import_kind = data.get("import_kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("plain_use")
                    .to_string();

                let import_path = data.get("import_path")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();

                let imported_names = data.get("imported_names")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();

                let is_glob = data.get("is_glob")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let is_reexport = data.get("is_reexport")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                imports.push(ImportEntity {
                    id,
                    file_path,
                    import_kind,
                    import_path,
                    imported_names,
                    is_glob,
                    is_reexport,
                });
            }
        }

        Ok(imports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_imports() {
        // Test will be implemented after database setup
        // For now, just ensure it compiles
    }
}
