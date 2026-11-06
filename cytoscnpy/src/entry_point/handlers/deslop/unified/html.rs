//! Slop Index page for the HTML report.

use std::path::Path;

use super::{ComprehensiveRequest, GateFailure, ScanIntegrity};

/// Adds the Slop Index page to the HTML report. Progress goes to stderr so
/// JSON on stdout stays parseable.
pub(super) fn write_html_page(
    request: &ComprehensiveRequest<'_>,
    scoring: &crate::scoring::ScoreResult,
    failures: &[GateFailure],
    integrity: &ScanIntegrity,
) {
    use crate::report::templates::{
        DeslopDimensionView, DeslopGateView, DeslopRecommendationView, DeslopView,
    };

    let view = DeslopView {
        slop_index: scoring.slop_index,
        verdict: scoring.verdict.to_string(),
        raw_score: format!("{:.1}", scoring.raw_score),
        size_multiplier: format!("{:.2}", scoring.size_multiplier),
        passed_gate: scoring.passed_gate,
        dimensions: scoring
            .dimensions
            .iter()
            .map(|dim| DeslopDimensionView {
                name: dim.name.clone(),
                weight: dim.weight,
                rating: dim.rating,
                contribution: format!("{:.1}", dim.raw_contribution),
                evidence: dim.evidence.clone(),
            })
            .collect(),
        recommendations: scoring
            .recommendations
            .iter()
            .map(|rec| DeslopRecommendationView {
                title: rec.title.clone(),
                dimension: rec.dimension.clone(),
                estimated_reduction: rec.estimated_reduction,
                effort: rec.effort.to_string(),
                description: rec.description.clone(),
                action_steps: rec.action_steps.clone(),
                affected_files: rec.affected_files.clone(),
            })
            .collect(),
        gate_failures: failures
            .iter()
            .map(|failure| DeslopGateView {
                check: failure.check.to_owned(),
                actual: failure.actual.clone(),
                limit: failure.limit.clone(),
            })
            .collect(),
        files_discovered: integrity.files_discovered,
        files_checked: integrity.files_checked,
        skipped_files: integrity
            .issues
            .iter()
            .map(|issue| (issue.path.display().to_string(), issue.reason.clone()))
            .collect(),
    };

    let report_dir = Path::new(".cytoscnpy/report");
    match crate::report::generator::generate_deslop_report(
        view,
        request.analysis_root,
        report_dir,
        request.args.html_theme,
    ) {
        Ok(page) => {
            eprintln!("Slop Index page written to: {}", page.display());
            if let Err(e) = open::that(&page) {
                eprintln!("Failed to open report in browser: {e}");
            }
        }
        Err(e) => eprintln!("Failed to generate HTML report: {e}"),
    }
}
