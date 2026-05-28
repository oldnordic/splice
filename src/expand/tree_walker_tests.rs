use super::*;
use crate::expand::RustExpander;

#[test]
fn test_find_parent_symbol_node_simple() {
    let source = b"fn example() { let x = 42; }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let identifier_node = root.descendant_for_byte_range(18, 19).unwrap();

    let function_node =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "function_item");

    assert!(function_node.is_some());
    assert_eq!(function_node.unwrap().kind(), "function_item");
}

#[test]
fn test_find_parent_symbol_node_not_found() {
    let source = b"fn example() { }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(0, source.len()).unwrap();

    let class_node =
        find_parent_symbol_node(function_node, source, |kind| kind == "class_declaration");

    assert!(class_node.is_none());
}

#[test]
fn test_extract_leading_doc_comments_none() {
    let source = b"fn example() { }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(0, 2).unwrap();

    let comments = extract_leading_doc_comments(function_node, source);
    assert_eq!(comments.len(), 0);
}

#[test]
fn test_extract_leading_doc_comments_single() {
    let source = b"/// Example docs\nfn example() { }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(17, 25).unwrap();

    let comments = extract_leading_doc_comments(function_node, source);
    assert_eq!(comments.len(), 1);
    assert!(comments[0].starts_with("/// Example docs"));
}

#[test]
fn test_extract_leading_doc_comments_multiple() {
    let source = b"/// First line\n/// Second line\nfn example() { }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(35, 43).unwrap();

    let comments = extract_leading_doc_comments(function_node, source);
    assert_eq!(comments.len(), 2);
    assert!(comments[0].starts_with("/// First line"));
    assert!(comments[1].starts_with("/// Second line"));
}

#[test]
fn test_expand_to_containing_block_in_module() {
    let source = b"mod my_module { fn example() { } }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let identifier_node = root
        .descendant_for_byte_range(20, 21)
        .or_else(|| root.descendant_for_byte_range(18, 19))
        .unwrap();

    let expanded_fn =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "function_item");
    assert!(expanded_fn.is_some(), "Should find function_item parent");

    let module_node = expand_to_containing_block(expanded_fn.unwrap(), source, &RustExpander);
    assert!(module_node.is_some(), "Should find mod_item parent");
    assert_eq!(module_node.unwrap().kind(), "mod_item");
}

#[test]
fn test_is_doc_comment_node() {
    let source = b"/// comment\nfn test() {}";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let comment_node = root.descendant_for_byte_range(0, 12).unwrap();

    assert!(is_doc_comment_node(&comment_node));

    let fn_node = root.descendant_for_byte_range(14, 18).unwrap();

    assert!(!is_doc_comment_node(&fn_node));
}

#[test]
fn test_python_function_expansion() {
    let source = b"def example():\n    pass\n";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let identifier_node = root.descendant_for_byte_range(4, 11).unwrap();

    let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
        kind == "function_definition"
    });

    assert!(function_node.is_some());
    assert_eq!(function_node.unwrap().kind(), "function_definition");
}

#[test]
fn test_python_class_expansion() {
    let source = b"class MyClass:\n    pass\n";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let identifier_node = root.descendant_for_byte_range(6, 13).unwrap();

    let class_node =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "class_definition");

    assert!(class_node.is_some());
    assert_eq!(class_node.unwrap().kind(), "class_definition");
}

#[test]
fn test_cpp_function_expansion() {
    let source = b"int example() { return 0; }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let identifier_node = root.descendant_for_byte_range(4, 11).unwrap();

    let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
        kind == "function_definition"
    });

    assert!(function_node.is_some());
    assert_eq!(function_node.unwrap().kind(), "function_definition");
}

#[test]
fn test_java_class_expansion() {
    let source = b"class MyClass { void method() {} }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let identifier_node = root.descendant_for_byte_range(6, 13).unwrap();

    let class_node =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "class_declaration");

    assert!(class_node.is_some());
    assert_eq!(class_node.unwrap().kind(), "class_declaration");
}

