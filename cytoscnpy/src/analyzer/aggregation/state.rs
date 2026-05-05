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
    /// References from non-test files only. Used by dead-code classification so that
    /// production symbols called exclusively from tests are still flagged as unused.
    pub(super) prod_ref_counts: FxHashMap<String, usize>,
    pub(super) all_import_bindings: FxHashMap<String, String>,
    pub(super) all_secrets: Vec<SecretFinding>,
    pub(super) all_danger: Vec<Finding>,
    pub(super) all_quality: Vec<Finding>,
    pub(super) all_parse_errors: Vec<ParseError>,
    pub(super) all_fixture_definitions: Vec<FixtureDefinitionRecord>,
    pub(super) all_fixture_requests: Vec<FixtureRequestRecord>,
    pub(super) all_fixture_imports: Vec<FixtureImportBinding>,
    pub(super) all_pytest_plugins: Vec<PytestPluginDeclaration>,
    /// Exports listed in each module's `__all__`: `module_name` → list of unqualified names.
    pub(super) all_module_exports: Vec<(String, Vec<String>)>,
    /// `(importer_module, source_module)` pairs from `from x import *` statements.
    pub(super) all_star_imports: Vec<(String, String)>,
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
            import_bindings,
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
            exports,
            module_name,
            star_imports,
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

        let relative_path = file_path
            .strip_prefix(&analyzer.analysis_root)
            .unwrap_or(file_path);
        let is_test = crate::utils::is_test_path(&relative_path.to_string_lossy());
        for (name, count) in references {
            *self.ref_counts.entry(name.clone()).or_insert(0) += count;
            if !is_test {
                *self.prod_ref_counts.entry(name).or_insert(0) += count;
            }
        }
        self.all_import_bindings.extend(import_bindings);

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

        if !exports.is_empty() && !module_name.is_empty() {
            self.all_module_exports.push((module_name.clone(), exports));
        }
        if !module_name.is_empty() {
            self.all_star_imports.extend(
                star_imports
                    .into_iter()
                    .map(|source_module| (module_name.clone(), source_module)),
            );
        }

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

    pub(super) fn apply_import_binding_reference_increments(&mut self) {
        let mut used_symbols: FxHashSet<String> = self
            .ref_counts
            .iter()
            .filter_map(|(name, count)| if *count > 0 { Some(name.clone()) } else { None })
            .collect();

        let mut worklist: Vec<String> = used_symbols.iter().cloned().collect();
        while let Some(symbol) = worklist.pop() {
            if let Some(source_symbol) = self.all_import_bindings.get(&symbol) {
                let source_symbol = source_symbol.clone();
                if used_symbols.insert(source_symbol.clone()) {
                    worklist.push(source_symbol);
                }
            }
        }

        for symbol in used_symbols {
            self.ref_counts
                .entry(symbol)
                .and_modify(|count| *count = (*count).max(1))
                .or_insert(1);
        }
    }

    /// Same as `apply_import_binding_reference_increments` but operates on `prod_ref_counts`.
    /// Must run after `apply_star_import_bindings` and `apply_export_reference_increments`.
    pub(super) fn apply_prod_import_binding_reference_increments(&mut self) {
        let mut used_symbols: FxHashSet<String> = self
            .prod_ref_counts
            .iter()
            .filter_map(|(name, count)| if *count > 0 { Some(name.clone()) } else { None })
            .collect();

        let mut worklist: Vec<String> = used_symbols.iter().cloned().collect();
        while let Some(symbol) = worklist.pop() {
            if let Some(source_symbol) = self.all_import_bindings.get(&symbol) {
                let source_symbol = source_symbol.clone();
                if used_symbols.insert(source_symbol.clone()) {
                    worklist.push(source_symbol);
                }
            }
        }

        for symbol in used_symbols {
            self.prod_ref_counts
                .entry(symbol)
                .and_modify(|count| *count = (*count).max(1))
                .or_insert(1);
        }
    }

    /// Resolves `from x import *` cross-file.
    ///
    /// For each recorded star-import `(importer_module, source_module)`, looks up the
    /// source module's `__all__` exports (collected during file analysis)
    /// and synthesises explicit import bindings:
    ///   `importer_module.Name  →  source_module.Name`
    ///
    /// This must run **before** `apply_import_binding_reference_increments` so the new bindings feed
    /// into the worklist.
    ///
    pub(super) fn apply_star_import_bindings(&mut self) {
        // Build a fast lookup: source_module → [exported_names]
        let mut export_map: FxHashMap<&str, &[String]> = FxHashMap::default();
        for (module, names) in &self.all_module_exports {
            export_map.insert(module.as_str(), names.as_slice());
        }

        for (importer, source) in &self.all_star_imports {
            let Some(names) = export_map.get(source.as_str()) else {
                continue;
            };
            for name in *names {
                let local_key = format!("{importer}.{name}");
                let source_val = format!("{source}.{name}");
                // Synthesise binding: importer.Name → source.Name
                self.all_import_bindings
                    .entry(local_key)
                    .or_insert(source_val);
            }
        }
    }

    /// For every symbol listed in any module's `__all__`, ensures its qualified name
    /// (`module.symbol`) is present in `ref_counts` with a count of at least 1.
    /// This must run **before** `apply_import_binding_reference_increments` so the
    /// export refs seed the worklist that propagates through import-binding chains.
    pub(super) fn apply_export_reference_increments(&mut self) {
        for (module_name, exports) in &self.all_module_exports {
            for export_name in exports {
                let qualified = format!("{module_name}.{export_name}");
                self.ref_counts
                    .entry(qualified.clone())
                    .and_modify(|c| *c = (*c).max(1))
                    .or_insert(1);
                self.prod_ref_counts
                    .entry(qualified)
                    .and_modify(|c| *c = (*c).max(1))
                    .or_insert(1);
            }
        }
    }

    pub(super) fn fixture_definition_names(&self) -> FxHashSet<String> {
        self.all_fixture_definitions
            .iter()
            .map(|def| def.full_name.clone())
            .collect()
    }
}
