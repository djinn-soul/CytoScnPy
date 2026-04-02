mod commands;
mod options;

pub use commands::Commands;
pub use options::{
    ClientKind, FilesArgs, IncludeOptions, MetricArgs, OutputFormat, OutputOptions, PathArgs,
    RankArgs, ScanOptions,
};

use clap::Parser;
use std::path::PathBuf;

/// Help text for configuration file options, shown at the bottom of --help.
const CONFIG_HELP: &str = "\
CONFIGURATION FILE
  Supported locations (highest priority first):
    .cytoscnpy.toml          [cytoscnpy] table
    pyproject.toml           [tool.cytoscnpy] table

  Run `cytoscnpy init` to scaffold a fully-commented config file.

  [cytoscnpy]

  # Minimum confidence score (0-100) a finding must reach before it is reported.
  # confidence = 60

  # Scan for hard-coded secrets and high-entropy strings (API keys, tokens, etc.).
  # secrets = true

  # Scan for dangerous code patterns: SQL injection, XSS, command injection, etc.
  # danger = true

  # Report code-quality issues: high complexity, deep nesting, long functions, etc.
  # quality = true

  # Include pytest/unittest test files in the analysis.
  # include_tests = false

  # Include Jupyter notebook (.ipynb) cells in the analysis.
  # include_ipynb = false

  # \"library\" - public symbols treated as exported API (fewer false positives).
  # \"application\" - stricter dead-code detection for app-style projects.
  # project_type = \"library\"

  # McCabe complexity limit per function. Exceeding this is a quality finding.
  # max_complexity = 10

  # Maximum nesting depth (if/for/while/try) before a finding is emitted.
  # max_nesting = 3

  # Maximum number of parameters a function may have.
  # max_args = 5

  # Maximum number of lines a function body may span.
  # max_lines = 50

  # Minimum Maintainability Index (0-100). Below this score is flagged.
  # min_mi = 40.0

  # Directories to skip entirely during analysis.
  # exclude_folders = [\"build\", \"dist\", \".venv\", \"__pycache__\"]

  # Directories to force-include even when git-ignored.
  # include_folders = [\"src\"]

  # Rule IDs to silence globally across the entire project.
  # ignore = [\"CSP-P003\"]

  # Silence specific rules only for files matching a glob pattern.
  # per-file-ignores = { \"tests/*\" = [\"CSP-D701\"] }

  # Detect Type-1/2/3 duplicate code blocks across the project.
  # clones = false

  # How similar two blocks must be (0.0-1.0) to be reported as a clone pair.
  # clone_similarity = 0.8

  # Exit with code 1 when unused-definition percentage exceeds this value.
  # fail_threshold = 5.0

  Advanced subsections (run `cytoscnpy init` or see docs/CLI.md for full options):
    [cytoscnpy.secrets_config]   entropy tuning, custom patterns
    [cytoscnpy.danger_config]    taint sources/sinks/sanitizers
    [[cytoscnpy.whitelist]]      inline dead-code suppressions
";

/// Command line interface configuration using `clap`.
/// This struct defines the arguments and flags accepted by the program.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "CytoScnPy - Fast, accurate Python static analysis for dead code, secrets, and quality issues",
    long_about = None,
    after_help = CONFIG_HELP
)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are legitimately booleans
pub struct Cli {
    #[command(subcommand)]
    /// The subcommand to execute (e.g., raw, cc, hal).
    pub command: Option<Commands>,

    /// Global path options (paths vs root).
    #[command(flatten)]
    pub paths: PathArgs,

    /// Confidence threshold (0-100).
    /// Only findings with confidence higher than this value will be reported.
    #[arg(short, long)]
    pub confidence: Option<u8>,

    /// Scan type options (secrets, danger, quality).
    #[command(flatten)]
    pub scan: ScanOptions,

    /// Output formatting options.
    #[command(flatten)]
    pub output: OutputOptions,

    /// Identify the editor/client integration (currently only `vscode`).
    #[arg(long, value_enum)]
    pub client: Option<ClientKind>,

    /// Include options for additional file types.
    #[command(flatten)]
    pub include: IncludeOptions,

    /// Folders to exclude from analysis.
    #[arg(long, alias = "exclude-folder")]
    pub exclude_folders: Vec<String>,

    /// Folders to force-include in analysis (overrides default exclusions).
    #[arg(long, alias = "include-folder")]
    pub include_folders: Vec<String>,

    /// Exit with code 1 if finding percentage exceeds this threshold (0-100).
    /// For CI/CD integration: --fail-threshold 5 fails if >5% of definitions are unused.
    #[arg(long)]
    pub fail_threshold: Option<f64>,

    /// Set maximum allowed Cyclomatic Complexity (overrides config).
    /// Findings with complexity > N will be reported.
    #[arg(long)]
    pub max_complexity: Option<usize>,

    /// Set minimum allowed Maintainability Index.
    /// Files with MI < N will be reported.
    #[arg(long)]
    pub min_mi: Option<f64>,

    /// Set maximum allowed nesting depth.
    #[arg(long)]
    pub max_nesting: Option<usize>,

    /// Set maximum allowed function arguments.
    #[arg(long)]
    pub max_args: Option<usize>,

    /// Set maximum allowed function lines.
    #[arg(long)]
    pub max_lines: Option<usize>,

    /// Add artificial delay (ms) per file for testing progress bar.
    #[arg(long, hide = true)]
    pub debug_delay: Option<u64>,

    /// Enable code clone detection (Type-1/2/3).
    /// Finds duplicate or near-duplicate code fragments.
    #[arg(long)]
    pub clones: bool,

    /// Minimum similarity threshold for clone detection (0.0-1.0).
    /// Default is 0.8 (80% similarity). Overrides the config file value.
    #[arg(long)]
    pub clone_similarity: Option<f64>,

    /// Auto-fix detected dead code (removes unused functions, classes, imports,
    /// and renames unused variables to `_`).
    /// By default, shows a preview of what would be changed (dry-run).
    /// Use --apply to actually modify files.
    /// Note: Clone detection is report-only; clones are never auto-removed.
    #[arg(long)]
    pub fix: bool,

    /// Apply the fixes to files (use with --fix).
    /// Without this flag, --fix only shows a preview of what would be changed.
    #[arg(short = 'a', long)]
    pub apply: bool,

    /// Generate a whitelist file from detected unused code.
    /// Outputs valid Python syntax that can be used to suppress false positives.
    /// The whitelist can be added to your project and scanned alongside your code.
    /// Example: cytoscnpy src/ --make-whitelist > whitelist.py
    #[arg(long)]
    pub make_whitelist: bool,

    /// Path to an existing whitelist file to load.
    /// The whitelist can be a Python file (like Vulture's format) or a TOML file.
    /// Multiple whitelist files can be specified.
    #[arg(long = "whitelist")]
    pub whitelist_files: Vec<PathBuf>,
}
