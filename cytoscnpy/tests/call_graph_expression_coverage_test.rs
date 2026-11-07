//! Tests that the call graph correctly records calls made inside every
//! expression form and statement form that was previously missed.
//!
//! Each test:
//!  - Defines a helper function that is called ONLY through one specific
//!    expression or statement construct.
//!  - Asserts that the helper is NOT reported as unreachable (the call graph
//!    must have picked up the edge, otherwise it would be flagged).
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::ProjectType;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn mk_dir(prefix: &str) -> TempDir {
    let mut base = std::env::current_dir().unwrap();
    base.push("target");
    base.push("test-cg-expr-tmp");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(base)
        .unwrap()
}

fn app_analyzer() -> CytoScnPy {
    let mut a = CytoScnPy::default().with_confidence(60).with_tests(false);
    a.config.cytoscnpy.project_type = Some(ProjectType::Application);
    a
}

fn unreachable_fns(result: &cytoscnpy::analyzer::types::AnalysisResult) -> Vec<String> {
    result
        .unused_functions
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.clone())
        .collect()
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn write_py(dir: &TempDir, name: &str, code: &str) {
    let mut f = File::create(dir.path().join(name)).unwrap();
    writeln!(f, "{code}").unwrap();
}

#[path = "call_graph_expression_coverage_test/cases_1.rs"]
mod cases_1;
#[path = "call_graph_expression_coverage_test/cases_2.rs"]
mod cases_2;
