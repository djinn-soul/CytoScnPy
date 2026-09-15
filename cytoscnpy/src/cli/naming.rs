use super::PathArgs;
use clap::Args;

/// Arguments for identifier naming style distribution and consistency analysis.
#[derive(Args, Debug)]
pub struct NamingArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Minimum required naming consistency ratio (e.g. 0.85 for 85%).
    #[arg(long)]
    pub min_consistency: Option<f64>,

    /// Exit with code 1 if naming consistency falls below the threshold (default: 0.85 or min-consistency).
    #[arg(long, visible_alias = "fail-on-any")]
    pub fail_on_inconsistent: bool,

    /// Exclude folders.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
