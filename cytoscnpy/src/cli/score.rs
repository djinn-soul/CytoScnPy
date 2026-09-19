use clap::Args;

use super::options::PathArgs;

/// Command-line arguments for the Slop Index calculation and scoring model.
#[derive(Args, Debug, Clone)]
pub struct ScoreArgs {
    /// Path options (paths vs root).
    #[command(flatten)]
    pub paths: PathArgs,
    /// Maximum allowed Slop Index (0-100) before exiting with failure code 1.
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=100))]
    pub max_score: Option<u32>,
    /// CI mode: exit code 1 if slop index exceeds acceptable verdict or max-score gate.
    #[arg(long)]
    pub ci: bool,
    /// Fail if any slop dimension rating is non-zero (strictest threshold).
    #[arg(long)]
    pub fail_on_any: bool,
    /// Output score and dimension breakdown in JSON format.
    #[arg(long)]
    pub json: bool,
    /// Output format: "terminal" (default), "json", or "llm".
    #[arg(long, value_parser = ["terminal", "json", "llm"])]
    pub format: Option<String>,
    /// Disable Git history analysis for size multiplier.
    #[arg(long)]
    pub no_git: bool,
    /// Lookback window in months for Git activity (default: auto-scaled to repo age).
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub git_months: Option<u32>,
    /// Usable LLM context capacity in tokens (default: 176000).
    #[arg(long, default_value_t = 176_000)]
    pub context_budget: usize,
    /// Exclude folders matching names.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,
    /// Patterns or directory names to ignore during analysis (repeatable).
    #[arg(long, short = 'i')]
    pub ignore: Vec<String>,
    /// Show detailed metrics and all scoring dimensions.
    #[arg(long, short = 'v')]
    pub verbose: bool,
    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
