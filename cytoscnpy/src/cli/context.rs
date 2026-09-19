use clap::Args;

use super::options::PathArgs;

/// Command-line arguments for Git activity, churn, and token budget context analysis.
#[derive(Args, Debug, Clone)]
pub struct ContextArgs {
    /// Path options (paths vs root).
    #[command(flatten)]
    pub paths: PathArgs,
    /// Lookback window in months for Git activity (default: auto-scaled to repo age).
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub git_months: Option<u32>,
    /// Usable LLM context capacity in tokens (default: 176000).
    #[arg(long)]
    pub context_budget: Option<usize>,
    /// Disable Git history analysis (pure static context estimation).
    #[arg(long)]
    pub no_git: bool,
    /// Output JSON format.
    #[arg(long)]
    pub json: bool,
    /// Only show churn-complexity hotspot files.
    #[arg(long)]
    pub hotspots_only: bool,
    /// Exit with code 1 if severe (Critical or High) hotspots are detected.
    #[arg(long)]
    pub fail_on_hotspots: bool,
    /// Exclude folders.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,
    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