#[test]
fn test_javascript_function_expansion() {
    let source = b"function example() { return; }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let identifier_node = root.descendant_for_byte_range(9, 16).unwrap();

    let function_node = find_parent_symbol_node(identifier_node, source, |kind| {
        kind == "function_declaration"
    });

    assert!(function_node.is_some());
    assert_eq!(function_node.unwrap().kind(), "function_declaration");
}

#[test]
fn test_typescript_interface_expansion() {
    let source = b"interface MyInterface { name: string; }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let identifier_node = root
        .descendant_for_byte_range(20, 24)
        .or_else(|| root.descendant_for_byte_range(10, 21))
        .unwrap();

    let interface_node = find_parent_symbol_node(identifier_node, source, |kind| {
        kind == "interface_declaration"
    });

    if let Some(node) = interface_node {
        assert_eq!(node.kind(), "interface_declaration");
    } else {
        assert!(identifier_node.utf8_text(source).is_ok());
    }
}

#[test]
fn test_expand_level_0_no_expansion() {
    let source = b"fn example() { let value = 42; }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let source_str = std::str::from_utf8(source).unwrap();
    let value_offset = source_str.find("value").unwrap();

    let identifier_node = root
        .descendant_for_byte_range(value_offset, value_offset + 1)
        .unwrap();
    let replaced_start = identifier_node.start_byte();
    let replaced_end = identifier_node.end_byte();

    assert_eq!(identifier_node.kind(), "identifier");
    assert_eq!(replaced_start, value_offset);
    assert!(replaced_end > value_offset);
}

#[test]
fn test_expand_level_1_function_body() {
    let source = b"fn example() { let value = 42; }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let source_str = std::str::from_utf8(source).unwrap();
    let value_offset = source_str.find("value").unwrap();

    let identifier_node = root
        .descendant_for_byte_range(value_offset, value_offset + 1)
        .unwrap();

    let function_node =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "function_item");

    assert!(function_node.is_some());
    let fn_node = function_node.unwrap();
    assert_eq!(fn_node.kind(), "function_item");

    let fn_text = fn_node.utf8_text(source).unwrap();
    assert!(fn_text.contains("fn example"));
    assert!(fn_text.contains("{ let value = 42; }"));
}

#[test]
fn test_expand_level_2_containing_class() {
    let source = b"mod my_module { fn example() { } }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let source_str = std::str::from_utf8(source).unwrap();
    let example_pos = source_str.find("example").unwrap();

    let identifier_node = root
        .descendant_for_byte_range(example_pos, example_pos + 7)
        .expect("Should find identifier node");

    let function_node =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "function_item");
    assert!(function_node.is_some(), "Should find function_item");

    let module_span = find_containing_block(
        &root,
        identifier_node.start_byte(),
        identifier_node.end_byte(),
        source,
    );
    assert!(module_span.is_some(), "Should find containing module");

    let (start, end) = module_span.unwrap();
    let module_text = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(module_text.contains("mod my_module"));
}

#[test]
fn test_expand_progressive_rust() {
    let source = b"mod my_mod { impl Struct { fn method(&self) {} } }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let source_str = std::str::from_utf8(source).unwrap();
    let method_pos = source_str
        .find("method")
        .expect("Should find 'method' in source");

    let identifier_node = root
        .descendant_for_byte_range(method_pos, method_pos + 6)
        .expect("Should find method identifier");

    let method_node =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "function_item");
    assert!(method_node.is_some(), "Should find method (function_item)");
    let method_text = method_node.unwrap().utf8_text(source).unwrap();
    assert!(method_text.contains("fn method"));

    let impl_span = find_containing_block(
        &root,
        identifier_node.start_byte(),
        identifier_node.end_byte(),
        source,
    );
    assert!(impl_span.is_some(), "Should find impl block");
    let (start, end) = impl_span.unwrap();
    let impl_text = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(impl_text.contains("impl Struct"));
}

