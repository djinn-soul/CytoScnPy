//! Comprehensive Radon Halstead parity tests.
//! Ported from: `radon/tests/test_halstead.py`

#![allow(clippy::unwrap_used)]
#![allow(clippy::cast_precision_loss)] // usize to f64 is intentional

use cytoscnpy::halstead::analyze_halstead;
use ruff_python_parser::{parse, Mode};

/// Helper to get Halstead metrics from code
fn get_halstead_counts(code: &str) -> (usize, usize, usize, usize) {
    if let Ok(ast) = parse(code, Mode::Module.into()) {
        if let ruff_python_ast::Mod::Module(m) = ast.into_syntax() {
            let metrics = analyze_halstead(&ruff_python_ast::Mod::Module(m));
            // Returns (total_operators, total_operands, distinct_operators, distinct_operands)
            return (metrics.n1, metrics.n2, metrics.h1, metrics.h2);
        }
    }
    (0, 0, 0, 0)
}

#[path = "radon_parity_halstead_test/cases_1.rs"]
mod cases_1;
#[path = "radon_parity_halstead_test/cases_2.rs"]
mod cases_2;
