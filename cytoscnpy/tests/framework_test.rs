//! Tests for framework detection (Flask, Django, etc.).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::str_to_string,
    clippy::needless_raw_string_hashes
)]

use cytoscnpy::framework::FrameworkAwareVisitor;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};

fn run_framework_visitor<'a>(source: &str, line_index: &'a LineIndex) -> FrameworkAwareVisitor<'a> {
    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let mut visitor = FrameworkAwareVisitor::new(line_index);

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            visitor.visit_stmt(stmt);
        }
    }
    visitor
}

// ============================================================================
// TestDetectFrameworkUsage (6 tests)
// ============================================================================

use cytoscnpy::framework::detect_framework_usage;

// ============================================================================
// TestFrameworkPatterns (3 tests)
// ============================================================================

use cytoscnpy::framework::{get_framework_imports, FRAMEWORK_DECORATORS, FRAMEWORK_FUNCTIONS};

#[path = "framework_test/cases_1.rs"]
mod cases_1;
#[path = "framework_test/cases_2.rs"]
mod cases_2;
#[path = "framework_test/cases_3.rs"]
mod cases_3;
