use super::PathArgs;
use clap::Args;

/// Arguments for the `anti-patterns` detection subcommand.
#[derive(Args, Debug)]
pub struct AntiPatternsArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 when any anti-pattern is found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exit with code 1 when total anti-pattern count exceeds this limit.
    #[arg(long)]
    pub max_anti_patterns: Option<usize>,

    /// Exit with code 1 when magic number count exceeds this limit.
    #[arg(long)]
    pub max_magic_numbers: Option<usize>,

    /// Exit with code 1 when deeply nested callback count exceeds this limit.
    #[arg(long)]
    pub max_nested_callbacks: Option<usize>,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
