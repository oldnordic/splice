//! Cargo output parsing tests.
//!
//! TDD approach: Write failing tests first, then implement parser.

use splice::validate::{parse_cargo_output, ErrorLevel};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_output() {
        let output = "";
        let errors = parse_cargo_output(output);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_parse_error_with_location() {
        // Standard cargo error format
        let output = r#"error[E0277]: the trait bound `Foo: Bar` is not satisfied
   --> src/main.rs:10:5"#;

        let errors = parse_cargo_output(output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].level, ErrorLevel::Error);
        assert_eq!(errors[0].file, "src/main.rs");
        assert_eq!(errors[0].line, 10);
        assert_eq!(errors[0].column, 5);
        assert!(errors[0].message.contains("trait bound"));
    }

    #[test]
    fn test_parse_warning_with_location() {
        let output = r#"warning: unused variable: `x`
  --> src/lib.rs:42:9"#;

        let errors = parse_cargo_output(output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].level, ErrorLevel::Warning);
        assert_eq!(errors[0].file, "src/lib.rs");
        assert_eq!(errors[0].line, 42);
        assert_eq!(errors[0].column, 9);
        assert!(errors[0].message.contains("unused variable"));
    }

    #[test]
    fn test_parse_multiple_errors() {
        let output = r#"error[E0277]: the trait bound `Foo: Bar` is not satisfied
   --> src/main.rs:10:5
warning: unused variable: `x`
  --> src/lib.rs:42:9"#;

        let errors = parse_cargo_output(output);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].level, ErrorLevel::Error);
        assert_eq!(errors[1].level, ErrorLevel::Warning);
    }

    #[test]
    fn test_parse_ignores_non_matching_lines() {
        let output = r#"Some random text
error: aborting due to previous error
   --> another irrelevant line"#;

        let errors = parse_cargo_output(output);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_parse_error_with_full_path() {
        let output = r#"error[E0382]: use of moved value
   --> /home/user/project/src/main.rs:15:10"#;

        let errors = parse_cargo_output(output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].file, "/home/user/project/src/main.rs");
        assert_eq!(errors[0].line, 15);
        assert_eq!(errors[0].column, 10);
    }
}
