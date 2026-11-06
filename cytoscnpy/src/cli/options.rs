use clap::Args;
use std::path::PathBuf;

/// Options for scan types (secrets, danger, quality).
#[derive(Args, Debug, Default, Clone)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are legitimately booleans
pub struct ScanOptions {
    /// Scan for API keys/secrets.
    #[arg(short = 's', long, overrides_with = "no_secrets")]
    pub secrets: bool,

    /// Disable the secrets scan, even when project configuration enables it.
    #[arg(long, overrides_with = "secrets")]
    pub no_secrets: bool,

    /// Scan for dangerous code (includes taint analysis).
    #[arg(short = 'd', long, overrides_with = "no_danger")]
    pub danger: bool,

    /// Disable the danger scan, even when project configuration enables it.
    #[arg(long, overrides_with = "danger")]
    pub no_danger: bool,

    /// Scan for code quality issues.
    #[arg(short = 'q', long, overrides_with = "no_quality")]
    pub quality: bool,

    /// Disable the quality scan, even when project configuration enables it.
    #[arg(long, overrides_with = "quality")]
    pub no_quality: bool,

    /// Skip dead code detection (only run security/quality scans).
    #[arg(short = 'n', long = "no-dead")]
    pub no_dead: bool,

    /// Analyze unused and missing dependencies (reads pyproject.toml / requirements.txt).
    #[arg(long)]
    pub deps: bool,
}

/// Supported output formats for scan results.
#[derive(Debug, Clone, clap::ValueEnum, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Standard plain text table.
    #[default]
    Text,
    /// Raw JSON format.
    Json,
    /// Grouped findings (deprecated, use Text instead).
    Grouped,
    /// `JUnit` XML format for CI/CD.
    Junit,
    /// `GitHub` Annotations (via workflow commands).
    Github,
    /// `GitLab` Code Quality JSON.
    Gitlab,
    /// Markdown document.
    Markdown,
    /// SARIF (Static Analysis Results Interchange Format).
    Sarif,
}

/// Supported editor/automation clients.
#[derive(Debug, Clone, clap::ValueEnum, PartialEq, Eq)]
pub enum ClientKind {
    /// Visual Studio Code extension.
    Vscode,
}

/// Options for output formatting and verbosity.
#[derive(Args, Debug, Default, Clone)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are legitimately booleans
pub struct OutputOptions {
    /// Output raw JSON.
    #[arg(long)]
    pub json: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Enable verbose output for debugging (shows files being analyzed).
    #[arg(short, long)]
    pub verbose: bool,

    /// Quiet mode: show only summary, time, and gate results (no detailed tables).
    #[arg(long)]
    pub quiet: bool,

    /// Exit with code 1 if any supported gate finds actionable issues.
    #[arg(long)]
    pub fail_on_any: bool,

    /// Exit with code 1 if any quality issues are found.
    #[arg(long)]
    pub fail_on_quality: bool,

    /// Exit with code 1 if any secret findings are found.
    #[arg(long)]
    pub fail_on_secrets: bool,

    /// Exit with code 1 if any danger or taint findings are found.
    #[arg(long)]
    pub fail_on_danger: bool,

    /// Exit with code 1 if any missing dependency findings are found.
    #[arg(long)]
    pub fail_on_missing_deps: bool,

    /// Exit with code 1 if any unused dependency findings are found.
    #[arg(long)]
    pub fail_on_unused_deps: bool,

    /// Generate HTML report.
    #[arg(long)]
    #[cfg(feature = "html_report")]
    pub html: bool,

    /// Default color theme for the HTML report (viewers can still toggle it).
    #[arg(long, value_enum, default_value_t, requires = "html")]
    #[cfg(feature = "html_report")]
    pub html_theme: crate::report::generator::ReportTheme,
}

/// Options for including additional files in analysis.
#[derive(Args, Debug, Default, Clone)]
pub struct IncludeOptions {
    /// Include test files in analysis.
    #[arg(long, overrides_with = "no_include_tests")]
    pub include_tests: bool,

    /// Exclude test files, even when project configuration includes them.
    #[arg(long, overrides_with = "include_tests")]
    pub no_include_tests: bool,

    /// Include `IPython` Notebooks (.ipynb files) in analysis.
    #[arg(long, overrides_with = "no_include_ipynb")]
    pub include_ipynb: bool,

    /// Skip notebooks, even when project configuration includes them.
    #[arg(long, overrides_with = "include_ipynb")]
    pub no_include_ipynb: bool,

    /// Report findings at cell level for notebooks.
    #[arg(long)]
    pub ipynb_cells: bool,
}

/// Shared path arguments (mutually exclusive paths/root).
#[derive(Args, Debug, Default, Clone)]
pub struct PathArgs {
    /// Paths to analyze (files or directories).
    /// Can be a single directory, multiple files, or a mix of both.
    /// When no paths are provided, defaults to the current directory.
    /// Cannot be used with --root.
    #[arg(conflicts_with = "root")]
    pub paths: Vec<PathBuf>,

    /// Project root for path containment and analysis.
    /// Use this instead of positional paths when running from a different directory.
    /// When specified, this path is used as both the analysis target AND the
    /// security containment boundary for file operations.
    /// Cannot be used together with positional path arguments.
    #[arg(long, conflicts_with = "paths")]
    pub root: Option<PathBuf>,
}

/// Common options for metric subcommands (cc, hal, mi, raw).
/// Use `#[command(flatten)]` to include these in a subcommand.
#[derive(Args, Debug, Default, Clone)]
pub struct MetricArgs {
    /// Path options (paths vs root).
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON.
    #[arg(long, short = 'j')]
    pub json: bool,

    /// Exclude folders.
    #[arg(long, short = 'e', alias = "exclude-folder")]
    pub exclude: Vec<String>,

    /// Ignore directories matching glob pattern.
    #[arg(long, short = 'i')]
    pub ignore: Vec<String>,

    /// Save output to file.
    #[arg(long, short = 'O')]
    pub output_file: Option<String>,
}

/// Rank filtering options (A-F grades) for complexity/MI commands.
#[derive(Args, Debug, Default, Clone, Copy)]
pub struct RankArgs {
    /// Set minimum rank (A-F or A-C depending on command).
    #[arg(long, short = 'n', alias = "min")]
    pub min_rank: Option<char>,

    /// Set maximum rank (A-F or A-C depending on command).
    #[arg(long, short = 'x', alias = "max")]
    pub max_rank: Option<char>,
}

/// Options for the projects subcommand.
#[derive(Args, Debug, Default, Clone)]
pub struct ProjectsArgs {
    /// Path options (paths vs root).
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON.
    #[arg(long)]
    pub json: bool,
}

/// Common options for the files subcommand.
#[derive(Args, Debug, Default, Clone)]
pub struct FilesArgs {
    /// Path options (paths vs root).
    #[command(flatten)]
    pub paths: PathArgs,

    /// Output JSON.
    #[arg(long)]
    pub json: bool,

    /// Exclude folders.
    #[arg(long, alias = "exclude-folder")]
    pub exclude: Vec<String>,
}
