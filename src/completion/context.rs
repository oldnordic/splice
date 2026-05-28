use crate::completion::imports::{ImportEntity, ImportResolver};
use crate::completion::tokenizer;
use crate::completion::types::{Symbol, SymbolKind};
use crate::graph::MagellanIntegration;
use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Context around cursor position
#[derive(Debug, Clone)]
/// Context around cursor position.
pub struct CompletionContext {
    /// Absolute path to the file being edited.
    pub file_path: PathBuf,
    /// 1-based line number of the cursor.
    pub cursor_line: usize,
    /// 1-based column number of the cursor.
    pub cursor_col: usize,
    /// Function enclosing the cursor, if any.
    pub enclosing_function: Option<Symbol>,
    /// Module enclosing the cursor, if any.
    pub enclosing_module: Option<Symbol>,
    /// Symbols visible from the cursor position.
    pub visible_symbols: Vec<Symbol>,
    /// Token currently being typed at the cursor, if any.
    pub current_token: Option<String>,
}

impl CompletionContext {
    /// Normalize file path to absolute path for database queries
    /// Converts relative paths (e.g., "src/file.rs") to absolute paths
    fn normalize_path(file_path: &Path) -> PathBuf {
        if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            // Convert relative path to absolute by prepending current directory
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(file_path)
                .canonicalize()
                .unwrap_or_else(|_| file_path.to_path_buf())
        }
    }

    /// Analyze code context at cursor position
    pub fn analyze(
        file_path: &Path,
        line: usize,
        column: usize,
        magellan: &Arc<MagellanIntegration>,
    ) -> Result<Self> {
        // Normalize relative paths to absolute for database queries
        let normalized_path = Self::normalize_path(file_path);

        // Find enclosing function
        let enclosing_function = Self::find_enclosing_function(&normalized_path, line, magellan)?;

        // Find enclosing module
        let enclosing_module = Self::find_enclosing_module(&normalized_path, magellan)?;

        // Get local symbols (in current file)
        let local_symbols =
            Self::get_visible_symbols(&normalized_path, &enclosing_module, magellan)?;

        // Get imported symbols (from other files)
        let imported_symbols =
            Self::get_imported_symbols(&normalized_path, magellan).unwrap_or_default(); // Graceful degradation if import resolution fails

        // Merge local and imported symbols
        let mut visible_symbols = local_symbols;
        visible_symbols.extend(imported_symbols);

        // Extract current token being typed
        let current_token = tokenizer::extract_current_token(&normalized_path, line, column);

        Ok(Self {
            file_path: normalized_path,
            cursor_line: line,
            cursor_col: column,
            enclosing_function,
            enclosing_module,
            visible_symbols,
            current_token,
        })
    }

    fn find_enclosing_function(
        file_path: &Path,
        line: usize,
        magellan: &Arc<MagellanIntegration>,
    ) -> Result<Option<Symbol>> {
        // Query for symbols in the same file
        let db_query = r#"
            SELECT id, name, kind, file_path, data
            FROM graph_entities
            WHERE file_path = ?1
              AND kind = 'Symbol'
            LIMIT 1000
        "#;

        let conn = Connection::open(magellan.db_path())
            .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?;

        let mut stmt = conn.prepare(db_query)?;
        let rows = stmt.query_map([&file_path.to_string_lossy()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        // Find the function that encloses the cursor position
        let mut best_match: Option<Symbol> = None;

        for row_result in rows {
            let (id, name, _kind, path, data_str) = row_result?;
            let id = id.to_string(); // Convert i64 to String

            // Parse the data JSON
            if let Ok(data) = serde_json::from_str::<JsonValue>(&data_str) {
                if let Some(start_line) = data["start_line"].as_u64() {
                    if let Some(end_line) = data["end_line"].as_u64() {
                        // Check if cursor is within this symbol's span
                        if (line as u64) >= start_line && (line as u64) <= end_line {
                            // Check if it's a function
                            if let Some(symbol_kind) = data["kind"].as_str() {
                                if symbol_kind == "Function" || symbol_kind == "Method" {
                                    let symbol = Symbol {
                                        id,
                                        name,
                                        kind: Self::parse_symbol_kind(symbol_kind),
                                        path,
                                        line: start_line as usize,
                                        column: data["start_col"].as_u64().unwrap_or(0) as usize,
                                    };

                                    // Keep the most specific (smallest) enclosing function
                                    if best_match.is_none()
                                        || symbol.line > best_match.as_ref().expect("invariant: checked is_none above").line
                                    {
                                        best_match = Some(symbol);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(best_match)
    }

    fn find_enclosing_module(
        _file_path: &Path,
        _magellan: &Arc<MagellanIntegration>,
    ) -> Result<Option<Symbol>> {
        // Extract module path from file_path
        // e.g., src/patch/engine.rs -> patch::engine

        // For now, return None - will be implemented with more sophisticated logic
        Ok(None)
    }

    fn get_visible_symbols(
        file_path: &Path,
        _module: &Option<Symbol>,
        magellan: &Arc<MagellanIntegration>,
    ) -> Result<Vec<Symbol>> {
        // Query for symbols in the same file (simplified for MVP)
        let db_query = r#"
            SELECT id, name, kind, file_path, data
            FROM graph_entities
            WHERE file_path = ?1
              AND kind = 'Symbol'
            ORDER BY name
            LIMIT 100
        "#;

        let conn = Connection::open(magellan.db_path())
            .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?;

        let mut stmt = conn.prepare(db_query)?;
        let rows = stmt.query_map([&file_path.to_string_lossy()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut symbols = Vec::new();
        for row_result in rows {
            let (id, name, _kind, path, data_str) = row_result?;
            let id = id.to_string(); // Convert i64 to String

            // Parse the data JSON to get symbol kind and location
            if let Ok(data) = serde_json::from_str::<JsonValue>(&data_str) {
                let kind_str = data["kind"].as_str().unwrap_or("Unknown");
                let start_line = data["start_line"].as_u64().unwrap_or(0) as usize;
                let start_col = data["start_col"].as_u64().unwrap_or(0) as usize;

                let symbol = Symbol {
                    id,
                    name,
                    kind: Self::parse_symbol_kind(kind_str),
                    path,
                    line: start_line,
                    column: start_col,
                };

                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    fn get_imported_symbols(
        file_path: &Path,
        magellan: &Arc<MagellanIntegration>,
    ) -> Result<Vec<Symbol>> {
        let resolver = ImportResolver::new(magellan.db_path());

        // Get all imports for this file
        let imports = resolver.get_file_imports(file_path)?;

        let mut imported_symbols = Vec::new();

        // For each import, get symbols from target file
        for import in imports {
            // Resolve import path to target file
            // For MVP: Use direct file path matching
            // TODO: Integrate ModulePathIndex for proper resolution

            // Build potential target file paths
            let target_files = Self::resolve_import_targets(&import, file_path);

            for target_file in target_files {
                // Get public symbols from target file
                if let Ok(symbols) = Self::get_public_symbols(&target_file, magellan) {
                    let filtered = if import.is_glob {
                        symbols // All public symbols
                    } else {
                        // Filter by imported_names
                        symbols
                            .into_iter()
                            .filter(|s| import.imported_names.contains(&s.name))
                            .collect()
                    };

                    imported_symbols.extend(filtered);
                }
            }
        }

        Ok(imported_symbols)
    }

    fn resolve_import_targets(import: &ImportEntity, current_file: &Path) -> Vec<PathBuf> {
        // For MVP: Try sibling files and common patterns
        // TODO: Use ModulePathIndex for proper resolution

        let mut targets = Vec::new();

        // Try same directory
        if let Some(parent) = current_file.parent() {
            for segment in &import.import_path {
                let target = parent.join(segment).with_extension("rs");
                if target.exists() {
                    targets.push(target);
                }
            }

            // Try mod.rs
            let mod_path = parent.join("mod.rs");
            if mod_path.exists() {
                targets.push(mod_path);
            }
        }

        targets
    }

    fn get_public_symbols(
        file_path: &Path,
        magellan: &Arc<MagellanIntegration>,
    ) -> Result<Vec<Symbol>> {
        let db_query = r#"
            SELECT id, name, kind, file_path, data
            FROM graph_entities
            WHERE file_path = ?1
              AND kind = 'Symbol'
            LIMIT 100
        "#;

        let conn = Connection::open(magellan.db_path())
            .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?;

        let mut stmt = conn.prepare(db_query)?;
        let rows = stmt.query_map([file_path.to_string_lossy()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut symbols = Vec::new();
        for row_result in rows {
            let (id, name, _kind, path, data_str) = row_result?;
            let id = id.to_string();

            // Parse data to check if symbol is public
            if let Ok(data) = serde_json::from_str::<JsonValue>(&data_str) {
                // Symbol is public if it has canonical_fqn or visibility=public
                let is_public = data.get("canonical_fqn").is_some()
                    || data.get("visibility").and_then(|v| v.as_str()) == Some("public");

                if is_public {
                    let kind_str = data
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let start_line =
                        data.get("start_line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let start_col =
                        data.get("start_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                    let symbol = Symbol {
                        id: id.clone(),
                        name,
                        kind: Self::parse_symbol_kind(kind_str),
                        path,
                        line: start_line,
                        column: start_col,
                    };

                    symbols.push(symbol);
                }
            }
        }

        Ok(symbols)
    }

    fn parse_symbol_kind(kind_str: &str) -> SymbolKind {
        match kind_str {
            "Function" | "Method" => SymbolKind::Function,
            "Struct" => SymbolKind::Struct,
            "Enum" => SymbolKind::Enum,
            "Trait" => SymbolKind::Trait,
            "Module" => SymbolKind::Module,
            "Constant" => SymbolKind::Constant,
            "TypeAlias" => SymbolKind::TypeAlias,
            "Constructor" => SymbolKind::Constructor,
            "Impl" => SymbolKind::Impl,
            _ => SymbolKind::Variable,
        }
    }
}
