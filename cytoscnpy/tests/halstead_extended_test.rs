//! Extended tests for `halstead.rs` - Halstead complexity metrics.
#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

use cytoscnpy::halstead::{analyze_halstead, analyze_halstead_functions, HalsteadMetrics};
use ruff_python_ast as ast;
use ruff_python_parser::parse_module;

/// Parse source and analyze Halstead metrics.
fn analyze_source(source: &str) -> HalsteadMetrics {
    let parsed = parse_module(source).unwrap();
    let module = ast::Mod::Module(parsed.into_syntax());
    analyze_halstead(&module)
}

/// Parse source and analyze per-function Halstead metrics.
fn analyze_functions(source: &str) -> Vec<(String, HalsteadMetrics)> {
    let parsed = parse_module(source).unwrap();
    let module = ast::Mod::Module(parsed.into_syntax());
    analyze_halstead_functions(&module)
}

#[path = "halstead_extended_test/cases_1.rs"]
mod cases_1;
#[path = "halstead_extended_test/cases_2.rs"]
mod cases_2;
