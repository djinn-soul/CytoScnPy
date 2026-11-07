//! Test suite for the analyzer module.
#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::{WhitelistEntry, WhitelistPattern};
use cytoscnpy::whitelist::WhitelistMatcher;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-analyzer-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("analyzer_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[path = "analyzer_test/cases_1.rs"]
mod cases_1;
