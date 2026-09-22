use super::PathArgs;
use clap::Args;

/// Arguments for the `functions` metric extraction subcommand.
#[derive(Args, Debug)]
pub struct FunctionsArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Filter to functions with cyclomatic complexity >= this value.
    #[arg(long)]
    pub min_complexity: Option<usize>,

    /// Filter to functions with line count >= this value.
    #[arg(long)]
    pub min_lines: Option<usize>,

    /// Filter to functions with nesting depth >= this value.
    #[arg(long)]
    pub min_nesting: Option<usize>,

    /// Exit with code 1 when any function exceeds this complexity threshold.
    #[arg(long)]
    pub max_complexity: Option<usize>,

    /// Exit with code 1 when any function exceeds this line threshold.
    #[arg(long)]
    pub max_lines: Option<usize>,

    /// Exit with code 1 when any function exceeds this nesting threshold.
    #[arg(long)]
    pub max_nesting: Option<usize>,

    /// Fail if any function exceeds standard quality thresholds (complexity > 10, lines > 50, nesting > 3).
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Patterns or directory names to ignore during analysis (repeatable).
    #[arg(long, short = 'i')]
    pub ignore: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
