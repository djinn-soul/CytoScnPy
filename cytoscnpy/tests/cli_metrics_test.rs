//! Tests for CLI metrics output.

#![allow(
    clippy::unwrap_used,
    clippy::str_to_string,
    clippy::uninlined_format_args,
    clippy::ignore_without_reason
)]

#[allow(deprecated)]
use cytoscnpy::commands::{
    run_cc, run_files, run_hal, run_mi, run_raw, run_stats, run_stats_v2, Inspections, ScanOptions,
};
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("cli_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "cli_metrics_test/cases_1.rs"]
mod cases_1;
#[path = "cli_metrics_test/cases_2.rs"]
mod cases_2;
#[path = "cli_metrics_test/cases_3.rs"]
mod cases_3;
