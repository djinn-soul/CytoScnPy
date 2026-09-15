use super::PathArgs;
use clap::Args;

/// Options for dependency analysis.
#[derive(Args, Debug)]
pub struct DepsArgs {
    /// Common options for paths.
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON.
    #[arg(long)]
    pub json: bool,

    /// Path to requirements file.
    #[arg(long)]
    pub requirements: Option<String>,

    /// Comma-separated list of packages to ignore if unused.
    #[arg(long = "ignore-unused", value_delimiter = ',')]
    pub ignore_unused: Vec<String>,

    /// Comma-separated list of packages to ignore if missing.
    #[arg(long = "ignore-missing", value_delimiter = ',')]
    pub ignore_missing: Vec<String>,

    /// Exclude folders.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Output file path.
    #[arg(long, short = 'O')]
    pub output_file: Option<String>,

    /// Show packages installed in the environment but not declared in the project.
    #[arg(long)]
    pub extra_installed: bool,

    /// Show orphan packages (installed, not declared, not imported, not required).
    #[arg(long)]
    pub orphans: bool,

    /// Include development dependencies in CSP-R002 findings.
    #[arg(long)]
    pub include_dev_unused: bool,

    /// Exit with code 1 if any dependency findings are found.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exit with code 1 if unused dependencies are found.
    #[arg(long)]
    pub fail_on_unused: bool,

    /// Exit with code 1 if missing dependencies are found.
    #[arg(long)]
    pub fail_on_missing: bool,

    /// Exit with code 1 if extra installed packages are found.
    #[arg(long)]
    pub fail_on_extra_installed: bool,

    /// Exit with code 1 if orphan packages are found.
    #[arg(long)]
    pub fail_on_orphans: bool,

    /// Show removal impact for a specific package (transitive deps that would also go).
    #[arg(long)]
    pub impact: Option<String>,

    /// Override the path to the virtual environment (default: auto-detect .venv).
    #[arg(long)]
    pub venv: Option<String>,

    /// Override the path to the lockfile (default: auto-detect uv.lock / poetry.lock).
    #[arg(long)]
    pub lockfile: Option<String>,
}
