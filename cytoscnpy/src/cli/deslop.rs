use clap::Args;

/// Options for the unified DeSlopify-compatible assessment.
#[derive(Args, Debug)]
pub struct DeslopArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: super::PathArgs,
    /// Fail if any configured `DeSlopify` gate fails.
    #[arg(long)]
    pub fail_on_any: bool,
    /// Fail when the Slop Index exceeds the configured or default CI ceiling.
    #[arg(long)]
    pub ci: bool,
    /// Maximum allowed Slop Index (0-100).
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=100))]
    pub max_score: Option<u32>,
    /// Output one JSON report.
    #[arg(long)]
    pub json: bool,
    /// Output format: terminal, JSON, or LLM remediation instructions.
    #[arg(long, value_parser = ["terminal", "json", "llm"])]
    pub format: Option<String>,
    /// Show the full analysis report and detailed scoring diagnostics.
    #[arg(long, short = 'v')]
    pub verbose: bool,
    /// Output report file.
    #[arg(long, short = 'o')]
    pub output: Option<String>,
    /// Exclude folders.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,
    /// Patterns or directory names to ignore during analysis (repeatable).
    #[arg(long, short = 'i')]
    pub ignore: Vec<String>,
    /// Disable Git history analysis.
    #[arg(long)]
    pub no_git: bool,
    /// Maximum Git lookback in months.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub git_months: Option<u32>,
    /// Usable LLM context capacity in tokens.
    #[arg(long, default_value_t = 176_000)]
    pub context_budget: usize,
}
