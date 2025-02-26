use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Read {name}-kamut.yaml files in the current directory"
)]
pub struct Args {
    /// Name to search for in {name}-kamut.yaml files
    #[clap(short, long)]
    pub name: Option<String>,
}

pub fn parse_args() -> Args {
    Args::parse()
}
