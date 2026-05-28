//! DOT graph generation for impact and reference visualization.

use crate::cli::ReachabilityDirection;
use crate::error::{Result, SpliceError};
use std::path::Path;

use super::types::*;
use super::MagellanIntegration;

impl MagellanIntegration {
    /// Generate DOT graph output for impact visualization.
    ///
    /// # Arguments
    /// * `symbol_id` - Entity ID of the root symbol
    /// * `direction` - Direction of traversal (Forward/Reverse/Both)
    /// * `config` - Configuration for DOT output
    ///
    /// # Returns
    /// DOT format string suitable for Graphviz rendering
    pub fn generate_impact_dot(
        &mut self,
        symbol_id: &str,
        direction: &ReachabilityDirection,
        config: &ImpactDotConfig,
    ) -> Result<String> {
        // Parse symbol_id as "file_path:symbol_name" format
        let (file_path, symbol_name) = symbol_id.split_once(':').ok_or_else(|| {
            SpliceError::Other(format!(
                "Invalid symbol_id format: '{}'. Expected 'file_path:symbol_name'",
                symbol_id
            ))
        })?;

        let file_path_obj = Path::new(file_path);

        // Collect reachable symbols based on direction
        let (forward_symbols, reverse_symbols) = match direction {
            ReachabilityDirection::Forward => {
                let symbols = self.reachable_symbols(
                    file_path_obj,
                    symbol_name,
                    config.max_depth.unwrap_or(10),
                )?;
                (symbols, Vec::new())
            }
            ReachabilityDirection::Reverse => {
                let symbols = self.reverse_reachable_symbols(
                    file_path_obj,
                    symbol_name,
                    config.max_depth.unwrap_or(10),
                )?;
                (Vec::new(), symbols)
            }
            ReachabilityDirection::Both => {
                let max_depth = config.max_depth.unwrap_or(10);
                let forward = self.reachable_symbols(file_path_obj, symbol_name, max_depth)?;
                let reverse =
                    self.reverse_reachable_symbols(file_path_obj, symbol_name, max_depth)?;
                (forward, reverse)
            }
        };

        // Generate DOT output
        let mut dot = String::from("digraph Impact {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box, style=rounded];\n\n");

        // Track all edges to avoid duplicates
        let mut edges = std::collections::HashSet::new();
        let mut nodes = std::collections::HashSet::new();

        // Add root node
        let root_label = if config.show_symbol_kinds {
            format!(
                "{} ({})",
                symbol_name,
                _get_root_kind(self, file_path, symbol_name)
            )
        } else {
            symbol_name.to_string()
        };

        let root_attrs = if config
            .highlight_symbol
            .as_ref()
            .is_some_and(|h| *h == symbol_name)
        {
            " [style=filled, fillcolor=lightblue]"
        } else {
            ""
        };

        dot.push_str(&format!(
            "  \"{}\"{} [label=\"{}\"];\n",
            _sanitize_id(symbol_id),
            root_attrs,
            _escape_label(&root_label)
        ));
        nodes.insert(symbol_id.to_string());

        // Add forward edges (callees)
        for reachable in &forward_symbols {
            let caller_id = format!("{}:{}", reachable.symbol.file_path, reachable.symbol.name);
            let label = if config.show_symbol_kinds {
                format!("{} ({})", reachable.symbol.name, reachable.symbol.kind)
            } else {
                reachable.symbol.name.clone()
            };

            // Add node if not already added
            if nodes.insert(caller_id.clone()) {
                let attrs = if config
                    .highlight_symbol
                    .as_ref()
                    .is_some_and(|h| *h == reachable.symbol.name)
                {
                    " [style=filled, fillcolor=lightblue]"
                } else {
                    ""
                };
                dot.push_str(&format!(
                    "  \"{}\"{} [label=\"{}\"];\n",
                    _sanitize_id(&caller_id),
                    attrs,
                    _escape_label(&label)
                ));
            }

            // Add edge: root -> callee
            let edge = (symbol_id.to_string(), caller_id.clone());
            if edges.insert(edge) {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\";\n",
                    _sanitize_id(symbol_id),
                    _sanitize_id(&caller_id)
                ));
            }

