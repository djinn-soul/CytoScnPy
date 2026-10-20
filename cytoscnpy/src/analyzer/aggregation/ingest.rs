//! Merging per-file results into the aggregation state.

use super::references::add_reference;
use super::state::AggregationState;
use super::{CytoScnPy, FileAnalysisResult};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

impl AggregationState {
    /// Merges every file result in order.
    ///
    /// The call graph and both reference-count maps dominate large scans and
    /// touch no other state, so they merge on other threads while this one
    /// ingests the rest. Each merge still visits files in order, so the
    /// merged maps match a serial merge.
    pub(super) fn ingest_results(
        &mut self,
        results: Vec<FileAnalysisResult>,
        files: &[PathBuf],
        analyzer: &mut CytoScnPy,
    ) {
        self.reserve_for(&results);
        let mut call_graphs = Vec::with_capacity(results.len());
        let mut references = Vec::with_capacity(results.len());
        let mut rest = Vec::with_capacity(results.len());
        for mut result in results {
            call_graphs.push(std::mem::take(&mut result.call_graph));
            references.push(std::mem::take(&mut result.references));
            rest.push(result);
        }

        let mut call_graph = std::mem::take(&mut self.global_call_graph);
        let mut ref_counts = std::mem::take(&mut self.ref_counts);
        let mut prod_ref_counts = std::mem::take(&mut self.prod_ref_counts);
        let analysis_root = analyzer.analysis_root.clone();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                rayon::join(
                    || {
                        for graph in call_graphs {
                            call_graph.merge(graph);
                        }
                    },
                    || {
                        rayon::join(
                            || merge_references(&mut ref_counts, &references, None),
                            || {
                                let roots = Some((files, analysis_root.as_path()));
                                merge_references(&mut prod_ref_counts, &references, roots);
                            },
                        )
                    },
                );
            });
            for (result, file_path) in rest.into_iter().zip(files) {
                self.ingest_file_result(result, file_path, analyzer);
            }
        });
        self.global_call_graph = call_graph;
        self.ref_counts = ref_counts;
        self.prod_ref_counts = prod_ref_counts;
        crate::utils::drop_in_background(references);
    }

    /// Sizes the merged collections once instead of regrowing them per file.
    /// Reference and call-graph totals overcount names shared across files,
    /// so they are an upper bound.
    fn reserve_for(&mut self, results: &[FileAnalysisResult]) {
        let total = |count: fn(&FileAnalysisResult) -> usize| results.iter().map(count).sum();
        let definitions: usize = total(|result| result.definitions.len());
        let references: usize = total(|result| result.references.len());
        let nodes: usize = total(|result| result.call_graph.nodes.len());
        self.all_defs.reserve(definitions);
        self.ref_counts.reserve(references);
        self.prod_ref_counts.reserve(references);
        self.global_call_graph.nodes.reserve(nodes);
        self.file_metrics.reserve(results.len());
    }
}

/// Adds each file's reference counts to `totals`. With `production_only`,
/// files under a test path relative to the root are skipped.
fn merge_references(
    totals: &mut FxHashMap<String, usize>,
    references: &[FxHashMap<String, usize>],
    production_only: Option<(&[PathBuf], &Path)>,
) {
    for (index, file_references) in references.iter().enumerate() {
        if let Some((files, root)) = production_only {
            let file = &files[index];
            let relative = file.strip_prefix(root).unwrap_or(file);
            if crate::utils::is_test_path(&relative.to_string_lossy()) {
                continue;
            }
        }
        for (name, count) in file_references {
            add_reference(totals, name, *count);
        }
    }
}
