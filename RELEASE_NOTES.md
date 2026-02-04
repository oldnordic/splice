# Splice v2.4.0 Release Notes

**Release Date:** 2026-02-04

Splice v2.4.0 is a feature release that improves preview mode JSON output and updates documentation.

---

## What's Fixed

### Preview Mode JSON Output

The `splice patch --preview --json` command now returns structured JSON:

```json
{
  "status": "ok",
  "message": "Previewed patch 'foo' at bytes 0..29 (dry-run)",
  "data": {
    "symbol": "foo",
    "preview_report": {
      "file": "src/lib.rs",
      "line_start": 1,
      "line_end": 3,
      "lines_added": 3,
      "lines_removed": 3,
      "bytes_added": 31,
      "bytes_removed": 29
    },
    "files": [{"file": "src/lib.rs"}]
  }
}
```

**Fields:**
- `preview_report.file`: Path to the modified file
- `preview_report.line_start`: Starting line number (1-indexed)
- `preview_report.line_end`: Ending line number (1-indexed)
- `preview_report.lines_added`: Number of lines added
- `preview_report.lines_removed`: Number of lines removed
- `preview_report.bytes_added`: Number of bytes added
- `preview_report.bytes_removed`: Number of bytes removed

## Documentation Updates

- **README.md**: Reorganized with badges, toolset diagram, and clearer command reference
- **MANUAL.md**: Condensed from 1654 to 342 lines, added preview JSON examples
- **CHANGELOG.md**: Updated with v2.2.4 changes

## Installation

```bash
cargo install splice
```

Or build from source:

```bash
git clone https://github.com/oldnordic/splice
cd splice
cargo build --release
cp target/release/splice ~/.local/bin/
```

## Upgrade from v2.2.x

No breaking changes. Simply reinstall:

```bash
cargo install splice --force
```

## Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for complete version history.
