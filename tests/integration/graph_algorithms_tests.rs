//! Integration tests for graph algorithm commands.
//!
//! These tests verify that CLI argument parsing works correctly for the
//! new graph algorithm commands added in Phase 30.

use clap::Parser;
use splice::cli::{Cli, Commands, OutputFormat, ReachabilityDirection, SliceDirection};
use std::path::PathBuf;

#[test]
fn test_reachable_command_basic() {
    let args = vec![
        "splice",
        "reachable",
        "--symbol",
        "main",
        "--path",
        "src/main.rs",
        "--db",
        ".magellan/splice.db",
        "--max-depth",
        "5",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Reachable {
                symbol,
                path,
                db,
                direction,
                max_depth,
                output,
                impact_graph: false,
            } => {
                assert_eq!(symbol, "main");
                assert_eq!(path, PathBuf::from("src/main.rs"));
                assert_eq!(db, PathBuf::from(".magellan/splice.db"));
                assert_eq!(max_depth, 5);
                assert_eq!(direction, ReachabilityDirection::Forward);
                assert_eq!(output, OutputFormat::Human);
            }
            other => panic!("Expected Reachable command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_reachable_command_reverse() {
    let args = vec![
        "splice",
        "reachable",
        "--symbol",
        "process",
        "--path",
        "src/lib.rs",
        "--db",
        "test.db",
        "--direction",
        "reverse",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Reachable { direction, .. } => {
                assert_eq!(direction, ReachabilityDirection::Reverse);
            }
            other => panic!("Expected Reachable command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_reachable_command_both() {
    let args = vec![
        "splice",
        "reachable",
        "--symbol",
        "main",
        "--path",
        "src/main.rs",
        "--direction",
        "both",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Reachable { direction, .. } => {
                assert_eq!(direction, ReachabilityDirection::Both);
            }
            other => panic!("Expected Reachable command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_dead_code_command_basic() {
    let args = vec![
        "splice",
        "dead-code",
        "--entry",
        "main",
        "--path",
        "src/main.rs",
        "--db",
        ".magellan/splice.db",
        "--exclude-public",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::DeadCode {
                entry,
                path,
                db,
                exclude_public,
                group_by_file,
                output,
            } => {
                assert_eq!(entry, "main");
                assert_eq!(path, PathBuf::from("src/main.rs"));
                assert_eq!(db, PathBuf::from(".magellan/splice.db"));
                assert!(exclude_public);
                assert!(group_by_file); // default is true
                assert_eq!(output, OutputFormat::Human);
            }
            other => panic!("Expected DeadCode command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_dead_code_command_json_output() {
    let args = vec![
        "splice",
        "dead-code",
        "--entry",
        "main",
        "--path",
        "src/main.rs",
        "--output",
        "json",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::DeadCode { output, .. } => {
                assert_eq!(output, OutputFormat::Json);
            }
            other => panic!("Expected DeadCode command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_cycles_command_basic() {
    let args = vec![
        "splice",
        "cycles",
        "--db",
        ".magellan/splice.db",
        "--max-cycles",
        "50",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Cycles {
                db,
                symbol,
                path,
                max_cycles,
                show_members,
                output,
            } => {
                assert_eq!(db, PathBuf::from(".magellan/splice.db"));
                assert_eq!(max_cycles, 50);
                assert!(show_members); // default is true
                assert!(symbol.is_none());
                assert!(path.is_none());
                assert_eq!(output, OutputFormat::Human);
            }
            other => panic!("Expected Cycles command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_cycles_with_symbol() {
    let args = vec![
        "splice",
        "cycles",
        "--db",
        ".magellan/splice.db",
        "--symbol",
        "process",
        "--path",
        "src/lib.rs",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Cycles { symbol, path, .. } => {
                assert_eq!(symbol.as_deref(), Some("process"));
                assert_eq!(
                    path.as_ref().map(|p| p.as_path()),
                    Some(PathBuf::from("src/lib.rs").as_path())
                );
            }
            other => panic!("Expected Cycles command with symbol, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_cycles_default_members() {
    // Test that show_members defaults to true
    let args = vec!["splice", "cycles"];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Cycles { show_members, .. } => {
                assert!(show_members); // default is true
            }
            other => panic!("Expected Cycles command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_condense_command_basic() {
    let args = vec![
        "splice",
        "condense",
        "--db",
        ".magellan/splice.db",
        "--show-levels",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Condense {
                db,
                show_members,
                show_levels,
                output,
            } => {
                assert_eq!(db, PathBuf::from(".magellan/splice.db"));
                assert!(show_members); // default is true
                assert!(show_levels);
                assert_eq!(output, OutputFormat::Human);
            }
            other => panic!("Expected Condense command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_condense_command_pretty_output() {
    let args = vec!["splice", "condense", "--output", "pretty"];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Condense { output, .. } => {
                assert_eq!(output, OutputFormat::Pretty);
            }
            other => panic!("Expected Condense command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_condense_default_members() {
    // Test that show_members defaults to true
    let args = vec!["splice", "condense"];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Condense { show_members, .. } => {
                assert!(show_members); // default is true
            }
            other => panic!("Expected Condense command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_slice_command_forward() {
    let args = vec![
        "splice",
        "slice",
        "--target",
        "main",
        "--path",
        "src/main.rs",
        "--db",
        ".magellan/splice.db",
        "--direction",
        "forward",
        "--max-depth",
        "3",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Slice {
                target,
                path,
                db,
                direction,
                max_depth,
                output,
            } => {
                assert_eq!(target, "main");
                assert_eq!(path, PathBuf::from("src/main.rs"));
                assert_eq!(db, PathBuf::from(".magellan/splice.db"));
                assert_eq!(direction, SliceDirection::Forward);
                assert_eq!(max_depth, Some(3));
                assert_eq!(output, OutputFormat::Human);
            }
            other => panic!("Expected Slice command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_slice_command_backward() {
    let args = vec![
        "splice",
        "slice",
        "--target",
        "main",
        "--path",
        "src/main.rs",
        "--db",
        ".magellan/splice.db",
        "--direction",
        "backward",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Slice { direction, .. } => {
                assert_eq!(direction, SliceDirection::Backward);
            }
            other => panic!(
                "Expected Slice command with backward direction, got {:?}",
                other
            ),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_slice_command_unlimited_depth() {
    let args = vec![
        "splice",
        "slice",
        "--target",
        "process",
        "--path",
        "src/lib.rs",
    ];

    match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command {
            Commands::Slice { max_depth, .. } => {
                assert_eq!(max_depth, None); // unlimited
            }
            other => panic!("Expected Slice command, got {:?}", other),
        },
        Err(e) => panic!("Failed to parse args: {}", e),
    }
}

#[test]
fn test_output_format_flags() {
    // Test that all graph commands support --output flag
    let test_cases = vec![
        (
            vec![
                "splice",
                "reachable",
                "--symbol",
                "x",
                "--path",
                "f.rs",
                "--output",
                "json",
            ],
            "reachable",
        ),
        (
            vec![
                "splice",
                "dead-code",
                "--entry",
                "main",
                "--path",
                "f.rs",
                "--output",
                "pretty",
            ],
            "dead-code",
        ),
        (vec!["splice", "cycles", "--output", "json"], "cycles"),
        (vec!["splice", "condense", "--output", "pretty"], "condense"),
        (
            vec![
                "splice", "slice", "--target", "x", "--path", "f.rs", "--output", "json",
            ],
            "slice",
        ),
    ];

    for (args, cmd_name) in test_cases {
        match Cli::try_parse_from(args) {
            Ok(cli) => {
                // Verify output format is set
                let expected_format =
                    if cmd_name == "reachable" || cmd_name == "cycles" || cmd_name == "slice" {
                        OutputFormat::Json
                    } else {
                        OutputFormat::Pretty
                    };
                assert_eq!(
                    cli.output, expected_format,
                    "{} command should have correct output format",
                    cmd_name
                );
            }
            Err(e) => {
                panic!(
                    "Failed to parse {} command with --output flag: {}",
                    cmd_name, e
                );
            }
        }
    }
}

#[test]
fn test_all_graph_commands_have_db_parameter() {
    // Verify all graph commands accept --db parameter
    let commands_with_db: Vec<Vec<&str>> = vec![
        vec![
            "splice",
            "reachable",
            "--symbol",
            "x",
            "--path",
            "f.rs",
            "--db",
            "test.db",
        ],
        vec![
            "splice",
            "dead-code",
            "--entry",
            "main",
            "--path",
            "f.rs",
            "--db",
            "test.db",
        ],
        vec!["splice", "cycles", "--db", "test.db"],
        vec!["splice", "condense", "--db", "test.db"],
        vec![
            "splice", "slice", "--target", "x", "--path", "f.rs", "--db", "test.db",
        ],
    ];

    for args in &commands_with_db {
        match Cli::try_parse_from(args) {
            Ok(_) => {
                // Successfully parsed - db parameter is accepted
            }
            Err(e) => {
                panic!(
                    "Failed to parse command with --db parameter: {} - error: {}",
                    args.join(" "),
                    e
                );
            }
        }
    }
}
