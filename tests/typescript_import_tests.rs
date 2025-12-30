//! TypeScript import statement extraction tests.
//!
//! TDD for TypeScript Import Extraction
//! - ES6 imports (same as JavaScript)
//! - Type-only imports (TypeScript-specific)
//! - import type { Foo } from 'bar'
//! - import type Foo from 'bar'

use splice::ingest::imports::{extract_typescript_imports, ImportKind};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_named_import() {
        // Source: import { Component } from 'react';
        let source = b"import { Component } from 'react';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JsImport);
        assert_eq!(imports[0].path, vec!["react"]);
        assert_eq!(imports[0].imported_names, vec!["Component"]);
    }

    #[test]
    fn test_extract_default_import() {
        // Source: import React from 'react';
        let source = b"import React from 'react';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JsDefaultImport);
        assert_eq!(imports[0].imported_names, vec!["React"]);
    }

    #[test]
    fn test_extract_namespace_import() {
        // Source: import * as utils from './utils';
        let source = b"import * as utils from './utils';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JsNamespaceImport);
        assert!(imports[0].is_glob);
        assert_eq!(imports[0].imported_names, vec!["utils"]);
    }

    #[test]
    fn test_extract_side_effect_import() {
        // Source: import 'polyfills';
        let source = b"import 'polyfills';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::JsSideEffectImport);
        assert_eq!(imports[0].path, vec!["polyfills"]);
    }

    #[test]
    fn test_extract_type_only_named_import() {
        // Source: import type { User, Admin } from './types';
        let source = b"import type { User, Admin } from './types';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::TsTypeImport);
        assert_eq!(imports[0].path, vec![".", "types"]);
        assert_eq!(imports[0].imported_names, vec!["User", "Admin"]);
    }

    #[test]
    fn test_extract_type_only_default_import() {
        // Source: import type UserModel from './models';
        let source = b"import type UserModel from './models';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::TsTypeDefaultImport);
        assert_eq!(imports[0].imported_names, vec!["UserModel"]);
    }

    #[test]
    fn test_extract_mixed_imports() {
        // Source: import { Component, type Props } from 'react';
        let source = b"import { Component, type Props } from 'react';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        // Mixed imports - should extract both regular and type imports
        assert_eq!(imports[0].imported_names.len(), 2);
        assert!(imports[0].imported_names.contains(&"Component".to_string()));
        assert!(imports[0].imported_names.contains(&"Props".to_string()));
    }

    #[test]
    fn test_extract_multiple_imports() {
        // Source:
        //   import { Foo } from 'bar';
        //   import type Baz from 'qux';
        let source = b"import { Foo } from 'bar';\nimport type Baz from 'qux';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_extract_import_with_alias() {
        // Source: import { Button as Btn } from './ui';
        let source = b"import { Button as Btn } from './ui';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        // Should extract the local alias (Btn)
        assert_eq!(imports[0].imported_names, vec!["Btn"]);
    }

    #[test]
    fn test_extract_import_with_escaped_name() {
        // Source: import { 'foo-bar' as fooBar } from './module';
        let source = b"import { 'foo-bar' as fooBar } from './module';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].imported_names, vec!["fooBar"]);
    }

    #[test]
    fn test_typescript_import_has_byte_span() {
        // Source: import { Foo } from 'bar';
        let source = b"import { Foo } from 'bar';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        // Byte span is set
        assert!(imports[0].byte_span.0 < imports[0].byte_span.1);
    }

    #[test]
    fn test_extract_relative_import() {
        // Source: import { helper } from '../utils/helper';
        let source = b"import { helper } from '../utils/helper';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].path, vec!["..", "utils", "helper"]);
    }

    #[test]
    fn test_extract_scoped_import() {
        // Source: import { useState } from '@react/core';
        let source = b"import { useState } from '@react/core';\n";

        let imports =
            extract_typescript_imports(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].path, vec!["@react", "core"]);
    }
}
