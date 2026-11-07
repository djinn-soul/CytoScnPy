//! Additional CLI, Test-Aware, and Integration tests

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::str_to_string,
    clippy::uninlined_format_args,
    clippy::needless_raw_string_hashes
)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::Config;
use cytoscnpy::test_utils::TestAwareVisitor;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-additional-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("additional_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "additional_coverage_test/cases_1.rs"]
mod cases_1;
#[path = "additional_coverage_test/cases_2.rs"]
mod cases_2;
