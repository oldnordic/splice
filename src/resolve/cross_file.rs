//! Cross-file symbol resolution.
//!
//! This module provides symbol resolution across file boundaries using import data.
//! Handles local symbols, explicit imports, glob imports, renamed imports, and super/self references.

use crate::ingest::imports::{ImportFact, ImportKind};
use crate::resolve::module_resolver::{resolve_module_path, ModulePathIndex};
use std::collections::HashMap;

/// A resolved symbol with location information.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSymbol {
    /// Symbol name (original name, not the local alias if renamed)
    pub name: String,
    /// File path where symbol is defined.
    pub file_path: String,
    /// Symbol kind (function, struct, etc.).
    pub kind: String,
}

/// Symbol registry mapping (file_path, symbol_name) → (original_name, kind).
///
/// Used by CrossFileResolver to track symbols defined in each file.
type SymbolRegistry = HashMap<String, HashMap<String, (String, String)>>;

/// Cross-file symbol resolver using import data.
///
/// Tracks local symbols and imports to resolve symbols across files.
pub struct CrossFileResolver<'a> {
    /// Module path index for resolving import paths to files.
    index: &'a ModulePathIndex,

    /// Local symbols: (file_path, symbol_name) → (original_name, kind)
    symbols: SymbolRegistry,

    /// Imports by file: file_path → Vec<ImportFact>
    imports: HashMap<String, Vec<ImportFact>>,
}

impl<'a> CrossFileResolver<'a> {
    /// Create a new cross-file resolver.
    pub fn new(index: &'a ModulePathIndex) -> Self {
        Self {
            index,
            symbols: HashMap::new(),
            imports: HashMap::new(),
        }
    }

    /// Register a local symbol in a file.
    ///
    /// # Arguments
    /// * `file_path` - File containing the symbol
    /// * `name` - Symbol name
    /// * `kind` - Symbol kind (function, struct, etc.)
    pub fn add_local_symbol(&mut self, file_path: &str, name: &str, kind: &str) {
        let entry = self.symbols.entry(file_path.to_string()).or_default();
        entry.insert(name.to_string(), (name.to_string(), kind.to_string()));
    }

    /// Register an import for a file.
    ///
    /// # Arguments
    /// * `import` - Import fact to register
    pub fn add_import(&mut self, import: ImportFact) {
        let file_path = import.file_path.to_str().unwrap_or("").to_string();
        self.imports.entry(file_path).or_default().push(import);
    }

    /// Resolve a symbol from the perspective of a given file.
    ///
    /// Resolution strategy:
    /// 1. Check local symbols first (highest priority)
    /// 2. Check explicit imports
    /// 3. Check glob imports
    ///
    /// # Arguments
    /// * `current_file` - File path from which we're resolving
    /// * `identifier` - Symbol name to resolve (local name, not original)
    ///
    /// # Returns
    /// * `Some(ResolvedSymbol)` - If symbol is found
    /// * `None` - If symbol cannot be resolved
    pub fn resolve_symbol(&self, current_file: &str, identifier: &str) -> Option<ResolvedSymbol> {
        // Step 1: Check local symbols first (highest priority)
        if let Some(symbol) = self.find_local_symbol(current_file, identifier) {
            return Some(symbol);
        }

        // Step 2: Check explicit imports
        if let Some(symbol) = self.find_in_explicit_imports(current_file, identifier) {
            return Some(symbol);
        }

        // Step 3: Check glob imports
        if let Some(symbol) = self.find_in_glob_imports(current_file, identifier) {
            return Some(symbol);
        }

        None
    }

    /// Find a symbol in the local file.
    fn find_local_symbol(&self, file_path: &str, identifier: &str) -> Option<ResolvedSymbol> {
        self.symbols
            .get(file_path)
            .and_then(|symbols| symbols.get(identifier))
            .map(|(name, kind)| ResolvedSymbol {
                name: name.clone(),
                file_path: file_path.to_string(),
                kind: kind.clone(),
            })
    }

