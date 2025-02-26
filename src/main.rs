use anyhow::Result;

fn main() -> Result<()> {
    let args = kamut::cli::parse_args();

    // Pattern to search for
    let pattern = match &args.name {
        Some(name) => format!("{}-kamut.yaml", name),
        None => "*-kamut.yaml".to_string(),
    };

    // Find matching files
    let files = kamut::config::find_config_files(&pattern)?;

    if files.is_empty() {
        println!("No matching kamut files found");
        return Ok(());
    }

    for file_path in files {
        kamut::config::process_file(&file_path)?;
    }

    Ok(())
}
