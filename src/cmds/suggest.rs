//! Suggest command handler - intent-based scaffold generation

use std::path::Path;

use serde::Serialize;
use splice::cli::OutputFormat;

/// Execute suggest command with intent-based scaffold generation
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_suggest(
    fn_name: &str,
    desc: &str,
    db: Option<&Path>,
    output: OutputFormat,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    // Auto-discover database if not provided
    let db_path = if let Some(db) = db {
        db.to_path_buf()
    } else {
        splice::graph::discover_db_path(None)?.path
    };

    // Check if database exists
    if !db_path.exists() {
        return Err(splice::SpliceError::IoContext {
            context: format!("Database not found: {}", db_path.display()),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "database not found"),
        });
    }

    // Query the database for similar symbols
    let mut magellan = splice::graph::MagellanIntegration::open(&db_path)?;

    // Find symbols matching the function name pattern
    let pattern = format!("{}*", fn_name);
    let matching_symbols = magellan.find_symbol_by_name(&pattern, false)?;

    // Find symbols that might be callers (based on description keywords)
    let keywords = extract_keywords(desc);
    let caller_candidates = find_caller_candidates(&mut magellan, &keywords);

    // Generate scaffold
    let scaffold = generate_scaffold(fn_name, desc, &matching_symbols, &caller_candidates);

    // Format output based on requested format
    match output {
        splice::cli::OutputFormat::Json => {
            let json_data = serde_json::json!({
                "fn_name": fn_name,
                "signature": scaffold.signature,
                "imports_needed": scaffold.imports_needed,
                "similar_existing": scaffold.similar_existing,
                "callers_found": scaffold.callers_found,
                "body_scaffold": scaffold.body_scaffold,
            });

            Ok(splice::cli::CliSuccessPayload {
                status: "success",
                message: format!("Generated scaffold for {}", fn_name),
                data: Some(json_data),
                already_emitted: false,
                has_pending_changes: false,
            })
        }
        splice::cli::OutputFormat::Pretty => {
            let pretty_output = format_pretty_scaffold(&scaffold);
            Ok(splice::cli::CliSuccessPayload {
                status: "success",
                message: format!("Generated scaffold for {}", fn_name),
                data: Some(serde_json::json!({
                    "scaffold": pretty_output,
                    "details": scaffold
                })),
                already_emitted: false,
                has_pending_changes: false,
            })
        }
        splice::cli::OutputFormat::Human => {
            let human_output = format_human_scaffold(&scaffold);
            Ok(splice::cli::CliSuccessPayload {
                status: "success",
                message: format!("Generated scaffold for {}", fn_name),
                data: Some(serde_json::json!({
                    "output": human_output
                })),
                already_emitted: false,
                has_pending_changes: false,
            })
        }
    }
}

/// Scaffold data structure
#[derive(Debug, Clone, Serialize)]
struct Scaffold {
    signature: String,
    imports_needed: Vec<String>,
    similar_existing: Vec<String>,
    callers_found: Vec<String>,
    body_scaffold: String,
}

/// Extract keywords from description
fn extract_keywords(desc: &str) -> Vec<String> {
    // Simple keyword extraction: split on whitespace and filter common words
    let common_words = [
        "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "from", "as", "is", "was", "are", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "should", "could", "may", "might", "can", "must",
    ];

    desc.split_whitespace()
        .map(|word| word.to_lowercase())
        .filter(|word| !common_words.contains(&word.as_str()))
        .collect()
}

/// Find potential caller candidates based on keywords
fn find_caller_candidates(
    magellan: &mut splice::graph::MagellanIntegration,
    keywords: &[String],
) -> Vec<String> {
    let mut candidates = Vec::new();

    // For each keyword, try to find symbols that might call it
    for keyword in keywords {
        if let Ok(symbols) = magellan.find_symbol_by_name(keyword, false) {
            for symbol in symbols {
                let location = format!("{}:{}", symbol.file_path, symbol.byte_start);
                if !candidates.contains(&location) {
                    candidates.push(location);
                }
            }
        }
    }

    candidates
}

