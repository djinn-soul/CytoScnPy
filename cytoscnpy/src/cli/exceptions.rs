use super::PathArgs;
use clap::Args;

/// Arguments for the `exceptions` bare-except and empty-handler detection subcommand.
#[derive(Args, Debug)]
pub struct ExceptionsArgs {
    /// Files or directories to analyze.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 when any bare-except or empty handler is found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exit with code 1 when bare-except count exceeds this limit.
    #[arg(long)]
    pub max_bare_excepts: Option<usize>,

    /// Exit with code 1 when empty-handler count exceeds this limit.
    #[arg(long)]
    pub max_empty_handlers: Option<usize>,

    /// Exclude folders or patterns.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