    /// Find a symbol through explicit (non-glob) imports.
    fn find_in_explicit_imports(
        &self,
        current_file: &str,
        identifier: &str,
    ) -> Option<ResolvedSymbol> {
        let imports = self.imports.get(current_file)?;

        for import in imports {
            // Skip glob imports
            if import.is_glob {
                continue;
            }

            // Check if identifier is in imported_names
            // For renamed imports, imported_names contains the local alias
            let import_index = import.imported_names.iter().position(|n| n == identifier);

            if let Some(idx) = import_index {
                // Resolve the import path to a file
                let target_file = self.resolve_import_path(current_file, import)?;
                let local_name = &import.imported_names[idx];

                // For renamed imports, we need to find the original name in target file
                // For now, use the local_name if found, otherwise search by kind
                return self
                    .find_symbol_in_file(&target_file, local_name)
                    .or_else(|| self.find_first_symbol_in_file(&target_file));
            }
        }

        None
    }

    /// Find a symbol through glob imports.
    fn find_in_glob_imports(&self, current_file: &str, identifier: &str) -> Option<ResolvedSymbol> {
        let imports = self.imports.get(current_file)?;

        for import in imports {
            if !import.is_glob {
                continue;
            }

            // Resolve the import path to a file
            let target_file = self.resolve_import_path(current_file, import)?;

            // Check if symbol exists in target file
            if let Some(symbol) = self.find_symbol_in_file(&target_file, identifier) {
                return Some(symbol);
            }
        }

        None
    }

    /// Resolve an import path to a target file path.
    ///
    /// Handles:
    /// - Absolute paths: crate::foo::bar
    /// - Super references: super, super::super
    /// - Self references: self
    /// - Python imports (TODO: implement Python-specific resolution)
    /// - C/C++ includes (TODO: implement C/C++ specific resolution)
    fn resolve_import_path(&self, current_file: &str, import: &ImportFact) -> Option<String> {
        let module_path_str = import.path.join("::");

        match import.import_kind {
            ImportKind::UseCrate | ImportKind::PlainUse => {
                // Resolve using the module path index
                self.index.resolve(&module_path_str)
            }
            ImportKind::UseSuper => {
                // Resolve super relative to current file
                resolve_module_path(self.index, current_file, &module_path_str)
            }
            ImportKind::UseSelf => {
                // Self references resolve to current file
                Some(current_file.to_string())
            }
            ImportKind::ExternCrate => {
                // External crates not supported yet
                None
            }
            // Python imports - not yet implemented for cross-file resolution
            ImportKind::PythonImport
            | ImportKind::PythonFrom
            | ImportKind::PythonFromRelative
            | ImportKind::PythonFromParent
            | ImportKind::PythonFromAncestor => None,
            // C/C++ includes - not yet implemented for cross-file resolution
            ImportKind::CppSystemInclude | ImportKind::CppLocalInclude => None,
            // JavaScript/TypeScript imports - not yet implemented for cross-file resolution
            ImportKind::JsImport
            | ImportKind::JsDefaultImport
            | ImportKind::JsNamespaceImport
            | ImportKind::JsSideEffectImport
            | ImportKind::JsRequire => None,
            // Java imports - not yet implemented for cross-file resolution
            ImportKind::JavaImport | ImportKind::JavaStaticImport => None,
            // TypeScript type-only imports - not yet implemented for cross-file resolution
            ImportKind::TsTypeImport | ImportKind::TsTypeDefaultImport => None,
        }
    }

    /// Find a specific symbol in a file.
    fn find_symbol_in_file(&self, file_path: &str, name: &str) -> Option<ResolvedSymbol> {
        self.symbols
            .get(file_path)
            .and_then(|symbols| symbols.get(name))
            .map(|(name, kind)| ResolvedSymbol {
                name: name.clone(),
                file_path: file_path.to_string(),
                kind: kind.clone(),
            })
    }

    /// Find the first symbol in a file (for glob imports where name isn't known).
    fn find_first_symbol_in_file(&self, file_path: &str) -> Option<ResolvedSymbol> {
        self.symbols
            .get(file_path)
            .and_then(|symbols| symbols.values().next())
            .map(|(name, kind)| ResolvedSymbol {
                name: name.clone(),
                file_path: file_path.to_string(),
                kind: kind.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolved_symbol_equality() {
        let s1 = ResolvedSymbol {
            name: "foo".to_string(),
            file_path: "/src/a.rs".to_string(),
            kind: "function".to_string(),
        };
        let s2 = ResolvedSymbol {
            name: "foo".to_string(),
            file_path: "/src/a.rs".to_string(),
            kind: "function".to_string(),
        };
        assert_eq!(s1, s2);
    }
}