/// Generate scaffold from context
fn generate_scaffold(
    fn_name: &str,
    desc: &str,
    matching_symbols: &[splice::graph::magellan_integration::SymbolInfo],
    caller_candidates: &[String],
) -> Scaffold {
    // Extract types from matching symbols
    let imports_needed = extract_imports_from_symbols(matching_symbols);

    // Find similar functions
    let similar_existing: Vec<String> = matching_symbols
        .iter()
        .take(5) // Limit to 5 most similar
        .map(|sym| sym.name.clone())
        .collect();

    // Infer function signature from description
    let signature = infer_signature(fn_name, desc, &imports_needed);

    // Generate body scaffold
    let body_scaffold = format!("// TODO: Implement based on description: {}", desc);

    Scaffold {
        signature,
        imports_needed,
        similar_existing,
        callers_found: caller_candidates.to_vec(),
        body_scaffold,
    }
}

/// Extract import paths from symbols
fn extract_imports_from_symbols(
    symbols: &[splice::graph::magellan_integration::SymbolInfo],
) -> Vec<String> {
    let mut imports = std::collections::HashSet::new();

    for symbol in symbols {
        // Extract module path from file path
        let module = &symbol.file_path;
        if let Some(import_path) = module.strip_prefix("/home/feanor/Projects/splice/") {
            // Convert file path to Rust import path
            let rust_path = import_path.replace(".rs", "").replace("/", "::");
            imports.insert(rust_path);
        }
    }

    imports.into_iter().collect()
}

/// Infer function signature from description and context
fn infer_signature(fn_name: &str, desc: &str, _imports: &[String]) -> String {
    // Simple heuristic: if description mentions "parse" or "read", return Result
    let returns_result = desc.to_lowercase().contains("parse")
        || desc.to_lowercase().contains("read")
        || desc.to_lowercase().contains("load");

    if returns_result {
        format!("fn {}() -> Result<(), Error>", fn_name)
    } else {
        format!("fn {}()", fn_name)
    }
}

/// Format scaffold as human-readable output
fn format_human_scaffold(scaffold: &Scaffold) -> String {
    let mut output = String::new();

    output.push_str(&format!("{}\n", scaffold.signature));
    output.push('\n');

    if !scaffold.imports_needed.is_empty() {
        output.push_str("Imports needed:\n");
        for import in &scaffold.imports_needed {
            output.push_str(&format!("  use {};\n", import));
        }
        output.push('\n');
    }

    if !scaffold.similar_existing.is_empty() {
        output.push_str("Similar existing functions:\n");
        for sim in &scaffold.similar_existing {
            output.push_str(&format!("  - {}\n", sim));
        }
        output.push('\n');
    }

    if !scaffold.callers_found.is_empty() {
        output.push_str("Potential callers:\n");
        for caller in &scaffold.callers_found {
            output.push_str(&format!("  - {}\n", caller));
        }
        output.push('\n');
    }

    output.push_str("Body scaffold:\n");
    output.push_str(&scaffold.body_scaffold);

    output
}

/// Format scaffold as pretty output
fn format_pretty_scaffold(scaffold: &Scaffold) -> String {
    format_human_scaffold(scaffold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let keywords = extract_keywords("Parse configuration file and return AppConfig");
        assert!(keywords.contains(&"parse".to_string()));
        assert!(keywords.contains(&"configuration".to_string()));
        assert!(keywords.contains(&"file".to_string()));
        assert!(keywords.contains(&"return".to_string()));
        assert!(keywords.contains(&"appconfig".to_string()));
    }

    #[test]
    fn test_extract_keywords_filters_common_words() {
        let keywords = extract_keywords("The function reads the file and returns the result");
        assert!(!keywords.contains(&"the".to_string()));
        assert!(!keywords.contains(&"and".to_string()));
        assert!(keywords.contains(&"function".to_string()));
        assert!(keywords.contains(&"reads".to_string()));
    }

    #[test]
    fn test_infer_signature_with_result() {
        let signature = infer_signature("parse_config", "Parse TOML config file", &[]);
        assert!(signature.contains("Result"));
    }

    #[test]
    fn test_infer_signature_simple() {
        let signature = infer_signature("hello", "Say hello world", &[]);
        assert!(!signature.contains("Result"));
    }

    #[test]
    fn test_format_human_scaffold() {
        let scaffold = Scaffold {
            signature: "fn test() -> Result<(), Error>".to_string(),
            imports_needed: vec!["std::fs".to_string()],
            similar_existing: vec!["test_v1".to_string()],
            callers_found: vec!["main.rs:42".to_string()],
            body_scaffold: "// TODO: Implement".to_string(),
        };

        let output = format_human_scaffold(&scaffold);
        assert!(output.contains("fn test()"));
        assert!(output.contains("Imports needed"));
        assert!(output.contains("Similar existing"));
        assert!(output.contains("Potential callers"));
        assert!(output.contains("Body scaffold"));
    }
}
