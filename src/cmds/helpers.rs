//! Shared helpers and types for CLI command handlers.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::SpliceExitCode;

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

pub(crate) fn log_execution_error(operation: &str, err: &splice::SpliceError) {
    eprintln!(
        "Warning: Failed to record execution for '{}': {}",
        operation, err
    );
}

/// Count lines in a byte span within a file.
///
/// This helper function counts how many lines are covered by a byte span.
/// Used to calculate lines_removed for delete operations.
pub(crate) fn count_lines_in_span(file_path: &Path, start: usize, end: usize) -> usize {
    use std::fs;

    if end <= start {
        return 0;
    }

    // Read the file and count newlines in the span
    if let Ok(content) = fs::read_to_string(file_path) {
        let span_end = end.min(content.len());
        let span = &content[start..span_end];
        span.lines().count()
    } else {
        0
    }
}

/// Capture a graph snapshot before a refactoring operation.
pub(crate) fn capture_snapshot(db_path: &Path, operation: &str) -> Result<(), splice::SpliceError> {
    use splice::proof::data_structures::RefactoringProof;
    use splice::proof::generation::{create_metadata, generate_snapshot};
    use std::fs;

    // Generate snapshot from current database state
    let snapshot = generate_snapshot(db_path)?;

    // Create snapshots directory if it doesn't exist
    let snapshots_dir = PathBuf::from(".splice").join("snapshots");
    fs::create_dir_all(&snapshots_dir).map_err(|e| {
        splice::SpliceError::Other(format!("Failed to create snapshots dir: {}", e))
    })?;

    // Create minimal proof metadata
    let metadata = create_metadata(operation, db_path);

    // Create minimal RefactoringProof (only before snapshot, no after)
    let proof = RefactoringProof {
        metadata,
        before: snapshot.clone(),
        after: snapshot, // Use same snapshot for before/after (snapshot-only mode)
        invariants: vec![],
        checksums: None,
    };

    // Write proof to file
    let timestamp = chrono::Utc::now().timestamp();
    let filename = format!("snapshot-{}-{}.json", operation, timestamp);
    let snapshot_path = snapshots_dir.join(&filename);

    let json = serde_json::to_string_pretty(&proof)
        .map_err(|e| splice::SpliceError::Other(format!("Failed to serialize snapshot: {}", e)))?;

    fs::write(&snapshot_path, json)
        .map_err(|e| splice::SpliceError::Other(format!("Failed to write snapshot: {}", e)))?;

    eprintln!("Snapshot captured: {}", snapshot_path.display());
    Ok(())
}

pub(crate) fn parse_date(input: &str) -> Result<i64, splice::SpliceError> {
    use splice::SpliceError;

    // Try Unix timestamp first
    if let Ok(ts) = input.parse::<i64>() {
        return Ok(ts);
    }

    // Try ISO 8601
    chrono::DateTime::parse_from_rfc3339(input)
        .map(|dt| dt.timestamp())
        .map_err(|_| SpliceError::InvalidDateFormat {
            input: input.to_string(),
        })
}

pub(crate) fn install_broken_pipe_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if is_broken_pipe_panic(info) {
            std::process::exit(SpliceExitCode::Success as u8 as i32);
        }
        default_hook(info);
    }));
}

pub(crate) fn is_broken_pipe_panic(info: &std::panic::PanicHookInfo<'_>) -> bool {
    if let Some(err) = info.payload().downcast_ref::<std::io::Error>() {
        return err.kind() == std::io::ErrorKind::BrokenPipe;
    }

    let message = if let Some(msg) = info.payload().downcast_ref::<&str>() {
        *msg
    } else if let Some(msg) = info.payload().downcast_ref::<String>() {
        msg.as_str()
    } else {
        ""
    };

    if message.contains("Broken pipe") || message.contains("failed printing to stdout") {
        return true;
    }

    let info_message = info.to_string();
    info_message.contains("Broken pipe") || info_message.contains("failed printing to stdout")
}

pub(crate) fn confirm_action(prompt: &str) -> Result<bool, splice::SpliceError> {
    use std::io::{self, Write};

    print!("{}", prompt);
    io::stdout()
        .flush()
        .map_err(|e| splice::SpliceError::IoContext {
            context: "Failed to flush stdout".to_string(),
            source: e,
        })?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| splice::SpliceError::IoContext {
            context: "Failed to read user input".to_string(),
            source: e,
        })?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

// ---------------------------------------------------------------------------
// I/O helpers
// ---------------------------------------------------------------------------

