/// Get detailed explanation for an error code.
///
/// Provides expanded documentation following the `rustc --explain` pattern.
/// Explanations include:
/// - Error code name and description
/// - Common causes
/// - What to do (step-by-step)
/// - Related error codes
///
/// # Arguments
/// * `code` - Error code string (e.g., "SPL-E001", "SPL-E002")
///
/// # Returns
/// * `Some(&str)` with explanation if code is known
/// * `None` if code is unknown
///
/// # Examples
/// ```
/// use splice::error_codes::get_error_explanation;
///
/// let explanation = get_error_explanation("SPL-E001");
/// assert!(explanation.is_some());
/// assert!(explanation.unwrap().contains("Symbol Not Found"));
/// ```
pub fn get_error_explanation(code: &str) -> Option<&'static str> {
    match code {
        // Symbol resolution errors
        "SPL-E001" => Some(
            r#"
Symbol Not Found (SPL-E001)

The specified symbol could not be found in the codebase.

POSSIBLE CAUSES:
- The symbol name is misspelled
- The symbol hasn't been ingested into the code graph
- The symbol exists in multiple files (use --file to disambiguate)
- The symbol is defined in a file that hasn't been indexed

WHAT TO DO:
1. Check the symbol name is spelled correctly
2. Run `magellan watch --root ./src --db <db> --scan-initial` to ensure the codebase is indexed
3. Use `splice query` to search for symbols by label
4. Use `splice delete --file <path>` to specify which file
5. Use `splice explain SPL-E002` for help with ambiguous symbols

RELATED: SPL-E002 (Ambiguous Symbol)
"#,
        ),

        "SPL-E002" => Some(
            r#"
Ambiguous Symbol (SPL-E002)

The symbol name exists in multiple files, making it ambiguous which one
to use without additional context.

POSSIBLE CAUSES:
- Common symbol names like `main`, `run`, `process` used in multiple files
- Multiple files define the same function/struct name
- File context was not specified for the operation

WHAT TO DO:
1. Use `--file <path>` to specify which file contains the symbol
2. Use `splice query --db <db> --label <label>` to list all symbols
3. Rename one of the conflicting symbols if appropriate
4. Use fully-qualified names if supported by the language

RELATED: SPL-E001 (Symbol Not Found)
"#,
        ),

        "SPL-E003" => Some(
            r#"
Reference Failed (SPL-E003)

Failed to locate symbol references in the codebase.

POSSIBLE CAUSES:
- The code graph is incomplete or outdated
- Reference edges haven't been created during ingestion
- The symbol exists but references weren't indexed

WHAT TO DO:
1. Re-run `magellan watch --root ./src --db <db> --scan-initial` to rebuild the code graph
2. Check that the source files haven't been modified since indexing
3. Use `splice log --operation-type ingest` to check ingestion status
4. Report this as a bug if the issue persists

RELATED: SPL-E061 (Graph Error)
"#,
        ),

        "SPL-E004" => Some(
            r#"
Ambiguous Reference (SPL-E004)

A reference could refer to multiple definitions, making it ambiguous
which one is being referenced.

POSSIBLE CAUSES:
- Multiple symbols with the same name in scope
- Import/use statements bring in conflicting names
- Type inference cannot determine which overload to use

WHAT TO DO:
1. Qualify the reference with module/type information
2. Use explicit imports instead of wildcards
3. Rename one of the conflicting symbols if appropriate
4. Check the error message for candidate definitions

RELATED: SPL-E002 (Ambiguous Symbol)
"#,
        ),

        // Parse/AST errors
        "SPL-E011" => Some(
            r#"
Parse Error (SPL-E011)

Tree-sitter failed to parse the source file.

POSSIBLE CAUSES:
- Invalid syntax for the detected language
- Incomplete source file (e.g., missing closing brace)
- File encoding issues (should be UTF-8)
- Corrupted or truncated file

WHAT TO DO:
1. Check file syntax is valid for the language
2. Ensure the file is complete and properly formatted
3. Verify the file is saved as UTF-8 encoding
4. Use a language server or compiler to check for syntax errors
5. Re-ingest the file after fixing syntax

RELATED: SPL-E012 (Invalid UTF-8), SPL-E013 (Invalid Syntax)
"#,
        ),

        "SPL-E012" => Some(
            r#"
Invalid UTF-8 (SPL-E012)

The file contains invalid UTF-8 encoding.

POSSIBLE CAUSES:
- File was saved with a different encoding (e.g., Latin-1, UTF-16)
- File contains binary data or corruption
- File was transferred without preserving encoding

WHAT TO DO:
1. Convert the file to UTF-8 encoding
2. Use `file --mime-encoding <path>` to check current encoding
3. Use `iconv` or similar tool to convert encodings
4. Ensure your editor is configured to save files as UTF-8

RELATED: SPL-E011 (Parse Error)
"#,
        ),

        "SPL-E013" => Some(
            r#"
Invalid Syntax (SPL-E013)

The compiler reported a syntax error in the source file.

POSSIBLE CAUSES:
- Typo or mistake in the source code
- Incomplete statement or block
- Language feature used incorrectly
- Missing imports or dependencies

WHAT TO DO:
1. Check the file syntax and fix any errors reported by the compiler
2. Use the compiler's error messages for specific line/column information
3. Run `cargo check` (Rust) or equivalent for your language
4. Refer to language documentation for correct syntax

RELATED: SPL-E011 (Parse Error), SPL-E043 (Compiler Validation Failed)
"#,
        ),

        // Span errors
        "SPL-E021" => Some(
            r#"
Invalid Span (SPL-E021)

The byte range specified is invalid.

POSSIBLE CAUSES:
- Start position is after end position
- Byte offsets are negative or extremely large
- Span was calculated incorrectly

WHAT TO DO:
1. Ensure start <= end for byte ranges
2. Check that byte offsets are within file bounds
3. Verify the span calculation logic
4. Use `splice query` to get valid spans for symbols

RELATED: SPL-E022 (Invalid Line Range), SPL-E023 (Span Out of Bounds)
"#,
        ),

        "SPL-E022" => Some(
            r#"
Invalid Line Range (SPL-E022)

The line range specified is invalid.

POSSIBLE CAUSES:
- Start line is after end line
- Line numbers are zero or negative
- Line numbers exceed file's total lines

WHAT TO DO:
1. Ensure line_start <= line_end
2. Use 1-based line numbering (first line is 1, not 0)
3. Check the file's total line count
4. Use `splice get` to retrieve valid line ranges

RELATED: SPL-E021 (Invalid Span), SPL-E023 (Span Out of Bounds)
"#,
        ),

        "SPL-E023" => Some(
            r#"
Span Out of Bounds (SPL-E023)

The span extends beyond the file's boundaries.

POSSIBLE CAUSES:
- File was modified since the span was calculated
- Byte offsets were calculated incorrectly
- File size has changed

WHAT TO DO:
1. Re-index the codebase with `magellan watch --root ./src --db <db> --scan-initial`
2. Check that the file hasn't been modified externally
3. Verify the file size matches expectations
4. Use checksums to detect file modifications

RELATED: SPL-E034 (File Externally Modified)
"#,
        ),

        // I/O errors
        "SPL-E031" => Some(
            r#"
File Read Error (SPL-E031)

Failed to read a file.

POSSIBLE CAUSES:
- File permissions prevent reading
- File is locked by another process
- File doesn't exist
- Insufficient system resources

WHAT TO DO:
1. Check file permissions with `ls -l <path>`
2. Ensure the file exists and is not locked
3. Check disk space and system resources
4. Verify the file path is correct

RELATED: SPL-E033 (File Not Found), SPL-E032 (File Write Error)
"#,
        ),

        "SPL-E032" => Some(
            r#"
File Write Error (SPL-E032)

Failed to write to a file.

POSSIBLE CAUSES:
- Insufficient disk space
- File permissions prevent writing
- File is locked by another process
- Directory doesn't exist

WHAT TO DO:
1. Check available disk space with `df -h`
2. Verify write permissions with `ls -ld <dir>`
3. Ensure the file is not open in another program
4. Create parent directories if needed

RELATED: SPL-E031 (File Read Error), SPL-E034 (File Externally Modified)
"#,
        ),

        "SPL-E033" => Some(
            r#"
File Not Found (SPL-E033)

The specified file does not exist.

POSSIBLE CAUSES:
- File path is incorrect or misspelled
- File was deleted or moved
- Relative path is used from wrong directory

WHAT TO DO:
1. Check the file path is correct
2. Use absolute paths if relative paths are problematic
3. Verify the file exists with `ls <path>`
4. Run `magellan watch --root ./src --db <db> --scan-initial` to re-index if file was moved

RELATED: SPL-E031 (File Read Error)
"#,
        ),

        "SPL-E034" => Some(
            r#"
File Externally Modified (SPL-E034)

The file was modified by another process since being indexed.

POSSIBLE CAUSES:
- File was edited in another editor or IDE
- Build process generated or modified the file
- Code formatter or linter modified the file

WHAT TO DO:
1. Re-index the codebase with `magellan watch --root ./src --db <db> --scan-initial`
2. Check for background processes modifying files
3. Use file checksums to detect modifications
4. Consider using file watchers to detect external changes

RELATED: SPL-E023 (Span Out of Bounds)
"#,
        ),

        // Validation errors
        "SPL-E041" => Some(
            r#"
Pre-Verification Failed (SPL-E041)

A pre-verification check failed before applying changes.

POSSIBLE CAUSES:
- Pre-condition checks detected an issue
- Validation gate found a blocking problem
- Resource constraints (disk space, etc.)

WHAT TO DO:
1. Review the specific check that failed
2. Fix the reported issue
3. Use `--skip-pre-verify` with caution (dangerous)
4. Check system resources if applicable

RELATED: SPL-E042 (Parse Validation Failed), SPL-E043 (Compiler Validation Failed)
"#,
        ),

        "SPL-E042" => Some(
            r#"
Parse Validation Failed (SPL-E042)

The file failed to parse after modification.

POSSIBLE CAUSES:
- Modification introduced a syntax error
- Incomplete replacement or deletion
- File was corrupted during write

WHAT TO DO:
1. Revert the change using `splice undo`
2. Check the modification for syntax errors
3. Use `--dry-run` to preview changes before applying
4. Ensure the replacement content is complete and valid

RELATED: SPL-E011 (Parse Error), SPL-E041 (Pre-Verification Failed)
"#,
        ),

        "SPL-E043" => Some(
            r#"
Compiler Validation Failed (SPL-E043)

Compiler reported errors after modification.

POSSIBLE CAUSES:
- Modification introduced compilation errors
- Type errors or missing imports
- Breaking changes to API usage

WHAT TO DO:
1. Check compiler output for specific errors
2. Fix compilation errors before proceeding
3. Use `--dry-run` to preview changes
4. Run compiler manually for detailed error messages

RELATED: SPL-E013 (Invalid Syntax), SPL-E081 (Analyzer Not Available)
"#,
        ),

        // Plan execution errors
        "SPL-E051" => Some(
            r#"
Invalid Plan Schema (SPL-E051)

The plan JSON schema is invalid.

POSSIBLE CAUSES:
- JSON syntax errors in plan file
- Missing required fields
- Invalid data types for fields
- Schema version mismatch

WHAT TO DO:
1. Validate JSON syntax with `jq . < plan.json`
2. Check the plan file format in documentation
3. Ensure all required fields are present
4. Verify schema version matches Splice version

RELATED: SPL-E053 (Invalid Batch Schema)
"#,
        ),

        "SPL-E052" => Some(
            r#"
Plan Execution Failed (SPL-E052)

A step in the plan failed during execution.

POSSIBLE CAUSES:
- One of the plan steps encountered an error
- Resource constraints during execution
- File system issues

WHAT TO DO:
1. Review the specific step that failed
2. Check the error message for details
3. Fix the underlying issue and re-run the plan
4. Use `--dry-run` to preview plan execution

RELATED: SPL-E051 (Invalid Plan Schema)
"#,
        ),

        "SPL-E053" => Some(
            r#"
Invalid Batch Schema (SPL-E053)

The batch JSON schema is invalid.

POSSIBLE CAUSES:
- JSON syntax errors in batch file
- Missing required fields
- Invalid data types for fields
- Incorrect array or object structure

WHAT TO DO:
1. Validate JSON syntax with `jq . < batch.json`
2. Check the batch file format in documentation
3. Ensure all required fields are present
4. Verify the file/replacement structure

RELATED: SPL-E051 (Invalid Plan Schema)
"#,
        ),

        // Graph/database errors
        "SPL-E061" => Some(
            r#"
Graph Error (SPL-E061)

The code graph database encountered an error.

POSSIBLE CAUSES:
- Database corruption
- Insufficient permissions
- Database locked by another process
- Incompatible database version

WHAT TO DO:
1. Check database permissions with `ls -l codegraph.db`
2. Ensure no other process is using the database
3. Try rebuilding the database with `magellan watch --root ./src --db <db> --scan-initial`
4. Check database version compatibility

RELATED: SPL-E062 (Database Error), SPL-E003 (Reference Failed)
"#,
        ),

        "SPL-E062" => Some(
            r#"
Database Error (SPL-E062)

A database operation failed.

POSSIBLE CAUSES:
- Query execution error
- Constraint violation
- Transaction failure
- Connection issues

WHAT TO DO:
1. Check database integrity
2. Rebuild the database if needed
3. Verify sufficient disk space
4. Check for concurrent access issues

RELATED: SPL-E061 (Graph Error)
"#,
        ),

        // Execution log errors
        "SPL-E071" => Some(
            r#"
Execution Log Error (SPL-E071)

Failed to access the execution log database.

POSSIBLE CAUSES:
- Database permissions issue
- Execution log database corrupted
- Database locked by another process

WHAT TO DO:
1. Check execution log permissions
2. Ensure Splice has write access to log directory
3. Close other processes that might be using the log
4. Re-create the log database if corrupted

RELATED: SPL-E062 (Database Error)
"#,
        ),

        "SPL-E072" => Some(
            r#"
Execution Not Found (SPL-E072)

The specified execution ID was not found in the log.

POSSIBLE CAUSES:
- Execution ID is incorrect or misspelled
- Execution was logged to a different database
- Execution log was cleared or truncated

WHAT TO DO:
1. Verify the execution ID is correct
2. Use `splice log --stats` to list recent executions
3. Check the execution log database location
4. Re-run the operation if needed

RELATED: SPL-E071 (Execution Log Error)
"#,
        ),

        // Analyzer errors
        "SPL-E081" => Some(
            r#"
Analyzer Not Available (SPL-E081)

The requested analyzer is not available.

POSSIBLE CAUSES:
- rust-analyzer or other tool is not installed
- Analyzer is not in PATH
- Invalid analyzer path specified

WHAT TO DO:
1. Install the required analyzer (e.g., `rustup component add rust-analyzer`)
2. Ensure the analyzer is in your PATH
3. Use `--analyzer path` to specify explicit path
4. Use `--analyzer off` to disable analyzer validation

RELATED: SPL-E082 (Analyzer Failed)
"#,
        ),

        "SPL-E082" => Some(
            r#"
Analyzer Failed (SPL-E082)

The analyzer reported diagnostics.

POSSIBLE CAUSES:
- Code has issues detected by the analyzer
- Analyzer configuration problems
- False positives from analyzer

WHAT TO DO:
1. Review the analyzer diagnostics
2. Fix legitimate issues reported
3. Consider disabling analyzer for false positives
4. Update analyzer version if outdated

RELATED: SPL-E043 (Compiler Validation Failed), SPL-E081 (Analyzer Not Available)
"#,
        ),

        "SPL-E091" => Some(
            r#"
Magellan Error (SPL-E091)

An error occurred opening or querying the Magellan code graph.

POSSIBLE CAUSES:
- Database file does not exist
- Insufficient permissions to read the database
- Database schema is older than the one this splice binary expects
  (e.g., "DB_COMPAT: sqlitegraph schema mismatch: ... found=3, expected=4")
- Magellan internal error

WHAT TO DO:
1. Check that the database file exists: ls -l <db_path>
2. Verify file permissions: readable by current user
3. If the underlying error mentions a schema mismatch, re-index with a magellan
   version matching this splice (run from the project root):
       magellan watch --root ./src --db .magellan/<project>.db --scan-initial
   This rewrites the database against the current schema.
4. To incrementally sync a stale database (no schema change), use:
       magellan refresh --db .magellan/<project>.db
5. Run `magellan status --db <db_path>` to confirm the database is readable
   by the magellan binary in PATH (if magellan can read it but splice cannot,
   the cause is almost certainly a schema version mismatch).

RELATED: SPL-E061 (Graph Error), SPL-E031 (File Read Error)
"#,
        ),

        // Unknown code
        _ => None,
    }
}
