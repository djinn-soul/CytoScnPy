//! Main binary entry point for the `CytoScnPy` static analysis tool.
//!
//! This binary simply delegates to the shared `entry_point::run_with_args()` function
//! to ensure consistent behavior across all entry points (CLI, Python bindings, etc.)

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Command line interface configuration using `clap`.
/// This struct defines the arguments and flags accepted by the program.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    /// The subcommand to execute (e.g., raw, cc, hal).
    command: Option<Commands>,

    /// Paths to analyze (files or directories).
    /// Can be a single directory, multiple files, or a mix of both.
    /// When no paths are provided, defaults to the current directory.
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,

    /// Confidence threshold (0-100).
    /// Only findings with confidence higher than this value will be reported.
    #[arg(short, long)]
    confidence: Option<u8>,

    /// Scan for API keys/secrets.
    #[arg(long)]
    secrets: bool,

    /// Scan for dangerous code.
    #[arg(long)]
    danger: bool,

    /// Scan for code quality issues.
    #[arg(long)]
    quality: bool,

    /// Enable taint analysis (data flow tracking).
    #[arg(long)]
    taint: bool,

    /// Output raw JSON.
    #[arg(long)]
    json: bool,

    /// Include test files in analysis.
    #[arg(long)]
    include_tests: bool,

    /// Folders to exclude from analysis.
    #[arg(long, alias = "exclude-folder")]
    exclude_folders: Vec<String>,

    /// Folders to force-include in analysis (overrides default exclusions).
    #[arg(long, alias = "include-folder")]
    include_folders: Vec<String>,

    /// Include `IPython` Notebooks (.ipynb files) in analysis.
    #[arg(long)]
    include_ipynb: bool,

    /// Report findings at cell level for notebooks.
    #[arg(long)]
    ipynb_cells: bool,

    /// Exit with code 1 if finding percentage exceeds this threshold (0-100).
    /// For CI/CD integration: --fail-under 5 fails if >5% of definitions are unused.
    #[arg(long)]
    fail_under: Option<f64>,

    /// Set maximum allowed Cyclomatic Complexity (overrides config).
    /// Findings with complexity > N will be reported.
    #[arg(long)]
    max_complexity: Option<usize>,

    /// Set minimum allowed Maintainability Index.
    /// Files with MI < N will be reported.
    #[arg(long)]
    min_mi: Option<f64>,

    /// Exit with code 1 if any quality issues are found.
    #[arg(long)]
    fail_on_quality: bool,
}

#[derive(Subcommand)]
/// Available subcommands for specific metric calculations.
enum Commands {
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

        /// Do not count multiline strings as comments
        #[arg(long, short = 'm')]
        multi: bool,

        /// Show actual MI value
        #[arg(long, short = 's')]
        show: bool,

        /// Show average MI
        #[arg(long, short = 'a')]
        average: bool,

        /// Exit with code 1 if any file has MI lower than this value
        #[arg(long)]
        fail_under: Option<f64>,

        /// Save output to file
        #[arg(long, short = 'O')]
        output_file: Option<String>,
    },
    /// Generate comprehensive project statistics report
    Stats {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Enable all analysis: secrets, danger, quality, and per-file metrics
        #[arg(long)]
        all: bool,

        /// Scan for API keys/secrets
        #[arg(long)]
        secrets: bool,

        /// Scan for dangerous code patterns
        #[arg(long)]
        danger: bool,

        /// Scan for code quality issues
        #[arg(long)]
        quality: bool,

        /// Output JSON instead of markdown
        #[arg(long, short = 'j')]
        json: bool,

        /// Output file path (default: `stats_report.md` or `stats_report.json`)
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Exclude folders
        #[arg(long, short = 'e', alias = "exclude-folder")]
        exclude: Vec<String>,
    },
    /// Show per-file metrics table (code, comments, empty lines, size)
    Files {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output JSON
        #[arg(long, short = 'j')]
        json: bool,

        /// Exclude folders
        #[arg(long, short = 'e', alias = "exclude-folder")]
        exclude: Vec<String>,
    },
}

fn main() -> Result<()> {
    // Delegate CLI args to shared entry_point function (same as cytoscnpy-cli and Python)
    let code = cytoscnpy::entry_point::run_with_args(std::env::args().skip(1).collect())?;
    std::process::exit(code);
}
