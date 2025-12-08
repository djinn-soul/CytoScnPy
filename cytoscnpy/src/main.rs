//! Main binary entry point for the `CytoScnPy` static analysis tool.

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::commands::{run_cc, run_hal, run_mi, run_raw};
use cytoscnpy::config::Config;

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
}

/// Main entry point of the application.
fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle subcommands
    if let Some(command) = cli.command {
        let mut stdout = std::io::stdout();
        match command {
            Commands::Raw {
                path,
                json,
                exclude,
                ignore,
                summary,
                output_file,
            } => run_raw(
                path,
                json,
                exclude,
                ignore,
                summary,
                output_file,
                &mut stdout,
            ),
            Commands::Cc {
                path,
                json,
                exclude,
                ignore,
                min_rank,
                max_rank,
                average,
                total_average,
                show_complexity,
                order,
                no_assert,
                xml,
                fail_threshold,
                output_file,
            } => run_cc(
                path,
                json,
                exclude,
                ignore,
                min_rank,
                max_rank,
                average,
                total_average,
                show_complexity,
                order,
                no_assert,
                xml,
                fail_threshold,
                output_file,
                &mut stdout,
            ),
            Commands::Hal {
                path,
                json,
                exclude,
                ignore,
                functions,
                output_file,
            } => run_hal(
                path,
                json,
                exclude,
                ignore,
                functions,
                output_file,
                &mut stdout,
            ),
            Commands::Mi {
                path,
                json,
                exclude,
                ignore,
                min_rank,
                max_rank,
                multi,
                show,
                average,
                fail_under,
                output_file,
            } => run_mi(
                path,
                json,
                exclude,
                ignore,
                min_rank,
                max_rank,
                multi,
                show,
                average,
                fail_under,
                output_file,
                &mut stdout,
            ),
        }
    } else {
        // Default behavior: Run full analysis

        // Load configuration from .cytoscnpy.toml if present
        // Use the first path for config discovery, or current dir if none provided
        let config_path = cli
            .paths
            .first()
            .map_or(std::path::Path::new("."), std::path::PathBuf::as_path);
        let config = Config::load_from_path(config_path);

        // Merge CLI arguments with config values (CLI takes precedence if provided)
        let confidence = cli.confidence.or(config.cytoscnpy.confidence).unwrap_or(60);
        let secrets = cli.secrets || config.cytoscnpy.secrets.unwrap_or(false);
        let danger = cli.danger || config.cytoscnpy.danger.unwrap_or(false);
        let mut include_folders = config.cytoscnpy.include_folders.clone().unwrap_or_default();
        include_folders.extend(cli.include_folders);

        // Update config with CLI quality thresholds if provided
        let mut config = config;
        if let Some(c) = cli.max_complexity {
            config.cytoscnpy.complexity = Some(c);
        }
        if let Some(m) = cli.min_mi {
            config.cytoscnpy.min_mi = Some(m);
        }
        // Force enable quality scan if quality arguments are provided
        let quality = cli.quality
            || config.cytoscnpy.quality.unwrap_or(false)
            || cli.max_complexity.is_some()
            || cli.min_mi.is_some();

        // Update config with CLI quality thresholds if provided
        let include_tests = cli.include_tests || config.cytoscnpy.include_tests.unwrap_or(false);

        let mut exclude_folders = config.cytoscnpy.exclude_folders.clone().unwrap_or_default();
        exclude_folders.extend(cli.exclude_folders);

        // Print styled exclusion list before analysis (like Python version)
        if !cli.json {
            let mut stdout = std::io::stdout();
            cytoscnpy::output::print_exclusion_list(&mut stdout, &exclude_folders).ok();
        }

        // Create spinner for visual feedback during analysis
        let spinner = if cli.json {
            None
        } else {
            Some(cytoscnpy::output::create_spinner())
        };

        let mut analyzer = CytoScnPy::new(
            confidence,
            secrets,
            danger,
            quality,
            include_tests,
            exclude_folders,
            include_folders,
            cli.include_ipynb,
            cli.ipynb_cells,
            cli.danger || cli.taint, // taint enabled with --danger or --taint
            config,
        );
        let result = analyzer.analyze_paths(&cli.paths)?;

        if let Some(s) = spinner {
            s.finish_and_clear();
        }

        if cli.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            let mut stdout = std::io::stdout();
            cytoscnpy::output::print_report(&mut stdout, &result)?;
        }

        // CI/CD Quality Gate: Exit with code 1 if finding percentage exceeds threshold
        // Threshold from: --fail-under flag > CYTOSCNPY_FAIL_THRESHOLD env var > default 10%
        let fail_threshold = cli.fail_under.or_else(|| {
            std::env::var("CYTOSCNPY_FAIL_THRESHOLD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
        });

        if let Some(threshold) = fail_threshold {
            // Count total unused items across all categories
            let total_findings = result.unused_functions.len()
                + result.unused_imports.len()
                + result.unused_classes.len()
                + result.unused_variables.len()
                + result.unused_parameters.len();

            let total_files = result.analysis_summary.total_files;

            if total_files > 0 {
                // Calculate findings per file ratio percentage
                let percentage = (total_findings as f64 / total_files as f64) * 100.0;

                if percentage > threshold {
                    eprintln!(
                        "\n[CI/CD] Quality gate FAILED: {total_findings} unused items ({percentage:.1} per 100 files) exceeds threshold of {threshold:.1}%"
                    );
                    std::process::exit(1);
                } else if !cli.json {
                    eprintln!(
                        "\n[CI/CD] Quality gate PASSED: {total_findings} unused items ({percentage:.1} per 100 files) is within threshold of {threshold:.1}%"
                    );
                }
            }
        }

        if cli.fail_on_quality && !result.quality.is_empty() {
            eprintln!(
                "\n[CI/CD] Quality gate FAILED: Found {} quality issues/violations.",
                result.quality.len()
            );
            std::process::exit(1);
        }

        Ok(())
    }
}
