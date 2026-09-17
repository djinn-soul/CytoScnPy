use super::PathArgs;
use clap::Args;

/// Arguments for the `graph` module architecture and dependency subcommand.
#[derive(Args, Debug)]
pub struct GraphArgs {
    /// Path options (paths vs root).
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Only show circular dependency cycles.
    #[arg(long)]
    pub cycles_only: bool,

    /// Exit with code 1 if any circular dependencies are detected.
    #[arg(long)]
    pub fail_on_cycles: bool,

    /// Exit with code 1 if any god modules are detected.
    #[arg(long)]
    pub fail_on_god_modules: bool,

    /// Exclude folders.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
