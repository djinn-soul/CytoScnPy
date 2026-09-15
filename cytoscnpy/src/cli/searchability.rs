use super::PathArgs;
use clap::Args;

/// Arguments for codebase searchability and name collision analysis.
#[derive(Args, Debug)]
pub struct SearchabilityArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 if any function collisions (defined in >= 3 distinct files) are detected.
    #[arg(long)]
    pub fail_on_collisions: bool,

    /// Exit with code 1 if any duplicate filenames are detected.
    #[arg(long)]
    pub fail_on_duplicates: bool,

    /// Exit with code 1 if any searchability issue (duplicate filenames or function collisions) is detected.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exclude folders.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
