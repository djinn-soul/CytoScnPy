use crate::analyzer::types::ParseError;
use crate::rules::secrets::SecretFinding;
use crate::rules::Finding;
use crate::taint::types::TaintFinding;
use crate::visitor::Definition;
use rayon::slice::ParallelSliceMut;
use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::Arc;

/// Definitions from one file share its `Arc`, so most comparisons skip the
/// component-wise path comparison.
fn compare_files(a: &Arc<PathBuf>, b: &Arc<PathBuf>) -> Ordering {
    if Arc::ptr_eq(a, b) {
        Ordering::Equal
    } else {
        a.cmp(b)
    }
}

pub(super) fn sort_definitions(definitions: &mut [Definition]) {
    // Stable like `sort_by`, so equal keys keep their order.
    definitions.par_sort_by(|a, b| {
        compare_files(&a.file, &b.file)
            .then(a.line.cmp(&b.line))
            .then(a.col.cmp(&b.col))
    });
}

pub(super) fn sort_findings(findings: &mut [Finding]) {
    findings.par_sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.col.cmp(&b.col))
    });
}

pub(super) fn sort_secrets(findings: &mut [SecretFinding]) {
    findings.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
}

pub(super) fn sort_parse_errors(errors: &mut [ParseError]) {
    errors.sort_by(|a, b| a.file.cmp(&b.file).then(a.error.cmp(&b.error)));
}

pub(super) fn sort_taint_findings(findings: &mut [TaintFinding]) {
    findings.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.sink_line.cmp(&b.sink_line))
            .then(a.sink_col.cmp(&b.sink_col))
    });
}
