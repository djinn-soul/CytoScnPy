use super::PathArgs;
use clap::Args;

/// Arguments for the `unreferenced` large functions in isolated files subcommand.
#[derive(Args, Debug)]
pub struct UnreferencedArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 when any unreferenced large function is found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exit with code 1 when unreferenced function count exceeds this limit.
    #[arg(long)]
    pub max_unreferenced: Option<usize>,

    /// Exit with code 1 when total unreferenced lines exceed this limit.
    #[arg(long)]
    pub max_unreferenced_lines: Option<usize>,

    /// Minimum physical lines for a function to be considered large (default: 15).
    #[arg(long)]
    pub min_lines: Option<usize>,

    /// Include test files in isolation and unreferenced analysis.
    #[arg(long)]
    pub include_tests: bool,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
