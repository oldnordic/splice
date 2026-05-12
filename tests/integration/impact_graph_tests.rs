//! Integration tests for --impact-graph flag on various commands

use splice::cli::{CallDirection, Commands, OutputFormat, ReachabilityDirection};
use std::path::PathBuf;

#[test]
fn test_refs_command_has_impact_graph_flag() {
    // Verify Refs command struct has impact_graph field
    let refs_cmd = Commands::Refs {
        db: PathBuf::from("/tmp/test.db"),
        name: "test".to_string(),
        path: PathBuf::from("/tmp/test.rs"),
        direction: CallDirection::Both,
        output: OutputFormat::Human,
        impact_graph: true, // This field must exist
    };

    // If we reach here, the Refs command has impact_graph field
    match refs_cmd {
        Commands::Refs { impact_graph, .. } => {
            assert!(impact_graph, "impact_graph flag should be true");
        }
        _ => panic!("Expected Refs command"),
    }
}

#[test]
fn test_reachable_command_has_impact_graph_flag() {
    // Verify Reachable command struct has impact_graph field
    let reachable_cmd = Commands::Reachable {
        symbol: "test".to_string(),
        path: PathBuf::from("/tmp/test.rs"),
        db: PathBuf::from("/tmp/test.db"),
        direction: ReachabilityDirection::Forward,
        max_depth: 10,
        output: OutputFormat::Human,
        impact_graph: true, // This field must exist
    };

    match reachable_cmd {
        Commands::Reachable { impact_graph, .. } => {
            assert!(impact_graph, "impact_graph flag should be true");
        }
        _ => panic!("Expected Reachable command"),
    }
}

#[test]
fn test_rename_command_has_impact_graph_flag() {
    // Verify Rename command struct has impact_graph field
    let rename_cmd = Commands::Rename {
        symbol: Some("abc123".to_string()),
        name: None,
        file: None,
        to: "new_name".to_string(),
        db: PathBuf::from("/tmp/test.db"),
        preview: true,
        proof: false,
        backup_dir: None,
        no_backup: false,
        create_backup: true,
        snapshot_before: false,
        impact_graph: true, // This field must exist
    };

    match rename_cmd {
        Commands::Rename { impact_graph, .. } => {
            assert!(impact_graph, "impact_graph flag should be true");
        }
        _ => panic!("Expected Rename command"),
    }
}

#[test]
fn test_patch_command_has_impact_graph_flag() {
    // Verify Patch command struct has impact_graph field
    let patch_cmd = Commands::Patch {
        file: Some(PathBuf::from("/tmp/test.rs")),
        symbol: Some("test_fn".to_string()),
        kind: None,
        analyzer: None,
        analyzer_binary: None,
        with_: Some(PathBuf::from("/tmp/patch_content.rs")),
        language: None,
        batch: None,
        context_after: 0,
        context_before: 0,
        context_both: 0,
        preview: true,
        unified: 3,
        create_backup: false,
        relationships: false,
        operation_id: None,
        metadata: None,
        db: Some(PathBuf::from("/tmp/test.db")),
        snapshot_before: false,
        impact_graph: true, // This field must exist
    };

    match patch_cmd {
        Commands::Patch { impact_graph, .. } => {
            assert!(impact_graph, "impact_graph flag should be true");
        }
        _ => panic!("Expected Patch command"),
    }
}
