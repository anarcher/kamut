use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Generate Kubernetes manifests from kamut configuration files"
)]
pub struct Args {
    /// Name to search for in {name}-kamut.yaml files
    #[clap(short, long)]
    pub name: Option<String>,
}

/// CLI interface for kamut
#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Generate Kubernetes manifests from kamut configuration files"
)]
pub struct Cli {
    /// File pattern to search for
    #[clap(default_value = "*.kamut.yaml")]
    pub pattern: String,

    /// Optional subcommand
    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate Kubernetes manifests from kamut files
    Generate {
        /// File pattern to search for
        #[clap(default_value = "*.kamut.yaml")]
        pattern: String,
    },
    /// Display the version information
    Version,
}

pub fn parse_args() -> Args {
    Args::parse()
}