pub(crate) fn write_stdout_bytes(bytes: &[u8]) -> Result<(), splice::SpliceError> {
    use std::io::{self, Write};

    let mut stdout = io::stdout();
    if let Err(err) = stdout.write_all(bytes) {
        if err.kind() == io::ErrorKind::BrokenPipe {
            return Err(splice::SpliceError::BrokenPipe);
        }
        return Err(splice::SpliceError::Io {
            path: PathBuf::from("<stdout>"),
            source: err,
        });
    }
    Ok(())
}

pub(crate) fn write_stdout_line(line: &str) -> Result<(), splice::SpliceError> {
    write_stdout_bytes(line.as_bytes())?;
    write_stdout_bytes(b"\n")
}

/// Emit JSON payload for successful CLI responses.
pub(crate) fn emit_success_payload(
    payload: &splice::cli::CliSuccessPayload,
    _json_output: bool,
) -> Result<(), splice::SpliceError> {
    // If payload was already emitted (e.g., --json mode with OperationResult), skip
    if payload.already_emitted {
        return Ok(());
    }

    match serde_json::to_string(payload) {
        Ok(json) => write_stdout_line(&json),
        Err(err) => {
            let fallback = json!({
                "status": "ok",
                "message": payload.message.clone(),
            });
            write_stdout_line(&fallback.to_string())?;
            eprintln!("Serialization warning: {}", err);
            Ok(())
        }
    }
}

/// Emit JSON payload for CLI errors.
pub(crate) fn emit_error_payload(payload: &splice::cli::CliErrorPayload, _json_output: bool) {
    match serde_json::to_string(payload) {
        Ok(json) => eprintln!("{}", json),
        Err(err) => {
            let fallback = json!({
                "status": "error",
                "error": {
                    "kind": "SerializationFailure",
                    "message": err.to_string()
                }
            });
            eprintln!("{}", fallback);
        }
    }
}

pub(crate) fn require_patch_arg<T>(flag: &str, value: Option<T>) -> Result<T, splice::SpliceError> {
    value.ok_or_else(|| {
        splice::SpliceError::Other(format!(
            "{} is required unless --batch <file> is provided",
            flag
        ))
    })
}

// ---------------------------------------------------------------------------
// Symbol helpers
// ---------------------------------------------------------------------------

pub(crate) fn _build_success_payload(
    message: String,
    files: Vec<splice::patch::FilePatchSummary>,
    preview_report: Option<splice::patch::PreviewReport>,
) -> splice::cli::CliSuccessPayload {
    let file_values: Vec<Value> = files
        .iter()
        .map(|summary| {
            json!({
                "file": summary.file.to_string_lossy(),
                "before_hash": summary.before_hash,
                "after_hash": summary.after_hash,
            })
        })
        .collect();

    let mut data = Map::new();
    data.insert("files".to_string(), Value::Array(file_values));

    if let Some(report) = preview_report {
        data.insert(
            "preview_report".to_string(),
            serde_json::to_value(report).expect("preview report should serialize"),
        );
    }

    splice::cli::CliSuccessPayload::with_data(message, Value::Object(data))
}

/// Extract symbols with explicit language (helper function).
pub(crate) fn extract_symbols_with_language(
    path: &Path,
    source: &[u8],
    language: splice::symbol::Language,
) -> Result<Vec<SymbolWrapper>, splice::SpliceError> {
    use splice::ingest::{
        extract_cpp_symbols, extract_java_symbols, extract_javascript_symbols,
        extract_python_symbols, extract_rust_symbols, extract_typescript_symbols,
    };

    match language {
        splice::symbol::Language::Rust => {
            let symbols = extract_rust_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::Rust).collect())
        }
        splice::symbol::Language::Python => {
            let symbols = extract_python_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::Python).collect())
        }
        splice::symbol::Language::C | splice::symbol::Language::Cpp => {
            let symbols = extract_cpp_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::Cpp).collect())
        }
        splice::symbol::Language::Java => {
            let symbols = extract_java_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::Java).collect())
        }
        splice::symbol::Language::JavaScript => {
            let symbols = extract_javascript_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::JavaScript).collect())
        }
        splice::symbol::Language::TypeScript => {
            let symbols = extract_typescript_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::TypeScript).collect())
        }
    }
}

/// Wrapper enum for language-specific symbols that implements Symbol trait.
pub(crate) enum SymbolWrapper {
    Rust(splice::ingest::rust::RustSymbol),
    Python(splice::ingest::python::PythonSymbol),
    Cpp(splice::ingest::cpp::CppSymbol),
    Java(splice::ingest::java::JavaSymbol),
    JavaScript(splice::ingest::javascript::JavaScriptSymbol),
    TypeScript(splice::ingest::typescript::TypeScriptSymbol),
}

