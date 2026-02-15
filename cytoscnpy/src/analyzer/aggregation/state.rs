use super::{CytoScnPy, FileAnalysisResult};
use crate::analyzer::fixtures::{
    FixtureDefinitionRecord, FixtureImportBinding, FixtureRequestRecord, PytestPluginDeclaration,
};
use crate::analyzer::types::{FileMetrics, ParseError};
use crate::halstead::HalsteadMetrics;
use crate::raw_metrics::RawMetrics;
use crate::rules::secrets::SecretFinding;
use crate::rules::Finding;
use crate::taint::call_graph::CallGraph;
use crate::visitor::Definition;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Default)]
pub(super) struct AggregationState {
    pub(super) all_defs: Vec<Definition>,
    pub(super) ref_counts: FxHashMap<String, usize>,
    pub(super) all_secrets: Vec<SecretFinding>,
    pub(super) all_danger: Vec<Finding>,
    pub(super) all_quality: Vec<Finding>,
    pub(super) all_parse_errors: Vec<ParseError>,
    pub(super) all_fixture_definitions: Vec<FixtureDefinitionRecord>,
    pub(super) all_fixture_requests: Vec<FixtureRequestRecord>,
    pub(super) all_fixture_imports: Vec<FixtureImportBinding>,
    pub(super) all_pytest_plugins: Vec<PytestPluginDeclaration>,
    pub(super) dynamic_imported_modules: FxHashSet<String>,
    pub(super) total_complexity: f64,
    pub(super) total_mi: f64,
    pub(super) total_size_bytes: usize,
    pub(super) files_with_quality_metrics: usize,
    pub(super) all_raw_metrics: RawMetrics,
    pub(super) all_halstead_metrics: HalsteadMetrics,
    pub(super) file_metrics: Vec<FileMetrics>,
    pub(super) all_protocols: FxHashMap<String, FxHashSet<String>>,
    pub(super) global_call_graph: CallGraph,
}

impl AggregationState {
    pub(super) fn ingest_file_result(
        &mut self,
        result: FileAnalysisResult,
        file_path: &std::path::PathBuf,
        analyzer: &mut CytoScnPy,
    ) {
        let FileAnalysisResult {
            definitions,
            references,
            protocol_methods,
            secrets,
            danger,
            quality,
            parse_errors,
            line_count,
            raw_metrics,
            halstead_metrics,
            complexity,
            mi,
            file_size,
            call_graph,
            dynamic_imports,
            fixture_definitions,
            fixture_requests,
            fixture_imports,
            pytest_plugins,
        } = result;

        self.global_call_graph.merge(call_graph);
        self.total_size_bytes += file_size;

        self.all_raw_metrics.loc += raw_metrics.loc;
        self.all_raw_metrics.lloc += raw_metrics.lloc;
        self.all_raw_metrics.sloc += raw_metrics.sloc;
        self.all_raw_metrics.comments += raw_metrics.comments;
        self.all_raw_metrics.multi += raw_metrics.multi;
        self.all_raw_metrics.blank += raw_metrics.blank;
        self.all_raw_metrics.single_comments += raw_metrics.single_comments;

        self.all_halstead_metrics.h1 += halstead_metrics.h1;
        self.all_halstead_metrics.h2 += halstead_metrics.h2;
        self.all_halstead_metrics.n1 += halstead_metrics.n1;
        self.all_halstead_metrics.n2 += halstead_metrics.n2;
        self.all_halstead_metrics.vocabulary += halstead_metrics.vocabulary;
        self.all_halstead_metrics.length += halstead_metrics.length;
        self.all_halstead_metrics.calculated_length += halstead_metrics.calculated_length;
        self.all_halstead_metrics.volume += halstead_metrics.volume;
        self.all_halstead_metrics.difficulty += halstead_metrics.difficulty;
        self.all_halstead_metrics.effort += halstead_metrics.effort;
        self.all_halstead_metrics.time += halstead_metrics.time;
        self.all_halstead_metrics.bugs += halstead_metrics.bugs;

        self.file_metrics.push(FileMetrics {
            file: file_path.clone(),
            loc: raw_metrics.loc,
            sloc: raw_metrics.sloc,
            complexity,
            mi,
            total_issues: danger.len() + quality.len() + secrets.len(),
        });

        self.all_defs.extend(definitions);

        for (name, count) in references {
            *self.ref_counts.entry(name).or_insert(0) += count;
        }

        for (proto, methods) in protocol_methods {
            self.all_protocols.entry(proto).or_default().extend(methods);
        }

        self.all_secrets.extend(secrets);
        self.all_danger.extend(danger);
        self.all_quality.extend(quality);
        self.all_parse_errors.extend(parse_errors);
        self.all_fixture_definitions.extend(fixture_definitions);
        self.all_fixture_requests.extend(fixture_requests);
        self.all_fixture_imports.extend(fixture_imports);
        self.all_pytest_plugins.extend(pytest_plugins);

        self.dynamic_imported_modules.extend(
            dynamic_imports
                .into_iter()
                .filter(|module| !module.trim().is_empty())
                .map(|module| module.trim_start_matches('.').to_owned()),
        );

        analyzer.total_lines_analyzed += line_count;

        if complexity > 0.0 || mi > 0.0 {
            self.total_complexity += complexity;
            self.total_mi += mi;
            self.files_with_quality_metrics += 1;
        }
    }

    pub(super) fn apply_fixture_reference_increments(&mut self) {
        let fixture_ref_increments =
            crate::analyzer::fixtures::resolve_fixture_reference_increments(
                &self.all_fixture_definitions,
                &self.all_fixture_requests,
                &self.all_fixture_imports,
                &self.all_pytest_plugins,
            );

        for (full_name, count) in fixture_ref_increments {
            *self.ref_counts.entry(full_name).or_insert(0) += count;
        }
    }

    pub(super) fn fixture_definition_names(&self) -> FxHashSet<String> {
        self.all_fixture_definitions
            .iter()
            .map(|def| def.full_name.clone())
            .collect()
    }
}
