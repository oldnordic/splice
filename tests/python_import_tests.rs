//! Python import extraction tests.
//!
//! TDD for Phase 5.2-5.3: Python Import Extraction
//! - import module
//! - from ... import
//! - relative imports (., ..)
//! - glob imports (*)

use splice::ingest::imports::{extract_python_imports, ImportKind};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_import() {
        // Source: import os
        let source = b"import os\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonImport);
        assert_eq!(imports[0].path, vec!["os"]);
        assert_eq!(imports[0].imported_names, vec!["os"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_dotted_import() {
        // Source: import os.path
        let source = b"import os.path\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonImport);
        assert_eq!(imports[0].path, vec!["os", "path"]);
        assert_eq!(imports[0].imported_names, vec!["os"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_multi_import() {
        // Source: import os, sys
        let source = b"import os, sys\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 2);

        // First import: os
        assert_eq!(imports[0].import_kind, ImportKind::PythonImport);
        assert_eq!(imports[0].path, vec!["os"]);
        assert_eq!(imports[0].imported_names, vec!["os"]);

        // Second import: sys
        assert_eq!(imports[1].import_kind, ImportKind::PythonImport);
        assert_eq!(imports[1].path, vec!["sys"]);
        assert_eq!(imports[1].imported_names, vec!["sys"]);
    }

    #[test]
    fn test_extract_from_import() {
        // Source: from os import path
        let source = b"from os import path\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFrom);
        assert_eq!(imports[0].path, vec!["os"]);
        assert_eq!(imports[0].imported_names, vec!["path"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_from_import_multiple() {
        // Source: from os import path, environ
        let source = b"from os import path, environ\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFrom);
        assert_eq!(imports[0].path, vec!["os"]);
        assert_eq!(imports[0].imported_names, vec!["path", "environ"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_from_import_dotted_module() {
        // Source: from os.path import join
        let source = b"from os.path import join\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFrom);
        assert_eq!(imports[0].path, vec!["os", "path"]);
        assert_eq!(imports[0].imported_names, vec!["join"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_from_import_glob() {
        // Source: from os import *
        let source = b"from os import *\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFrom);
        assert_eq!(imports[0].path, vec!["os"]);
        assert!(imports[0].is_glob);
    }

    #[test]
    fn test_extract_relative_import_current() {
        // Source: from . import helper
        let source = b"from . import helper\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFromRelative);
        assert_eq!(imports[0].path, vec!["."]);
        assert_eq!(imports[0].imported_names, vec!["helper"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_relative_import_parent() {
        // Source: from .. import parent
        let source = b"from .. import parent\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFromParent);
        assert_eq!(imports[0].path, vec![".."]);
        assert_eq!(imports[0].imported_names, vec!["parent"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_relative_import_ancestor() {
        // Source: from ... import ancestor
        let source = b"from ... import ancestor\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFromAncestor);
        assert_eq!(imports[0].path, vec!["..."]);
        assert_eq!(imports[0].imported_names, vec!["ancestor"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_relative_import_with_module() {
        // Source: from .utils import helper
        let source = b"from .utils import helper\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFromRelative);
        assert_eq!(imports[0].path, vec![".", "utils"]);
        assert_eq!(imports[0].imported_names, vec!["helper"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_parent_relative_with_module() {
        // Source: from ..utils import helper
        let source = b"from ..utils import helper\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFromParent);
        assert_eq!(imports[0].path, vec!["..", "utils"]);
        assert_eq!(imports[0].imported_names, vec!["helper"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_aliased_import() {
        // Source: import os as operating_system
        let source = b"import os as operating_system\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonImport);
        assert_eq!(imports[0].path, vec!["os"]);
        // imported_names contains the local alias
        assert_eq!(imports[0].imported_names, vec!["operating_system"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_from_aliased_import() {
        // Source: from os import path as p
        let source = b"from os import path as p\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PythonFrom);
        assert_eq!(imports[0].path, vec!["os"]);
        // imported_names contains the local alias
        assert_eq!(imports[0].imported_names, vec!["p"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_multiple_import_statements() {
        // Source with multiple imports
        let source = b"
            import os
            from sys import argv
            from . import local
            from ..parent import helper
        ";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 4);

        // Check first import
        assert_eq!(imports[0].import_kind, ImportKind::PythonImport);
        assert_eq!(imports[0].path, vec!["os"]);

        // Check second import
        assert_eq!(imports[1].import_kind, ImportKind::PythonFrom);
        assert_eq!(imports[1].path, vec!["sys"]);

        // Check third import
        assert_eq!(imports[2].import_kind, ImportKind::PythonFromRelative);
        assert_eq!(imports[2].path, vec!["."]);

        // Check fourth import
        assert_eq!(imports[3].import_kind, ImportKind::PythonFromParent);
        assert_eq!(imports[3].path, vec!["..", "parent"]);
    }

    #[test]
    fn test_python_import_has_byte_span() {
        // Source: import os
        let source = b"import os\n";

        let imports =
            extract_python_imports(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        // Byte span is (0, 9) for "import os"
        assert_eq!(imports[0].byte_span, (0, 9));
    }
}