            // Add edges along the path
            for i in 0..reachable.path.len() {
                let from = if i == 0 {
                    symbol_id.to_string()
                } else {
                    format!("{}:{}", reachable.symbol.file_path, reachable.path[i - 1])
                };
                let to = format!("{}:{}", reachable.symbol.file_path, reachable.path[i]);
                let edge = (from.clone(), to.clone());
                if edges.insert(edge) {
                    dot.push_str(&format!(
                        "  \"{}\" -> \"{}\";\n",
                        _sanitize_id(&from),
                        _sanitize_id(&to)
                    ));
                }
            }
        }

        // Add reverse edges (callers)
        for reachable in &reverse_symbols {
            let caller_id = format!("{}:{}", reachable.symbol.file_path, reachable.symbol.name);
            let label = if config.show_symbol_kinds {
                format!("{} ({})", reachable.symbol.name, reachable.symbol.kind)
            } else {
                reachable.symbol.name.clone()
            };

            if nodes.insert(caller_id.clone()) {
                let attrs = if config
                    .highlight_symbol
                    .as_ref()
                    .is_some_and(|h| *h == reachable.symbol.name)
                {
                    " [style=filled, fillcolor=lightblue]"
                } else {
                    ""
                };
                dot.push_str(&format!(
                    "  \"{}\"{} [label=\"{}\"];\n",
                    _sanitize_id(&caller_id),
                    attrs,
                    _escape_label(&label)
                ));
            }

            // Add edge: caller -> root
            let edge = (caller_id.clone(), symbol_id.to_string());
            if edges.insert(edge) {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\";\n",
                    _sanitize_id(&caller_id),
                    _sanitize_id(symbol_id)
                ));
            }

            // Add edges along the path
            for i in 0..reachable.path.len() {
                let from = format!("{}:{}", reachable.symbol.file_path, reachable.path[i]);
                let to = if i == reachable.path.len() - 1 {
                    symbol_id.to_string()
                } else {
                    format!("{}:{}", reachable.symbol.file_path, reachable.path[i + 1])
                };
                let edge = (from.clone(), to.clone());
                if edges.insert(edge) {
                    dot.push_str(&format!(
                        "  \"{}\" -> \"{}\";\n",
                        _sanitize_id(&from),
                        _sanitize_id(&to)
                    ));
                }
            }
        }

        dot.push_str("}\n");
        Ok(dot)
    }

    /// Generate DOT graph output for refs command.
    ///
    /// # Arguments
    /// * `symbol_name` - Symbol name
    /// * `file_path` - Path to file containing the symbol
    /// * `config` - Configuration for DOT output
    ///
    /// # Returns
    /// DOT format string suitable for Graphviz rendering
    pub fn generate_refs_dot(
        &mut self,
        symbol_name: &str,
        file_path: &Path,
        config: &ImpactDotConfig,
    ) -> Result<String> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        let symbol_id = format!("{}:{}", path_str, symbol_name);

        // Get callers and callees
        let callers = self
            .inner
            .callers_of_symbol(path_str, symbol_name)
            .map_err(|e| SpliceError::Other(format!("Failed to get callers: {}", e)))?;

        let callees = self
            .inner
            .calls_from_symbol(path_str, symbol_name)
            .map_err(|e| SpliceError::Other(format!("Failed to get callees: {}", e)))?;

        // Generate DOT output
        let mut dot = String::from("digraph Impact {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box, style=rounded];\n\n");

        // Add root node
        let root_label = if config.show_symbol_kinds {
            format!(
                "{} ({})",
                symbol_name,
                _get_root_kind(self, path_str, symbol_name)
            )
        } else {
            symbol_name.to_string()
        };

        let root_attrs = if config
            .highlight_symbol
            .as_ref()
            .is_some_and(|h| *h == symbol_name)
        {
            " [style=filled, fillcolor=lightblue]"
        } else {
            ""
        };

        dot.push_str(&format!(
            "  \"{}\"{} [label=\"{}\"];\n",
            _sanitize_id(&symbol_id),
            root_attrs,
            _escape_label(&root_label)
        ));

        // Add caller nodes and edges
        for call in &callers {
            let caller_id = format!("{}:{}", call.file_path.to_string_lossy(), call.caller);
            let label = if config.show_symbol_kinds {
                // Try to get kind info
                let kind = _get_symbol_kind(self, &call.file_path.to_string_lossy(), &call.caller);
                format!("{} ({})", call.caller, kind)
            } else {
                call.caller.clone()
            };

            let attrs = if config
                .highlight_symbol
                .as_ref()
                .is_some_and(|h| *h == call.caller)
            {
                " [style=filled, fillcolor=lightblue]"
            } else {
                ""
            };

            dot.push_str(&format!(
                "  \"{}\"{} [label=\"{}\"];\n",
                _sanitize_id(&caller_id),
                attrs,
                _escape_label(&label)
            ));
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\";\n",
                _sanitize_id(&caller_id),
                _sanitize_id(&symbol_id)
            ));
        }

        // Add callee nodes and edges
        for call in &callees {
            let callee_id = format!("{}:{}", call.file_path.to_string_lossy(), call.callee);
            let label = if config.show_symbol_kinds {
                let kind = _get_symbol_kind(self, &call.file_path.to_string_lossy(), &call.callee);
                format!("{} ({})", call.callee, kind)
            } else {
                call.callee.clone()
            };

            let attrs = if config
                .highlight_symbol
                .as_ref()
                .is_some_and(|h| *h == call.callee)
            {
                " [style=filled, fillcolor=lightblue]"
            } else {
                ""
            };

            dot.push_str(&format!(
                "  \"{}\"{} [label=\"{}\"];\n",
                _sanitize_id(&callee_id),
                attrs,
                _escape_label(&label)
            ));
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\";\n",
                _sanitize_id(&symbol_id),
                _sanitize_id(&callee_id)
            ));
        }

        dot.push_str("}\n");
        Ok(dot)
    }
}

/// Helper to get the kind of a root symbol for DOT labels.
fn _get_root_kind(
    integration: &mut MagellanIntegration,
    file_path: &str,
    symbol_name: &str,
) -> String {
    integration
        .inner
        .symbol_extents(file_path, symbol_name)
        .ok()
        .and_then(|facts| facts.first().map(|(_, fact)| fact.kind_normalized.clone()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Helper to get the kind of a symbol for DOT labels.
fn _get_symbol_kind(
    integration: &mut MagellanIntegration,
    file_path: &str,
    symbol_name: &str,
) -> String {
    integration
        .inner
        .symbol_extents(file_path, symbol_name)
        .ok()
        .and_then(|facts| facts.first().map(|(_, fact)| fact.kind_normalized.clone()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Escape special DOT characters in labels.
fn _escape_label(label: &str) -> String {
    label
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('|', "\\|")
}

/// Sanitize a string for use as a DOT node ID.
fn _sanitize_id(id: &str) -> String {
    // Replace invalid characters with underscores
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
