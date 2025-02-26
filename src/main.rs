use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = kamut::cli::Cli::parse();

    match &cli.command {
        kamut::cli::Commands::Generate { pattern } => {
            generate_manifests(pattern)?;
        }
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
