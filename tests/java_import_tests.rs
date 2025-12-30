//! Java import extraction tests.

use splice::ingest::imports::{extract_java_imports, ImportKind};
use std::path::Path;

#[test]
fn test_extract_simple_import() {
    let source = b"import java.util.List;\n";
    let path = Path::new("test.java");
    let result = extract_java_imports(path, source);
    assert!(result.is_ok());
    let imports = result.unwrap();
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].import_kind, ImportKind::JavaImport);
    assert_eq!(imports[0].path, vec!["java", "util", "List"]);
}

#[test]
fn test_extract_static_import() {
    let source = b"import static java.lang.Math.PI;\n";
    let path = Path::new("test.java");
    let result = extract_java_imports(path, source);
    assert!(result.is_ok());
    let imports = result.unwrap();
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].import_kind, ImportKind::JavaStaticImport);
    assert_eq!(imports[0].path, vec!["java", "lang", "Math", "PI"]);
}

#[test]
fn test_extract_wildcard_import() {
    let source = b"import java.util.*;\n";
    let path = Path::new("test.java");
    let result = extract_java_imports(path, source);
    assert!(result.is_ok());
    let imports = result.unwrap();
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].import_kind, ImportKind::JavaImport);
    assert!(imports[0].is_glob);
}

#[test]
fn test_extract_static_wildcard_import() {
    let source = b"import static java.lang.Math.*;\n";
    let path = Path::new("test.java");
    let result = extract_java_imports(path, source);
    assert!(result.is_ok());
    let imports = result.unwrap();
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].import_kind, ImportKind::JavaStaticImport);
    assert!(imports[0].is_glob);
}

#[test]
fn test_extract_multiple_imports() {
    let source = b"import java.util.List;\nimport java.util.ArrayList;\n";
    let path = Path::new("test.java");
    let result = extract_java_imports(path, source);
    assert!(result.is_ok());
    let imports = result.unwrap();
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].path, vec!["java", "util", "List"]);
    assert_eq!(imports[1].path, vec!["java", "util", "ArrayList"]);
}

#[test]
fn test_import_has_byte_span() {
    let source = b"import java.util.List;\n";
    let path = Path::new("test.java");
    let result = extract_java_imports(path, source);
    assert!(result.is_ok());
    let imports = result.unwrap();
    assert_eq!(imports.len(), 1);
    // The import_declaration node ends before the trailing newline
    assert_eq!(imports[0].byte_span, (0, 22));
}