impl splice::symbol::Symbol for SymbolWrapper {
    fn name(&self) -> &str {
        match self {
            SymbolWrapper::Rust(s) => s.name(),
            SymbolWrapper::Python(s) => s.name(),
            SymbolWrapper::Cpp(s) => s.name(),
            SymbolWrapper::Java(s) => s.name(),
            SymbolWrapper::JavaScript(s) => s.name(),
            SymbolWrapper::TypeScript(s) => s.name(),
        }
    }

    fn kind(&self) -> &str {
        match self {
            SymbolWrapper::Rust(s) => s.kind(),
            SymbolWrapper::Python(s) => s.kind(),
            SymbolWrapper::Cpp(s) => s.kind(),
            SymbolWrapper::Java(s) => s.kind(),
            SymbolWrapper::JavaScript(s) => s.kind(),
            SymbolWrapper::TypeScript(s) => s.kind(),
        }
    }

    fn byte_start(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.byte_start(),
            SymbolWrapper::Python(s) => s.byte_start(),
            SymbolWrapper::Cpp(s) => s.byte_start(),
            SymbolWrapper::Java(s) => s.byte_start(),
            SymbolWrapper::JavaScript(s) => s.byte_start(),
            SymbolWrapper::TypeScript(s) => s.byte_start(),
        }
    }

    fn byte_end(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.byte_end(),
            SymbolWrapper::Python(s) => s.byte_end(),
            SymbolWrapper::Cpp(s) => s.byte_end(),
            SymbolWrapper::Java(s) => s.byte_end(),
            SymbolWrapper::JavaScript(s) => s.byte_end(),
            SymbolWrapper::TypeScript(s) => s.byte_end(),
        }
    }

    fn line_start(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.line_start(),
            SymbolWrapper::Python(s) => s.line_start(),
            SymbolWrapper::Cpp(s) => s.line_start(),
            SymbolWrapper::Java(s) => s.line_start(),
            SymbolWrapper::JavaScript(s) => s.line_start(),
            SymbolWrapper::TypeScript(s) => s.line_start(),
        }
    }

    fn line_end(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.line_end(),
            SymbolWrapper::Python(s) => s.line_end(),
            SymbolWrapper::Cpp(s) => s.line_end(),
            SymbolWrapper::Java(s) => s.line_end(),
            SymbolWrapper::JavaScript(s) => s.line_end(),
            SymbolWrapper::TypeScript(s) => s.line_end(),
        }
    }

    fn col_start(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.col_start(),
            SymbolWrapper::Python(s) => s.col_start(),
            SymbolWrapper::Cpp(s) => s.col_start(),
            SymbolWrapper::Java(s) => s.col_start(),
            SymbolWrapper::JavaScript(s) => s.col_start(),
            SymbolWrapper::TypeScript(s) => s.col_start(),
        }
    }

    fn col_end(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.col_end(),
            SymbolWrapper::Python(s) => s.col_end(),
            SymbolWrapper::Cpp(s) => s.col_end(),
            SymbolWrapper::Java(s) => s.col_end(),
            SymbolWrapper::JavaScript(s) => s.col_end(),
            SymbolWrapper::TypeScript(s) => s.col_end(),
        }
    }

    fn fully_qualified(&self) -> &str {
        match self {
            SymbolWrapper::Rust(s) => s.fully_qualified(),
            SymbolWrapper::Python(s) => s.fully_qualified(),
            SymbolWrapper::Cpp(s) => s.fully_qualified(),
            SymbolWrapper::Java(s) => s.fully_qualified(),
            SymbolWrapper::JavaScript(s) => s.fully_qualified(),
            SymbolWrapper::TypeScript(s) => s.fully_qualified(),
        }
    }

    fn language(&self) -> splice::symbol::Language {
        match self {
            SymbolWrapper::Rust(_) => splice::symbol::Language::Rust,
            SymbolWrapper::Python(_) => splice::symbol::Language::Python,
            SymbolWrapper::Cpp(_) => splice::symbol::Language::Cpp,
            SymbolWrapper::Java(_) => splice::symbol::Language::Java,
            SymbolWrapper::JavaScript(_) => splice::symbol::Language::JavaScript,
            SymbolWrapper::TypeScript(_) => splice::symbol::Language::TypeScript,
        }
    }
}
