use super::PathArgs;
use clap::Args;

/// Arguments for the `side-effects` module-level side-effect detection subcommand.
#[derive(Args, Debug)]
pub struct SideEffectsArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 when any module-level side effect is found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exit with code 1 when side effect count exceeds this limit.
    #[arg(long)]
    pub max_side_effects: Option<usize>,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
