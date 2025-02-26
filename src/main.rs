use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    // Support both old and new CLI interfaces
    if std::env::args().len() > 1 && std::env::args().nth(1).map_or(false, |arg| arg == "generate" || arg.starts_with("gen")) {
        // New CLI with subcommands
        let cli = kamut::cli::Cli::parse();
        
        match cli.command {
            kamut::cli::Commands::Generate { pattern } => {
                generate_manifests(&pattern)?;
            }
        }
    } else {
        // Legacy CLI for backward compatibility
        let args = kamut::cli::parse_args();

        // Pattern to search for
        let pattern = match &args.name {
            Some(name) => format!("{}-kamut.yaml", name),
            None => "*-kamut.yaml".to_string(),
        };
        
        generate_manifests(&pattern)?;
    }

    Ok(())
}

fn generate_manifests(pattern: &str) -> Result<()> {
    // Find matching files
    let files = kamut::config::find_config_files(pattern)?;

    if files.is_empty() {
        println!("No matching kamut files found for pattern: {}", pattern);
        return Ok(());
    }

    println!("Found {} configuration files", files.len());

    for file_path in files {
        println!("\n=====================");
        kamut::config::process_file(&file_path)?;
        println!("=====================\n");
    }
    
    Ok(())
}
