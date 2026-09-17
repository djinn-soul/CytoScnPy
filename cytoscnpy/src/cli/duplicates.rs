use super::PathArgs;
use clap::Args;

/// Arguments for the `duplicates` code clusters and non-overlapping line totals subcommand.
#[derive(Args, Debug)]
pub struct DuplicatesArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 when any duplicate code cluster is found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exit with code 1 when duplicate cluster count exceeds this limit.
    #[arg(long)]
    pub max_clusters: Option<usize>,

    /// Exit with code 1 when total non-overlapping duplicate lines exceed this limit.
    #[arg(long)]
    pub max_duplicate_lines: Option<usize>,

    /// Exit with code 1 when duplication percentage exceeds this limit (e.g. 5.0 for 5%).
    #[arg(long)]
    pub max_duplicate_pct: Option<f64>,

    /// Minimum similarity threshold (0.0 - 1.0, default: 0.85).
    #[arg(long)]
    pub min_similarity: Option<f64>,

    /// Minimum line threshold for code fragments (default: 4).
    #[arg(long)]
    pub min_lines: Option<usize>,

    /// Include test files in duplication analysis.
    #[arg(long)]
    pub include_tests: bool,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