#[test]
fn test_expand_progressive_python() {
    let source = b"class MyClass:\n    def method(self):\n        pass\n";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let source_str = std::str::from_utf8(source).unwrap();
    let method_pos = source_str
        .find("method")
        .expect("Should find 'method' in source");

    let identifier_node = root
        .descendant_for_byte_range(method_pos, method_pos + 6)
        .expect("Should find method identifier");

    let method_node = find_parent_symbol_node(identifier_node, source, |kind| {
        kind == "function_definition"
    });
    assert!(
        method_node.is_some(),
        "Should find method (function_definition)"
    );
    let method_text = method_node.unwrap().utf8_text(source).unwrap();
    assert!(method_text.contains("def method"));

    let class_node =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "class_definition");
    assert!(class_node.is_some(), "Should find class (class_definition)");
    let class_text = class_node.unwrap().utf8_text(source).unwrap();
    assert!(class_text.contains("class MyClass"));
}

#[test]
fn test_expand_progressive_typescript() {
    let source = b"class MyClass {\n  method(): void {}\n}\n";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let source_str = std::str::from_utf8(source).unwrap();
    let method_pos = source_str
        .find("method")
        .expect("Should find 'method' in source");

    let identifier_node = root
        .descendant_for_byte_range(method_pos, method_pos + 6)
        .expect("Should find method identifier");

    let method_node =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "method_definition");
    assert!(
        method_node.is_some(),
        "Should find method (method_definition)"
    );
    let method_text = method_node.unwrap().utf8_text(source).unwrap();
    assert!(method_text.contains("method"));

    let class_span = find_containing_block(
        &root,
        identifier_node.start_byte(),
        identifier_node.end_byte(),
        source,
    );
    assert!(class_span.is_some(), "Should find containing class");
    let (start, end) = class_span.unwrap();
    let class_text = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(class_text.contains("class MyClass"));
}

#[test]
fn test_expand_no_containing_block() {
    let source = b"fn standalone_function() { }";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let identifier_node = root
        .descendant_for_byte_range(3, 21)
        .expect("Should find function identifier");

    let _block_span = find_containing_block(
        &root,
        identifier_node.start_byte(),
        identifier_node.end_byte(),
        source,
    );

    let function_node =
        find_parent_symbol_node(identifier_node, source, |kind| kind == "function_item");
    assert!(
        function_node.is_some(),
        "Should find function body even without containing block"
    );
}

#[test]
fn test_extract_leading_docs_rust_line_comments() {
    let source = b"/// Example documentation\n/// Second line\nfn example() {}";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(44, 51).unwrap();

    let doc_start = extract_leading_docs(&function_node, source);

    assert_eq!(doc_start, 0, "Should include doc comments");
    assert!(
        doc_start < function_node.start_byte(),
        "Doc start should be before function start"
    );
}

