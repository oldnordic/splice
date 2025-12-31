//! C/C++ include directive extraction tests.
//!
//! TDD for Phase 6.2-6.3: C/C++ Import Extraction
//! - system includes (#include <header.h>)
//! - local includes (#include "header.h")
//! - nested paths (boost/filesystem.hpp)
//! - preprocessor conditionals

use splice::ingest::imports::{extract_cpp_imports, ImportKind};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_system_include() {
        // Source: #include <stdio.h>
        let source = b"#include <stdio.h>\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[0].path, vec!["stdio.h"]);
        assert_eq!(imports[0].imported_names, vec!["stdio.h"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_local_include() {
        // Source: #include "myheader.h"
        let source = b"#include \"myheader.h\"\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppLocalInclude);
        assert_eq!(imports[0].path, vec!["myheader.h"]);
        assert_eq!(imports[0].imported_names, vec!["myheader.h"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_nested_system_include() {
        // Source: #include <boost/filesystem.hpp>
        let source = b"#include <boost/filesystem.hpp>\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[0].path, vec!["boost", "filesystem.hpp"]);
        assert_eq!(imports[0].imported_names, vec!["boost/filesystem.hpp"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_nested_local_include() {
        // Source: #include "utils/helper.h"
        let source = b"#include \"utils/helper.h\"\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppLocalInclude);
        assert_eq!(imports[0].path, vec!["utils", "helper.h"]);
        assert_eq!(imports[0].imported_names, vec!["utils/helper.h"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_multiple_includes() {
        // Source with multiple includes
        let source = b"
#include <stdio.h>
#include \"local.h\"
#include <vector>
#include \"utils/helper.h\"
";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 4);

        // First include: <stdio.h>
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[0].path, vec!["stdio.h"]);

        // Second include: "local.h"
        assert_eq!(imports[1].import_kind, ImportKind::CppLocalInclude);
        assert_eq!(imports[1].path, vec!["local.h"]);

        // Third include: <vector>
        assert_eq!(imports[2].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[2].path, vec!["vector"]);

        // Fourth include: "utils/helper.h"
        assert_eq!(imports[3].import_kind, ImportKind::CppLocalInclude);
        assert_eq!(imports[3].path, vec!["utils", "helper.h"]);
    }

    #[test]
    fn test_extract_include_with_whitespace() {
        // Source: #include  <  stdio.h  >  (extra whitespace)
        let source = b"#include  <  stdio.h  >\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        // Note: tree-sitter may normalize whitespace differently
        // The path should still contain stdio.h
        assert!(imports[0].path.last().is_some_and(|p| p.contains("stdio")));
    }

    #[test]
    fn test_extract_include_in_conditional() {
        // Source with conditional compilation
        let source = b"
#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#endif
";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 2);

        // Both includes should be extracted (we don't evaluate conditionals)
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[0].path, vec!["windows.h"]);

        assert_eq!(imports[1].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[1].path, vec!["unistd.h"]);
    }

    #[test]
    fn test_empty_source() {
        // Source with no includes
        let source = b"int main() { return 0; }\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 0);
    }

    #[test]
    fn test_include_with_code_after() {
        // Source with include followed by code
        let source = b"#include <stdio.h>\nint main() { return 0; }\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[0].path, vec!["stdio.h"]);
    }

    #[test]
    fn test_cpp_system_include_has_byte_span() {
        // Source: #include <stdio.h>
        let source = b"#include <stdio.h>\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        // Byte span should cover the entire include directive (excluding newline)
        assert_eq!(imports[0].byte_span, (0, 19));
    }

    #[test]
    fn test_cpp_local_include_has_byte_span() {
        // Source: #include "myheader.h"
        let source = b"#include \"myheader.h\"\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        // Byte span should cover the entire include directive
        assert_eq!(imports[0].byte_span, (0, 22));
    }

    #[test]
    fn test_deeply_nested_include_path() {
        // Source: #include "project/internal/utils/helper.h"
        let source = b"#include \"project/internal/utils/helper.h\"\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppLocalInclude);
        assert_eq!(
            imports[0].path,
            vec!["project", "internal", "utils", "helper.h"]
        );
        assert_eq!(
            imports[0].imported_names,
            vec!["project/internal/utils/helper.h"]
        );
    }

    #[test]
    fn test_include_no_newline() {
        // Source without trailing newline
        let source = b"#include <stdio.h>";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[0].path, vec!["stdio.h"]);
    }

    #[test]
    fn test_include_with_comment_before() {
        // Source with comment before include
        let source = b"// Standard I/O\n#include <stdio.h>\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.c"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        assert_eq!(imports[0].path, vec!["stdio.h"]);
    }

    #[test]
    fn test_angle_bracket_include_windows_style() {
        // Source: Windows-style header
        let source = b"#include <Windows.h>\n";

        let imports =
            extract_cpp_imports(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::CppSystemInclude);
        // Case is preserved
        assert_eq!(imports[0].path, vec!["Windows.h"]);
    }
}
