//! Tests for private function / method reachability edge cases.
//!
//! Covers:
//!  1. Private function called in an `AugAssign` — call graph gap; should NOT be unreachable.
//!  2. Private function called as a keyword argument — call graph gap; should NOT be unreachable.
//!  3. Private method called via `self.` — standard case; should NOT be unreachable.
//!  4. Pragma-suppressed (confidence = 0) private function that is also unreachable — must
//!     NOT be reported (Fix A: respect confidence == 0 in `should_report_definition`).
//!  5. Truly dead private function (never referenced anywhere) — MUST be reported.
//!  6. Private method of a dead class reported via `promote_methods_from_unused_classes`,
//!     even when its confidence is below the threshold (Fix D).
//!  7. Private method shared across two classes — only the class whose method is called
//!     is considered reachable; the other class is truly dead and its method should be
//!     promoted as dead code.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::ProjectType;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn mk_tempdir(prefix: &str) -> TempDir {
    let mut target = std::env::current_dir().unwrap();
    target.push("target");
    target.push("test-private-reach-tmp");
    std::fs::create_dir_all(&target).unwrap();
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(target)
        .unwrap()
}

fn app_analyzer() -> CytoScnPy {
    let mut a = CytoScnPy::default().with_confidence(60).with_tests(false);
    a.config.cytoscnpy.project_type = Some(ProjectType::Application);
    a
}

#[path = "private_function_reachability_test/cases_1.rs"]
mod cases_1;
#[path = "private_function_reachability_test/cases_2.rs"]
mod cases_2;
