//! Unit tests for performance rules
#![allow(clippy::expect_used)]

use cytoscnpy::config::Config;
use cytoscnpy::linter::LinterVisitor;
use cytoscnpy::rules::quality::get_quality_rules;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

fn run_linter(source: &str, config: Config) -> LinterVisitor {
    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let line_index = LineIndex::new(source);
    let rules = get_quality_rules(&config);
    let mut linter = LinterVisitor::new(
        rules,
        PathBuf::from("performance_test.py"),
        line_index,
        config,
        false,
    );

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            linter.visit_stmt(stmt);
        }
    }
    linter
}

#[path = "performance_rules_test/cases_1.rs"]
mod cases_1;
#[path = "performance_rules_test/cases_2.rs"]
mod cases_2;
