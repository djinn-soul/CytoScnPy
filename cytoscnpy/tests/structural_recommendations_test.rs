//! Integration tests for top-level directory breakdowns, function collision disambiguation,
//! and stable library extraction recommendations.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use std::process::Command;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[path = "structural_recommendations_test/cases_1.rs"]
mod cases_1;
