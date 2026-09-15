use super::PathArgs;
use clap::Args;

/// Arguments for the `todos` annotation and debug-print detection subcommand.
#[derive(Args, Debug)]
pub struct TodosArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 when any TODO/FIXME/HACK/XXX or debug print is found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
