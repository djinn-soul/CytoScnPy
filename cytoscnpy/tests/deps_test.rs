//! Integration tests for the `deps` subcommand.
#![allow(clippy::unwrap_used)]
use cytoscnpy::entry_point::run_with_args_to;
use std::fs;
use tempfile::tempdir;

fn run_deps_command(args: Vec<String>) -> (i32, String) {
    let mut buffer = Vec::new();
    let code = run_with_args_to(args, &mut buffer).unwrap_or(1);
    let output = String::from_utf8_lossy(&buffer).into_owned();
    (code, output)
}

#[path = "deps_test/cases_1.rs"]
mod cases_1;
#[path = "deps_test/cases_2.rs"]
mod cases_2;
#[path = "deps_test/cases_3.rs"]
mod cases_3;
#[path = "deps_test/cases_4.rs"]
mod cases_4;
#[path = "deps_test/cases_5.rs"]
mod cases_5;
