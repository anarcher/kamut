use clap::Parser;
use kamut::cli::{Args, Cli, Commands};

#[test]
fn test_cli_default_pattern() {
    // Test default pattern
    let cli = Cli::parse_from(["kamut"]);
    assert_eq!(cli.pattern, "*.kamut.yaml");
    assert!(cli.command.is_none());
}

#[test]
fn test_cli_custom_pattern() {
    // Test custom pattern
    let cli = Cli::parse_from(["kamut", "custom*.kamut.yaml"]);
    assert_eq!(cli.pattern, "custom*.kamut.yaml");
    assert!(cli.command.is_none());
}

#[test]
fn test_cli_generate_command_default_pattern() {
    // Test generate command with default pattern
    let cli = Cli::parse_from(["kamut", "generate"]);
    match cli.command {
        Some(Commands::Generate { pattern }) => {
            assert_eq!(pattern, "*.kamut.yaml");
        }
        _ => panic!("Expected Generate command"),
    }
}

#[test]
fn test_cli_generate_command_custom_pattern() {
    // Test generate command with custom pattern
    let cli = Cli::parse_from(["kamut", "generate", "custom*.kamut.yaml"]);
    match cli.command {
        Some(Commands::Generate { pattern }) => {
            assert_eq!(pattern, "custom*.kamut.yaml");
        }
        _ => panic!("Expected Generate command"),
    }
}

#[test]
fn test_cli_version_command() {
    // Test version command
    let cli = Cli::parse_from(["kamut", "version"]);
    match cli.command {
        Some(Commands::Version) => {
            // Command parsed correctly
        }
        _ => panic!("Expected Version command"),
    }
}

#[test]
fn test_args_with_name() {
    // Test Args with name
    let args = Args::parse_from(["kamut", "--name", "test-app"]);
    assert_eq!(args.name, Some("test-app".to_string()));
}

#[test]
fn test_args_with_short_name() {
    // Test Args with short name flag
    let args = Args::parse_from(["kamut", "-n", "test-app"]);
    assert_eq!(args.name, Some("test-app".to_string()));
}

#[test]
fn test_args_without_name() {
    // Test Args without name
    let args = Args::parse_from(["kamut"]);
    assert_eq!(args.name, None);
}
