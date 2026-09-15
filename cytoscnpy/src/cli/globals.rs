use super::PathArgs;
use clap::Args;

/// Arguments for the `globals` mutable global state analysis subcommand.
#[derive(Args, Debug)]
pub struct GlobalsArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 when any mutable global state is found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Maximum allowed mutable globals before failing (exit code 1).
    #[arg(long)]
    pub max_globals: Option<usize>,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
