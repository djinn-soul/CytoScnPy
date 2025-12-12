//! Main binary entry point for the `CytoScnPy` static analysis tool.

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::cli::{Cli, Commands};
use cytoscnpy::commands::{run_cc, run_hal, run_mi, run_raw};
use cytoscnpy::config::Config;

use anyhow::Result;
use clap::Parser;

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
            danger, // taint is now automatically enabled with --danger
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
