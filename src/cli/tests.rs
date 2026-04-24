//! CLI tests for create command

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::path::PathBuf;

    #[test]
    fn test_create_command_exists() {
        // Should parse create command without error
        let args = Cli::try_parse_from(["splice", "create", "--file", "test.rs"]);
        assert!(args.is_ok());
    }

    #[test]
    fn test_create_command_file_argument() {
        let args = Cli::try_parse_from([
            "splice",
            "create",
            "--file",
            "src/test.rs",
        ])
        .unwrap();

        // Should have Create subcommand with file path
        match args.command {
            Commands::Create { file, .. } => {
                assert_eq!(file, PathBuf::from("src/test.rs"));
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_create_command_validate_only_flag() {
        let args = Cli::try_parse_from([
            "splice",
            "create",
            "--file",
            "test.rs",
            "--validate-only",
        ])
        .unwrap();

        match args.command {
            Commands::Create { validate_only, .. } => {
                assert!(validate_only);
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_create_command_with_mod_flag() {
        let args = Cli::try_parse_from([
            "splice",
            "create",
            "--file",
            "src/commands/test.rs",
            "--with-mod",
        ])
        .unwrap();

        match args.command {
            Commands::Create { with_mod, .. } => {
                assert!(with_mod);
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_create_command_workspace_argument() {
        let args = Cli::try_parse_from([
            "splice",
            "create",
            "--file",
            "test.rs",
            "--workspace",
            "/path/to/workspace",
        ])
        .unwrap();

        match args.command {
            Commands::Create { workspace, .. } => {
                assert_eq!(workspace, PathBuf::from("/path/to/workspace"));
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_create_command_default_workspace() {
        let args = Cli::try_parse_from(["splice", "create", "--file", "test.rs"]).unwrap();

        match args.command {
            Commands::Create { workspace, .. } => {
                assert_eq!(workspace, PathBuf::from(".")); // Default is current directory
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_create_command_all_flags() {
        let args = Cli::try_parse_from([
            "splice",
            "create",
            "--file",
            "src/test.rs",
            "--validate-only",
            "--with-mod",
            "--workspace",
            "/custom/workspace",
        ])
        .unwrap();

        match args.command {
            Commands::Create {
                file,
                validate_only,
                with_mod,
                workspace,
                ..
            } => {
                assert_eq!(file, PathBuf::from("src/test.rs"));
                assert!(validate_only);
                assert!(with_mod);
                assert_eq!(workspace, PathBuf::from("/custom/workspace"));
            }
            _ => panic!("Expected Create command"),
        }
    }
}
