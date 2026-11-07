//! Tests for CLI JSON output format validation.
//!
//! These tests verify that JSON output from all subcommands is correctly structured
//! and can be deserialized by external tools.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::uninlined_format_args
)]

use cytoscnpy::commands::{run_cc, run_hal, run_mi, run_raw};
use serde_json::Value;
use std::fs;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-cli-json-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("cli_json_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "cli_json_test/cases_1.rs"]
mod cases_1;
