//! Comprehensive Radon complexity parity tests.
//! Ported from: `radon/tests/test_complexity_visitor.py`
//!
//! NOTE: `CytoScnPy`'s `analyze_complexity` only reports complexity for functions/methods,
//! not for module-level code. So we wrap test code in functions.

#![allow(clippy::unwrap_used)] // Tests use unwrap for clarity

use cytoscnpy::complexity::analyze_complexity;
use std::path::PathBuf;

/// Helper to get function complexity by name
fn get_function_complexity(code: &str, name: &str) -> usize {
    let findings = analyze_complexity(code, &PathBuf::from("test.py"), false);
    findings
        .iter()
        .find(|f| f.name == name)
        .map_or(0, |f| f.complexity)
}

/// Helper to get function complexity with `no_assert` flag
fn get_function_complexity_no_assert(code: &str, name: &str) -> usize {
    let findings = analyze_complexity(code, &PathBuf::from("test.py"), true);
    findings
        .iter()
        .find(|f| f.name == name)
        .map_or(0, |f| f.complexity)
}

// =============================================================================
// TODO: MODULE-LEVEL COMPLEXITY TESTS
// These tests will FAIL until module-level complexity reporting is implemented.
// Radon reports complexity for code at the module level (outside functions).
// CytoScnPy currently only reports complexity for functions/methods.
// =============================================================================

/// Helper to get total module-level complexity (sum of all findings)
fn get_module_complexity(code: &str) -> usize {
    let findings = analyze_complexity(code, &PathBuf::from("test.py"), false);
    findings.iter().map(|f| f.complexity).sum()
}

#[path = "radon_parity_complexity_test/cases_1.rs"]
mod cases_1;
#[path = "radon_parity_complexity_test/cases_2.rs"]
mod cases_2;
#[path = "radon_parity_complexity_test/cases_3.rs"]
mod cases_3;
