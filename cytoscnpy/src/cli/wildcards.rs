use super::PathArgs;
use clap::Args;

/// Arguments for the `wildcards` star-import detection subcommand.
#[derive(Args, Debug)]
pub struct WildcardsArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 when any wildcard import is found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exit with code 1 when wildcard import count exceeds this limit.
    #[arg(long)]
    pub max_wildcards: Option<usize>,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