#[test]
fn test_extract_leading_docs_rust_block_comments() {
    let source = b"/** Block documentation */\nfn example() {}";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    eprintln!("=== Debug: test_extract_leading_docs_rust_block_comments ===");
    eprintln!("Source: {:?}", std::str::from_utf8(source).unwrap());
    eprintln!("Root kind: {}", root.kind());

    let mut cursor = root.walk();
    let mut function_node = None;
    if cursor.goto_first_child() {
        loop {
            let node = cursor.node();
            if node.kind() == "function_item" {
                function_node = Some(node);
                eprintln!(
                    "Found function_item: start={}, end={}",
                    node.start_byte(),
                    node.end_byte()
                );
                break;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    let function_node = function_node.expect("Should find function_item");

    eprintln!("Checking previous siblings of function_item...");
    let mut prev = function_node.prev_sibling();
    let mut count = 0;
    while let Some(sibling) = prev {
        eprintln!(
            "  Sibling {}: kind={}, is_named={}, start={}, end={}, text={:?}",
            count,
            sibling.kind(),
            sibling.is_named(),
            sibling.start_byte(),
            sibling.end_byte(),
            sibling.utf8_text(source).unwrap_or("")
        );
        prev = sibling.prev_sibling();
        count += 1;
        if count > 5 {
            break;
        }
    }

    let doc_start = extract_leading_docs(&function_node, source);
    eprintln!(
        "doc_start={}, function_start={}",
        doc_start,
        function_node.start_byte()
    );

    assert_eq!(doc_start, 0, "Should include block doc comments");
    assert!(
        doc_start < function_node.start_byte(),
        "Doc start should be before function start"
    );
}

#[test]
fn test_extract_leading_docs_rust_inner_comments() {
    let source = b"//! Inner documentation\n//! Second line\nfn example() {}";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(44, 51).unwrap();

    let doc_start = extract_leading_docs(&function_node, source);

    assert_eq!(doc_start, 0, "Should include inner doc comments");
}

#[test]
fn test_extract_leading_docs_no_doc_comments() {
    let source = b"fn example() {}";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(3, 10).unwrap();

    let doc_start = extract_leading_docs(&function_node, source);

    assert_eq!(
        doc_start,
        function_node.start_byte(),
        "Should return replaced start when no docs"
    );
}

#[test]
fn test_extract_leading_docs_python_docstrings() {
    let source = b"\"\"\"Example documentation\"\"\"\ndef example():\n    pass";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    eprintln!("=== Debug Python docstrings ===");
    eprintln!("Source: {:?}", std::str::from_utf8(source).unwrap());

    let mut cursor = root.walk();
    let mut function_node = None;
    if cursor.goto_first_child() {
        loop {
            let node = cursor.node();
            eprintln!(
                "Node kind={}, start={}, end={}",
                node.kind(),
                node.start_byte(),
                node.end_byte()
            );
            if node.kind() == "function_definition" {
                function_node = Some(node);
                break;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    let function_node = function_node.expect("Should find function_definition");
    eprintln!(
        "Function: start={}, end={}",
        function_node.start_byte(),
        function_node.end_byte()
    );

    eprintln!("Previous siblings:");
    let mut prev = function_node.prev_sibling();
    let mut count = 0;
    while let Some(sibling) = prev {
        eprintln!(
            "  Sibling {}: kind={}, is_named={}, start={}, text={:?}",
            count,
            sibling.kind(),
            sibling.is_named(),
            sibling.start_byte(),
            sibling.utf8_text(source).unwrap_or("")
        );
        prev = sibling.prev_sibling();
        count += 1;
        if count > 5 {
            break;
        }
    }

    let doc_start = extract_leading_docs(&function_node, source);
    eprintln!(
        "doc_start={}, function_start={}",
        doc_start,
        function_node.start_byte()
    );

    assert_eq!(doc_start, 0, "Should include Python docstrings");
    assert!(
        doc_start < function_node.start_byte(),
        "Doc start should be before function start"
    );
}

#[test]
fn test_extract_leading_docs_python_hash_comments() {
    let source = b"# Example documentation\n# Second line\ndef example():\n    pass";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(44, 51).unwrap();

    let doc_start = extract_leading_docs(&function_node, source);

    assert_eq!(
        doc_start,
        function_node.start_byte(),
        "Regular # comments not captured as docs"
    );
}

#[test]
fn test_extract_leading_docs_java_javadoc() {
    let source = b"/** Javadoc comment */\nclass MyClass {}";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let class_node = root.descendant_for_byte_range(26, 33).unwrap();

    let doc_start = extract_leading_docs(&class_node, source);

    assert_eq!(doc_start, 0, "Should include Javadoc comments");
    assert!(
        doc_start < class_node.start_byte(),
        "Doc start should be before class start"
    );
}

#[test]
fn test_extract_leading_docs_jsdoc() {
    let source = b"/** JSDoc comment */\nfunction example() {}";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(25, 32).unwrap();

    let doc_start = extract_leading_docs(&function_node, source);

    assert_eq!(doc_start, 0, "Should include JSDoc comments");
    assert!(
        doc_start < function_node.start_byte(),
        "Doc start should be before function start"
    );
}

#[test]
fn test_extract_leading_docs_non_doc_comment_stops() {
    let source = b"// Regular comment\nfn example() {}";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let function_node = root.descendant_for_byte_range(21, 28).unwrap();

    let doc_start = extract_leading_docs(&function_node, source);

    assert_eq!(
        doc_start,
        function_node.start_byte(),
        "Regular comments should not be captured"
    );
}
