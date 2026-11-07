//! Tests for unused parameter detection.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-parameter-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("parameter_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "parameter_test/cases_1.rs"]
mod cases_1;
#[path = "parameter_test/cases_2.rs"]
mod cases_2;
