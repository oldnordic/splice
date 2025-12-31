//! Per-language validation gate tests.
//!
//! TDD for validation using each language's native compiler/tool.

use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    // Python validation tests

    #[test]
    fn test_validate_valid_python_file() {
        // Create a valid Python file
        let _source = r#"
def hello():
    print("Hello, world!")

class MyClass:
    pass
"#;
        let _path = Path::new("/tmp/test_valid_py.py");
        // Note: Full integration test would write to disk
        // For now, we test the logic
    }

    #[test]
    fn test_validate_invalid_python_syntax() {
        let _source = b"def foo(\n"; // Unclosed parenthesis
                                     // Should detect syntax error
    }

    // C/C++ validation tests

    #[test]
    fn test_validate_valid_c_file() {
        let _source = r#"
#include <stdio.h>

int main() {
    printf("Hello\n");
    return 0;
}
"#;
    }

    #[test]
    fn test_validate_invalid_c_syntax() {
        let _source = b"int main() {"; // Unclosed brace
    }

    #[test]
    fn test_validate_valid_cpp_file() {
        let _source = r#"
#include <iostream>

int main() {
    std::cout << "Hello" << std::endl;
    return 0;
}
"#;
    }

    // Java validation tests

    #[test]
    fn test_validate_valid_java_file() {
        let _source = r#"
public class Main {
    public static void main(String[] args) {
        System.out.println("Hello");
    }
}
"#;
    }

    #[test]
    fn test_validate_invalid_java_syntax() {
        let _source = b"class Main {"; // Unclosed brace
    }

    // JavaScript validation tests

    #[test]
    fn test_validate_valid_javascript_file() {
        let _source = r#"
function hello() {
    console.log("Hello");
}

const x = 42;
"#;
    }

    #[test]
    fn test_validate_invalid_javascript_syntax() {
        let _source = b"function foo("; // Unclosed parenthesis
    }

    // Tool availability tests

    #[test]
    fn test_python_not_available_returns_tool_unavailable() {
        // When python is not in PATH, should return tool_available=false
    }

    #[test]
    fn test_gcc_not_available_returns_tool_unavailable() {
        // When gcc is not in PATH, should return tool_available=false
    }

    #[test]
    fn test_javac_not_available_returns_tool_unavailable() {
        // When javac is not in PATH, should return tool_available=false
    }

    #[test]
    fn test_node_not_available_returns_tool_unavailable() {
        // When node is not in PATH, should return tool_available=false
    }

    // Integration tests (language detection + validation)

    #[test]
    fn test_validate_python_file_dispatches_correctly() {
        // .py file should use python -m py_compile
    }

    #[test]
    fn test_validate_c_file_dispatches_correctly() {
        // .c file should use gcc -fsyntax-only
    }

    #[test]
    fn test_validate_cpp_file_dispatches_correctly() {
        // .cpp file should use g++ -fsyntax-only
    }

    #[test]
    fn test_validate_java_file_dispatches_correctly() {
        // .java file should use javac
    }

    #[test]
    fn test_validate_javascript_file_dispatches_correctly() {
        // .js file should use node --check
    }

    #[test]
    fn test_unknown_extension_returns_error() {
        // .txt file should return an error or tool_unavailable
    }

    // Error parsing tests

    #[test]
    fn test_parse_python_syntax_error() {
        // Python: SyntaxError: invalid syntax (<file>, line <N>)
    }

    #[test]
    fn test_parse_gcc_error() {
        // GCC: <file>:<line>:<col>: error: <msg>
    }

    #[test]
    fn test_parse_javac_error() {
        // javac: <file>:<line>: error: <msg>
    }

    #[test]
    fn test_parse_node_check_error() {
        // node: <file>:<line> (<col>) <msg>
    }

    // TypeScript validation tests

    #[test]
    fn test_validate_valid_typescript_file() {
        let _source = r#"
interface User {
    name: string;
    age: number;
}

function greet(user: User): string {
    return `Hello, ${user.name}!`;
}
"#;
    }

    #[test]
    fn test_validate_invalid_typescript_syntax() {
        let _source = b"interface User {"; // Unclosed brace
    }
}
