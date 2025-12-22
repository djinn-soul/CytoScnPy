use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Help text for configuration file options, shown at the bottom of --help.
const CONFIG_HELP: &str = "\
CONFIGURATION FILE (.cytoscnpy.toml):
  Create this file in your project root to set defaults.

  [cytoscnpy]
  # Core settings
  confidence = 60            # Confidence threshold (0-100)
  secrets = true             # Enable secrets scanning
  danger = true              # Enable dangerous code scanning
  quality = true             # Enable quality checks
  include_tests = false      # Include test files in analysis
  include_ipynb = false      # Include Jupyter notebooks

  # Quality thresholds
  complexity = 10            # Max cyclomatic complexity
  nesting = 3                # Max nesting depth
  max_args = 5               # Max function arguments
  max_lines = 50             # Max function lines
  min_mi = 40.0              # Min Maintainability Index

  # Path filters
  exclude_folders = [\"build\", \"dist\", \".venv\"]
  include_folders = [\"src\"]  # Force-include these

  # CI/CD
  fail_threshold = 5.0       # Exit 1 if >N% unused code
";

/// Options for scan types (secrets, danger, quality).
#[derive(Args, Debug, Default, Clone)]
pub struct ScanOptions {
    /// Scan for API keys/secrets.
    #[arg(long)]
    pub secrets: bool,

    /// Scan for dangerous code (includes taint analysis).
    #[arg(long)]
    pub danger: bool,

    /// Scan for code quality issues.
    #[arg(long)]
    pub quality: bool,
}

/// Options for output formatting and verbosity.
#[derive(Args, Debug, Default, Clone)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are legitimately booleans
pub struct OutputOptions {
    /// Output raw JSON.
    #[arg(long)]
    pub json: bool,

    /// Enable verbose output for debugging (shows files being analyzed).
    #[arg(short, long)]
    pub verbose: bool,

    /// Quiet mode: show only summary, time, and gate results (no detailed tables).
    #[arg(short, long)]
    pub quiet: bool,

    /// Exit with code 1 if any quality issues are found.
    #[arg(long)]
    pub fail_on_quality: bool,

    /// Generate HTML report.
    #[arg(long)]
    #[cfg(feature = "html_report")]
    pub html: bool,
}

/// Options for including additional files in analysis.
#[derive(Args, Debug, Default, Clone)]
pub struct IncludeOptions {
    /// Include test files in analysis.
    #[arg(long)]
    pub include_tests: bool,

    /// Include `IPython` Notebooks (.ipynb files) in analysis.
    #[arg(long)]
    pub include_ipynb: bool,

    /// Report findings at cell level for notebooks.
    #[arg(long)]
    pub ipynb_cells: bool,
}

/// Command line interface configuration using `clap`.
/// This struct defines the arguments and flags accepted by the program.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, after_help = CONFIG_HELP)]
pub struct Cli {
    #[command(subcommand)]
    /// The subcommand to execute (e.g., raw, cc, hal).
    pub command: Option<Commands>,

    /// Paths to analyze (files or directories).
    /// Can be a single directory, multiple files, or a mix of both.
    /// When no paths are provided, defaults to the current directory.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

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
}

#[derive(Subcommand, Debug)]
/// Available subcommands for specific metric calculations.
pub enum Commands {
    /// Calculate raw metrics (LOC, LLOC, SLOC, Comments, Multi, Blank)
    Raw {
        /// Path to analyze (optional, defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output JSON
        #[arg(long, short = 'j')]
        json: bool,

        /// Exclude folders
        #[arg(long, short = 'e', alias = "exclude-folder")]
        exclude: Vec<String>,

        /// Ignore directories matching glob pattern
        #[arg(long, short = 'i')]
        ignore: Vec<String>,

        /// Show summary of gathered metrics
        #[arg(long, short = 's')]
        summary: bool,

        /// Save output to file
        #[arg(long, short = 'O')]
        output_file: Option<String>,
    },
    /// Calculate Cyclomatic Complexity
    Cc {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output JSON
        #[arg(long, short = 'j')]
        json: bool,

        /// Exclude folders
        #[arg(long, short = 'e', alias = "exclude-folder")]
        exclude: Vec<String>,

        /// Ignore directories matching glob pattern
        #[arg(long, short = 'i')]
        ignore: Vec<String>,

        /// Set minimum complexity rank (A-F)
        #[arg(long, short = 'n', alias = "min")]
        min_rank: Option<char>,

        /// Set maximum complexity rank (A-F)
        #[arg(long, short = 'x', alias = "max")]
        max_rank: Option<char>,

        /// Show average complexity
        #[arg(long, short = 'a')]
        average: bool,

        /// Show total average complexity
        #[arg(long)]
        total_average: bool,

        /// Show complexity score with rank
        #[arg(long, short = 's')]
        show_complexity: bool,

        /// Ordering function (score, lines, alpha)
        #[arg(long, short = 'o')]
        order: Option<String>,

        /// Do not count assert statements
        #[arg(long)]
        no_assert: bool,

        /// Output XML
        #[arg(long)]
        xml: bool,

        /// Exit with code 1 if any block has complexity higher than this value
        #[arg(long)]
        fail_threshold: Option<usize>,

        /// Save output to file
        #[arg(long, short = 'O')]
        output_file: Option<String>,
    },
    /// Calculate Halstead Metrics
    Hal {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output JSON
        #[arg(long, short = 'j')]
        json: bool,

        /// Exclude folders
        #[arg(long, short = 'e', alias = "exclude-folder")]
        exclude: Vec<String>,

        /// Ignore directories matching glob pattern
        #[arg(long, short = 'i')]
        ignore: Vec<String>,

        /// Compute metrics on function level
        #[arg(long, short = 'f')]
        functions: bool,

        /// Save output to file
        #[arg(long, short = 'O')]
        output_file: Option<String>,
    },
    /// Calculate Maintainability Index
    Mi {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output JSON
        #[arg(long, short = 'j')]
        json: bool,

        /// Exclude folders
        #[arg(long, short = 'e', alias = "exclude-folder")]
        exclude: Vec<String>,

        /// Ignore directories matching glob pattern
        #[arg(long, short = 'i')]
        ignore: Vec<String>,

        /// Set minimum MI rank (A-C)
        #[arg(long, short = 'n', alias = "min")]
        min_rank: Option<char>,

        /// Set maximum MI rank (A-C)
        #[arg(long, short = 'x', alias = "max")]
        max_rank: Option<char>,

        /// Count multiline strings as comments (enabled by default)
        #[arg(long, short = 'm', default_value = "true", action = clap::ArgAction::Set)]
        multi: bool,

        /// Show actual MI value
        #[arg(long, short = 's')]
        show: bool,

        /// Show average MI
        #[arg(long, short = 'a')]
        average: bool,

        /// Exit with code 1 if any file has MI lower than this value
        #[arg(long)]
        fail_threshold: Option<f64>,

        /// Save output to file
        #[arg(long, short = 'O')]
        output_file: Option<String>,
    },
    /// Start MCP server for LLM integration (Claude Desktop, VS Code Copilot, etc.)
    #[command(name = "mcp-server")]
    McpServer,
    /// Generate comprehensive project statistics report
    Stats {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Enable all analysis: secrets, danger, quality, and per-file metrics
        #[arg(long, short = 'a')]
        all: bool,

        /// Scan for API keys/secrets
        #[arg(long, short = 's')]
        secrets: bool,

        /// Scan for dangerous code patterns
        #[arg(long, short = 'd')]
        danger: bool,

        /// Scan for code quality issues
        #[arg(long, short = 'q')]
        quality: bool,

        /// Output JSON
        #[arg(long)]
        json: bool,

        /// Output file path
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Exclude folders
        #[arg(long, alias = "exclude-folder")]
        exclude: Vec<String>,
    },
    /// Show per-file metrics table
    Files {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output JSON
        #[arg(long)]
        json: bool,

        /// Exclude folders
        #[arg(long, alias = "exclude-folder")]
        exclude: Vec<String>,
    },
}
