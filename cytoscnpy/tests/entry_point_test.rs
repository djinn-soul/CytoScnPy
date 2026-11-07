//! Integration tests for the application entry point.
//!
//! Tests the `run_with_args_to` function with various arguments.

#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::run_with_args_to;
use serde_json::Value;
use tempfile::tempdir;

#[path = "entry_point_test/cases_1.rs"]
mod cases_1;
#[path = "entry_point_test/cases_2.rs"]
mod cases_2;
