//! TDD tests for F1 score and accuracy improvements.
//!
//! These tests define expected behavior for:
//! 1. Class-method linking (methods in unused classes should be flagged)
//! 2. Nested function call tracking (nested functions called within parent scope)
//! 3. Pattern matching variable usage

#![allow(
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::needless_raw_string_hashes
)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-accuracy-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("accuracy_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "accuracy_test/cases_1.rs"]
mod cases_1;
#[path = "accuracy_test/cases_2.rs"]
mod cases_2;
