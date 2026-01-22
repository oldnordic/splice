---
status: complete
phase: 11-rich-span-core
source: 11-01-SUMMARY.md, 11-02-SUMMARY.md, 11-03-SUMMARY.md, 11-04-SUMMARY.md, 11-05-SUMMARY.md, 11-06-SUMMARY.md, 11-07-SUMMARY.md
started: 2026-01-22T10:00:00Z
updated: 2026-01-22T10:06:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Context Extraction in JSON Output
expected: When running splice commands (delete, patch) with JSON output, the span includes a "context" field with "before", "selected", and "after" line arrays. Default is 3 lines of context. The --context-lines flag controls the amount.
result: pass

### 2. Semantic Kind Detection
expected: JSON output includes "semantic_kind" field showing the type of symbol (function, variable, parameter, type, etc.) based on tree-sitter node analysis.
result: pass

### 3. Language Detection
expected: JSON output includes "language" field detected from file extension (rust, python, javascript, typescript, java, c, cpp).
result: pass

### 4. Checksum Fields
expected: JSON output includes "checksum_before" and "file_checksum_before" SHA-256 checksums for race condition protection.
result: pass

### 5. Error Code Field
expected: JSON output includes "error_code" field with structured diagnostics including code (SPL-E### format), severity (error/warning/note), location (file:line:column), and hint (what to do).
result: pass

### 6. Backward Compatibility
expected: Old JSON output (without new fields) still works correctly. New fields are optional and omitted when None, so existing scripts and LLM consumers don't break.
result: pass

## Summary

total: 6
passed: 6
issues: 0
pending: 0
skipped: 0

## Gaps

[none]
