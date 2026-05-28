//! Status and export command handlers.

use std::path::Path;

use serde_json::json;

pub(crate) fn execute_status(
    db_path: &Path,
    json_output: bool,
    detect_backend: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;

    if !db_path.exists() {
        return Err(splice::SpliceError::Magellan {
            context: format!("Database not found: {}", db_path.display()),
            source: anyhow::anyhow!("No database file at the specified path"),
        });
    }

    if detect_backend {
        let backend = splice::graph::CodeGraph::detect_backend(db_path)?;
        if json_output {
            let output = json!({
                "backend": backend.to_string(),
                "database": db_path.to_string_lossy(),
            });
            return Ok(splice::cli::CliSuccessPayload::with_data(
                format!("Backend: {}", backend),
                output,
            ));
        } else {
            return Ok(splice::cli::CliSuccessPayload::message_only(format!(
                "Backend: {}\nDatabase: {}",
                backend,
                db_path.display()
            )));
        }
    }

    let integration = MagellanIntegration::open(db_path)?;
    let stats = integration.get_statistics()?;

    let data = serde_json::json!({
        "files": stats.files,
        "symbols": stats.symbols,
        "references": stats.references,
        "calls": stats.calls,
        "code_chunks": stats.code_chunks,
        "db_path": db_path.to_string_lossy(),
    });

    if json_output {
        Ok(splice::cli::CliSuccessPayload::with_data(
            format!(
                "Database has {} files, {} symbols",
                stats.files, stats.symbols
            ),
            data,
        ))
    } else {
        let message = format!(
            "Database statistics:\n  Files: {}\n  Symbols: {}\n  References: {}\n  Calls: {}\n  Code chunks: {}",
            stats.files, stats.symbols, stats.references, stats.calls, stats.code_chunks
        );
        Ok(splice::cli::CliSuccessPayload::message_only(message))
    }
}

pub(crate) fn execute_export(
    db_path: &Path,
    format: splice::cli::ExportFormat,
    output: Option<&Path>,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;
    use splice::output::{ExportData, ExportResponse, EXPORT_SCHEMA_VERSION};
    use splice::symbol_id::generate_symbol_id;

    let mut integration = MagellanIntegration::open(db_path)?;

    let files = integration.list_indexed_files(false)?;

    let mut all_symbols = Vec::new();

    for file_metadata in files.iter().take(100) {
        let file_path = std::path::PathBuf::from(&file_metadata.path);
        if let Ok(symbols) = integration.query_symbols_by_file(&file_path, None, false, false) {
            for swr in symbols {
                let sym = swr.symbol;
                let symbol_id = generate_symbol_id(&sym.name, &sym.file_path, sym.byte_start);
                all_symbols.push(splice::output::SymbolExport {
                    symbol_id: symbol_id.to_string(),
                    id_format: Some(if symbol_id.is_v1() { "v1" } else { "v2" }.to_string()),
                    name: sym.name,
                    kind: sym.kind,
                    file_path: sym.file_path,
                    byte_start: sym.byte_start,
                    byte_end: sym.byte_end,
                    start_line: 0,
                    end_line: 0,
                    start_col: 0,
                    end_col: 0,
                });
            }
        }
    }

    let export_data = ExportData {
        files: files
            .iter()
            .map(|f| splice::output::FileExport {
                path: f.path.clone(),
                hash: f.hash.clone(),
                last_indexed_at: f.last_indexed_at,
                last_modified: f.last_modified,
            })
            .collect(),
        symbols: all_symbols,
        references: vec![],
        calls: vec![],
    };

    let response = ExportResponse {
        schema_version: EXPORT_SCHEMA_VERSION.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        db_path: db_path.to_string_lossy().to_string(),
        data: export_data,
    };

    if let Some(path) = output {
        let file = std::fs::File::create(path).map_err(|source| splice::SpliceError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let writer = std::io::BufWriter::new(file);
        write_export(&response, format, writer)?;
    } else {
        let stdout = std::io::stdout();
        let writer = std::io::BufWriter::new(stdout.lock());
        write_export(&response, format, writer)?;
    }

    let file_count = response.data.files.len();
    let symbol_count = response.data.symbols.len();

    Ok(splice::cli::CliSuccessPayload::with_data(
        format!("Exported {} files, {} symbols", file_count, symbol_count),
        serde_json::json!({"files": file_count, "symbols": symbol_count}),
    ))
}

pub(crate) fn write_export<W: std::io::Write>(
    response: &splice::output::ExportResponse,
    format: splice::cli::ExportFormat,
    mut writer: std::io::BufWriter<W>,
) -> Result<(), splice::SpliceError> {
    use std::io::Write;

    match format {
        splice::cli::ExportFormat::Json => {
            serde_json::to_writer_pretty(&mut writer, response).map_err(|e| {
                splice::SpliceError::Other(format!("JSON serialization error: {}", e))
            })?;
        }
        splice::cli::ExportFormat::Jsonl => {
            writeln!(
                writer,
                r#"{{"schema_version": "{}", "type": "header"}}"#,
                response.schema_version
            )
            .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;

            for file in &response.data.files {
                let json = serde_json::to_string(file).map_err(|e| {
                    splice::SpliceError::Other(format!("JSON serialization error: {}", e))
                })?;
                writeln!(writer, r#"{{"type": "file", "data": {}}}"#, json)
                    .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;
            }

            for symbol in &response.data.symbols {
                let json = serde_json::to_string(symbol).map_err(|e| {
                    splice::SpliceError::Other(format!("JSON serialization error: {}", e))
                })?;
                writeln!(writer, r#"{{"type": "symbol", "data": {}}}"#, json)
                    .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;
            }
        }
        splice::cli::ExportFormat::Csv => {
            use csv::Writer;

            writeln!(writer, "# Files")
                .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;
            {
                let mut wtr = Writer::from_writer(&mut writer);
                for file in &response.data.files {
                    wtr.serialize(file).map_err(|e| {
                        splice::SpliceError::Other(format!("CSV write error: {}", e))
                    })?;
                }
                wtr.flush()
                    .map_err(|e| splice::SpliceError::Other(format!("CSV flush error: {}", e)))?;
            }

            writeln!(writer, "\n# Symbols")
                .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;
            {
                let mut wtr = Writer::from_writer(&mut writer);
                for symbol in &response.data.symbols {
                    wtr.serialize(symbol).map_err(|e| {
                        splice::SpliceError::Other(format!("CSV write error: {}", e))
                    })?;
                }
                wtr.flush()
                    .map_err(|e| splice::SpliceError::Other(format!("CSV flush error: {}", e)))?;
            }
        }
    }

    writer
        .flush()
        .map_err(|e| splice::SpliceError::Other(format!("Flush error: {}", e)))?;

    Ok(())
}
