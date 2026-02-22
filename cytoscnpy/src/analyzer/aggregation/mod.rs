//! Aggregation of analysis results.

mod classify;
mod reachability;
mod sorting;
mod state;
mod taint;

use self::classify::{classify_definitions, promote_methods_from_unused_classes};
use self::reachability::build_reachability;
use self::sorting::{
    sort_definitions, sort_findings, sort_parse_errors, sort_secrets, sort_taint_findings,
};
use self::state::AggregationState;

use super::{AnalysisResult, AnalysisSummary, CytoScnPy, FileAnalysisResult};
use crate::visitor::Definition;
use rustc_hash::FxHashMap;

impl CytoScnPy {
    /// Aggregates results from multiple file analyses.
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn aggregate_results(
        &mut self,
        results: Vec<FileAnalysisResult>,
        files: &[std::path::PathBuf],
        total_files: usize,
        total_directories: usize,
    ) -> AnalysisResult {
        let mut state = AggregationState::default();

        for (idx, res) in results.into_iter().enumerate() {
            state.ingest_file_result(res, &files[idx], self);
        }

        state.apply_fixture_reference_increments();
        let total_definitions = state.all_defs.len();
        let functions_count = state
            .all_defs
            .iter()
            .filter(|d| d.def_type == "function" || d.def_type == "method")
            .count();
        let classes_count = state
            .all_defs
            .iter()
            .filter(|d| d.def_type == "class")
            .count();

        let fixture_definition_names = state.fixture_definition_names();
        let reachability = build_reachability(
            &state.all_defs,
            &state.all_protocols,
            &state.dynamic_imported_modules,
            &state.global_call_graph,
        );

        let mut classified = classify_definitions(
            state.all_defs,
            &state.ref_counts,
            &reachability,
            &fixture_definition_names,
            self.confidence_threshold,
            self.whitelist_matcher.as_ref(),
        );

        promote_methods_from_unused_classes(
            &mut classified.unused_methods,
            &classified.methods_with_refs,
            self.confidence_threshold,
            &classified.unused_classes,
        );

        let mut taint_findings = taint::run_taint_analysis(self, files);

        let mut unused_counts: FxHashMap<std::path::PathBuf, usize> = FxHashMap::default();
        let all_unused_slices: [&[Definition]; 6] = [
            &classified.unused_functions,
            &classified.unused_methods,
            &classified.unused_imports,
            &classified.unused_classes,
            &classified.unused_variables,
            &classified.unused_parameters,
        ];

        for def in all_unused_slices.into_iter().flatten() {
            *unused_counts.entry(def.file.as_ref().clone()).or_insert(0) += 1;
        }

        for metric in &mut state.file_metrics {
            if let Some(count) = unused_counts.get(&metric.file) {
                metric.total_issues += count;
            }
        }

        sort_definitions(&mut classified.unused_functions);
        sort_definitions(&mut classified.unused_methods);
        sort_definitions(&mut classified.unused_imports);
        sort_definitions(&mut classified.unused_classes);
        sort_definitions(&mut classified.unused_variables);
        sort_definitions(&mut classified.unused_parameters);

        sort_findings(&mut state.all_danger);
        sort_findings(&mut state.all_quality);
        sort_secrets(&mut state.all_secrets);
        sort_parse_errors(&mut state.all_parse_errors);
        sort_taint_findings(&mut taint_findings);

        let taint_count = taint_findings.len();

        AnalysisResult {
            unused_functions: classified.unused_functions,
            unused_methods: classified.unused_methods,
            unused_imports: classified.unused_imports,
            unused_classes: classified.unused_classes,
            unused_variables: classified.unused_variables,
            unused_parameters: classified.unused_parameters,
            secrets: state.all_secrets.clone(),
            danger: state.all_danger.clone(),
            quality: state.all_quality.clone(),
            taint_findings,
            parse_errors: state.all_parse_errors.clone(),
            clones: Vec::new(),
            analysis_summary: AnalysisSummary {
                total_files,
                secrets_count: state.all_secrets.len(),
                danger_count: state.all_danger.len(),
                quality_count: state.all_quality.len(),
                taint_count,
                parse_errors_count: state.all_parse_errors.len(),
                total_lines_analyzed: self.total_lines_analyzed,
                total_definitions,
                average_complexity: if state.files_with_quality_metrics > 0 {
                    state.total_complexity / state.files_with_quality_metrics as f64
                } else {
                    0.0
                },
                average_mi: if state.files_with_quality_metrics > 0 {
                    state.total_mi / state.files_with_quality_metrics as f64
                } else {
                    0.0
                },
                total_directories,
                total_size: state.total_size_bytes as f64 / 1024.0,
                functions_count,
                classes_count,
                raw_metrics: state.all_raw_metrics,
                halstead_metrics: state.all_halstead_metrics,
            },
            file_metrics: state.file_metrics,
        }
    }
}
