// src/main_cli.rs
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Command line interface configuration using `clap`.
/// This struct defines the arguments and flags accepted by the program.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// The subcommand to execute.
    #[command(subcommand)]
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

    /// Scan for API keys/secrets.
    #[arg(long)]
    pub secrets: bool,

    /// Scan for dangerous code.
    #[arg(long)]
    pub danger: bool,

    /// Scan for code quality issues.
    #[arg(long)]
    pub quality: bool,

    /// Enable taint analysis (data flow tracking).
    #[arg(long)]
    pub taint: bool,

    /// Output raw JSON.
    #[arg(long)]
    pub json: bool,

    /// Include test files in analysis.
    #[arg(long)]
    pub include_tests: bool,

    /// Folders to exclude from analysis.
    #[arg(long, alias = "exclude-folder")]
    pub exclude_folders: Vec<String>,

    /// Folders to force-include in analysis (overrides default exclusions).
    #[arg(long, alias = "include-folder")]
    pub include_folders: Vec<String>,

    /// Include `IPython` Notebooks (.ipynb files) in analysis.
    #[arg(long)]
    pub include_ipynb: bool,

    /// Report findings at cell level for notebooks.
    #[arg(long)]
    pub ipynb_cells: bool,
}

/// Helper enum for the available subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Calculate raw metrics (LOC, LLOC, SLOC, Comments, Multi, Blank)
    Raw {
        /// Path to analyze (optional, defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output JSON
        #[arg(long)]
        json: bool,

        /// Exclude folders
        #[arg(long, alias = "exclude-folder")]
        exclude: Vec<String>,
    },
    /// Calculate Cyclomatic Complexity
    Cc {
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
    /// Calculate Halstead Metrics
    Hal {
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
    /// Calculate Maintainability Index
    Mi {
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
