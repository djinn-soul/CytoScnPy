//! Unit tests for security rules
//! Tests secrets and dangerous code detection
#![allow(
    clippy::unwrap_used,
    clippy::needless_raw_string_hashes,
    clippy::field_reassign_with_default
)]

use cytoscnpy::config::Config;
use cytoscnpy::linter::LinterVisitor;
use cytoscnpy::rules::danger::get_danger_rules;
use cytoscnpy::rules::secrets::scan_secrets_compat;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

// --- DANGER TESTS ---

macro_rules! scan_danger {
    ($source:expr, $linter:ident) => {
        let tree = parse($source, Mode::Module.into()).expect("Failed to parse");
        let line_index = LineIndex::new($source);
        let rules = get_danger_rules();
        let config = Config::default();
        let mut $linter =
            LinterVisitor::new(rules, PathBuf::from("test.py"), line_index, config, false);

        if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
            for stmt in &module.body {
                $linter.visit_stmt(stmt);
            }
        }
    };
}

// --- SECRETS IN COMMENTS TESTS ---
// These tests verify that secrets in comments are found by default

use cytoscnpy::config::SecretsConfig;
use cytoscnpy::rules::secrets::scan_secrets;

#[path = "security_test/cases_1.rs"]
mod cases_1;
#[path = "security_test/cases_2.rs"]
mod cases_2;
#[path = "security_test/cases_3.rs"]
mod cases_3;
#[path = "security_test/cases_4.rs"]
mod cases_4;
#[path = "security_test/cases_5.rs"]
mod cases_5;
#[path = "security_test/cases_6.rs"]
mod cases_6;
