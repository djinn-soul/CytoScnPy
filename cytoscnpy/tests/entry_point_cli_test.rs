//! Tests for `entry_point`.rs CLI argument handling and `run_with_args` function.
#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::{run_with_args, run_with_args_to};
use std::fs;
use tempfile::{tempdir, TempDir};

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-cli-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("cli_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

/// Helper function to run CLI with output captured to suppress test noise.
fn run_with_captured_output(args: Vec<String>) -> anyhow::Result<i32> {
    let mut buffer = Vec::new();
    run_with_args_to(args, &mut buffer)
}

#[path = "entry_point_cli_test/cases_1.rs"]
mod cases_1;
#[path = "entry_point_cli_test/cases_2.rs"]
mod cases_2;
#[path = "entry_point_cli_test/cases_3.rs"]
mod cases_3;
