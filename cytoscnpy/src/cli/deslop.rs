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
    /// Output one JSON report.
    #[arg(long)]
    pub json: bool,
    /// Output report file.
    #[arg(long, short = 'o')]
    pub output: Option<String>,
    /// Exclude folders.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,
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
