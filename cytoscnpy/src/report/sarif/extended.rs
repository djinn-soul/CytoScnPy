use super::{make_sarif_result, SarifResult};
use crate::analyzer::AnalysisResult;
use crate::report::category_findings::collect_extended_findings;
use std::path::Path;

pub(super) fn add_results(
    results: &mut Vec<SarifResult>,
    result: &AnalysisResult,
    root: Option<&Path>,
) {
    for finding in collect_extended_findings(result) {
        let (file, line) = finding.location_or_manifest(root);
        results.push(make_sarif_result(
            &finding.rule_id,
            &finding.message,
            &file,
            line,
            &finding.severity,
            None,
        ));
    }
}
