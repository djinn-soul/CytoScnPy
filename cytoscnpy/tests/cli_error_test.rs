//! Tests for CLI error handling and error message formatting.
//!
//! These tests verify that the CLI gracefully handles error conditions
//! and produces useful error messages.
#![allow(
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::needless_raw_string_hashes
)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::Config;
use std::fs;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-cli-error-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("cli_error_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "cli_error_test/cases_1.rs"]
mod cases_1;
#[path = "cli_error_test/cases_2.rs"]
mod cases_2;
