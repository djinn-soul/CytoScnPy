use super::PathArgs;
use clap::Args;

/// Arguments for the `doctor` repository health subcommand.
#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Path options (paths vs root).
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON format.
    #[arg(long)]
    pub json: bool,

    /// Exit with code 1 if critical or recommended setup items are missing or failing.
    #[arg(long)]
    pub fail_on_missing: bool,

    /// Exclude folders.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'o')]
    pub output_file: Option<String>,
}
