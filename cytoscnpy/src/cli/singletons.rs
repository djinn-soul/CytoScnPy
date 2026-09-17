use super::PathArgs;
use clap::Args;

/// Arguments for the `singletons` pattern detection subcommand.
#[derive(Args, Debug)]
pub struct SingletonsArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 when any singleton pattern is found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exit with code 1 when singleton count exceeds this limit.
    #[arg(long)]
    pub max_singletons: Option<usize>,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
