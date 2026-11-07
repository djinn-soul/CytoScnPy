//! Tests for accuracy improvements (return tracking, `__all__` exports, `TYPE_CHECKING` imports).

#![allow(
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::needless_raw_string_hashes
)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-acc-imp-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("accuracy_imp_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "accuracy_improvements_test/cases_1.rs"]
mod cases_1;
#[path = "accuracy_improvements_test/cases_2.rs"]
mod cases_2;
