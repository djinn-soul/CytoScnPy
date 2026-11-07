//! Test suite for the multi-path analyzer functionality.
#![allow(clippy::unwrap_used)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::uninlined_format_args)]
//!
//! These tests verify that the `analyze_paths` method correctly handles:
//! - Single directory paths (delegating to standard analyze)
//! - Multiple file paths (for pre-commit hook efficiency)
//! - Mixed file and directory paths
//! - Empty path lists

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let target_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp-multipath");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("multipath_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "multi_path_test/cases_1.rs"]
mod cases_1;
#[path = "multi_path_test/cases_2.rs"]
mod cases_2;
