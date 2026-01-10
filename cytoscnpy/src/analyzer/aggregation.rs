//! Aggregation of analysis results.

use super::{apply_heuristics, AnalysisResult, AnalysisSummary, CytoScnPy, ParseError};
use crate::analyzer::types::FileMetrics;
use crate::halstead::HalsteadMetrics;
use crate::raw_metrics::RawMetrics;
use crate::rules::secrets::SecretFinding;
use crate::rules::Finding;
use crate::visitor::Definition;

use rustc_hash::FxHashMap;
use std::fs;

impl CytoScnPy {
    /// Aggregates results from multiple file analyses.
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    pub(crate) fn aggregate_results(
        &mut self,
        results: Vec<(
            Vec<Definition>,
            FxHashMap<String, usize>,
            Vec<SecretFinding>,
            Vec<Finding>,
            Vec<Finding>,
            Vec<ParseError>,
            usize,
            RawMetrics,
            HalsteadMetrics,
            f64,
            f64,
            usize,
        )>,
        files: &[std::path::PathBuf],
        total_files: usize,
        total_directories: usize,
    ) -> AnalysisResult {
        let mut all_defs = Vec::new();
        let mut ref_counts: FxHashMap<String, usize> = FxHashMap::default();
        let mut all_secrets = Vec::new();
        let mut all_danger = Vec::new();
        let mut all_quality = Vec::new();
        let mut all_parse_errors = Vec::new();

        let mut total_complexity = 0.0;
        let mut total_mi = 0.0;
        let mut total_size_bytes = 0;
        let mut files_with_quality_metrics = 0;

        let mut all_raw_metrics = RawMetrics::default();
        let mut all_halstead_metrics = HalsteadMetrics::default();
        let mut file_metrics = Vec::new();

        for (
            i,
            (
                defs,
                refs,
                secrets,
                danger,
                quality,
                parse_errors,
                lines,
                raw,
                halstead,
                complexity,
                mi,
                size,
            ),
        ) in results.into_iter().enumerate()
        {
            let file_path: &std::path::PathBuf = &files[i];

            total_size_bytes += size;

            // Aggregate Raw Metrics
            all_raw_metrics.loc += raw.loc;
            all_raw_metrics.lloc += raw.lloc;
            all_raw_metrics.sloc += raw.sloc;
            all_raw_metrics.comments += raw.comments;
            all_raw_metrics.multi += raw.multi;
            all_raw_metrics.blank += raw.blank;
            all_raw_metrics.single_comments += raw.single_comments;

            // Aggregate Halstead Metrics (Summing for project total approximation)
            all_halstead_metrics.h1 += halstead.h1;
            all_halstead_metrics.h2 += halstead.h2;
            all_halstead_metrics.n1 += halstead.n1;
            all_halstead_metrics.n2 += halstead.n2;
            all_halstead_metrics.vocabulary += halstead.vocabulary;
            all_halstead_metrics.length += halstead.length;
            all_halstead_metrics.calculated_length += halstead.calculated_length;
            all_halstead_metrics.volume += halstead.volume;
            all_halstead_metrics.difficulty += halstead.difficulty;
            all_halstead_metrics.effort += halstead.effort;
            all_halstead_metrics.time += halstead.time;
            all_halstead_metrics.bugs += halstead.bugs;

            file_metrics.push(FileMetrics {
                file: file_path.clone(),
                loc: raw.loc,
                sloc: raw.sloc,
                complexity,
                mi,
                total_issues: danger.len() + quality.len() + secrets.len(),
            });

            all_defs.extend(defs);
            // Merge reference counts from each file

            for (name, count) in refs {
                *ref_counts.entry(name).or_insert(0) += count;
            }
            all_secrets.extend(secrets);
            all_danger.extend(danger);
            all_quality.extend(quality);
            all_parse_errors.extend(parse_errors);
            self.total_lines_analyzed += lines;

            if complexity > 0.0 || mi > 0.0 {
                total_complexity += complexity;
                total_mi += mi;
                files_with_quality_metrics += 1;
            }
        }

        let mut unused_functions = Vec::new();
        let mut unused_methods = Vec::new();
        let mut unused_classes = Vec::new();
        let mut unused_imports = Vec::new();
        let mut unused_variables = Vec::new();
        let mut unused_parameters = Vec::new();

        let total_definitions = all_defs.len();

        let functions_count = all_defs
            .iter()
            .filter(|d| d.def_type == "function" || d.def_type == "method")
            .count();
        let classes_count = all_defs.iter().filter(|d| d.def_type == "class").count();

        let mut methods_with_refs: Vec<Definition> = Vec::new();

        for mut def in all_defs {
            if let Some(count) = ref_counts.get(&def.full_name) {
                def.references = *count;
            } else if def.def_type != "variable" {
                if let Some(count) = ref_counts.get(&def.simple_name) {
                    def.references = *count;
                }
            }

            apply_heuristics(&mut def);

            if def.confidence < self.confidence_threshold {
                continue;
            }

            // Collect methods with references for class-method linking
            if def.def_type == "method" && def.references > 0 {
                methods_with_refs.push(def.clone());
            }

            if def.references == 0 {
                match def.def_type.as_str() {
                    "function" => unused_functions.push(def),
                    "method" => unused_methods.push(def),
                    "class" => unused_classes.push(def),
                    "import" => unused_imports.push(def),
                    "variable" => unused_variables.push(def),
                    "parameter" => unused_parameters.push(def),
                    _ => {}
                }
            }
        }

        // Class-method linking: ALL methods of unused classes should be flagged as unused.
        // This implements "cascading deadness" - if a class is unreachable, all its methods are too.
        // EXCEPTION: Skip methods protected by heuristics (visitor pattern, etc.)
        let unused_class_names: std::collections::HashSet<_> =
            unused_classes.iter().map(|c| c.full_name.clone()).collect();

        for def in &methods_with_refs {
            if def.confidence >= self.confidence_threshold {
                // Skip visitor pattern methods - they have heuristic protection
                if def.simple_name.starts_with("visit_")
                    || def.simple_name.starts_with("leave_")
                    || def.simple_name.starts_with("transform_")
                {
                    continue;
                }

                // Extract parent class from full_name (e.g., "module.ClassName.method_name" -> "module.ClassName")
                if let Some(last_dot) = def.full_name.rfind('.') {
                    let parent_class = &def.full_name[..last_dot];
                    if unused_class_names.contains(parent_class) {
                        unused_methods.push(def.clone());
                    }
                }
            }
        }

        // Run taint analysis if enabled
        let taint_findings = if self.enable_taint {
            let taint_config = crate::taint::analyzer::TaintConfig::all_levels();
            let taint_analyzer = crate::taint::analyzer::TaintAnalyzer::new(taint_config);

            let file_sources: Vec<_> = files
                .iter()
                .filter_map(|file_path| {
                    let is_notebook = file_path.extension().is_some_and(|e| e == "ipynb");
                    let source = if is_notebook {
                        crate::ipynb::extract_notebook_code(file_path, Some(&self.analysis_root))
                            .ok()
                    } else {
                        fs::read_to_string(file_path).ok()
                    };
                    source.map(|s| (file_path.clone(), s))
                })
                .collect();

            file_sources
                .iter()
                .flat_map(|(path, source)| taint_analyzer.analyze_file(source, path))
                .collect()
        } else {
            Vec::new()
        };

        // Update file_metrics to include unused items count
        let mut unused_counts: FxHashMap<std::path::PathBuf, usize> = FxHashMap::default();
        let all_unused_slices: [&[Definition]; 6] = [
            &unused_functions,
            &unused_methods,
            &unused_imports,
            &unused_classes,
            &unused_variables,
            &unused_parameters,
        ];
        let all_unused = all_unused_slices.into_iter().flatten();

        for def in all_unused {
            *unused_counts.entry(def.file.as_ref().clone()).or_insert(0) += 1;
        }

        for metric in &mut file_metrics {
            if let Some(count) = unused_counts.get(&metric.file) {
                metric.total_issues += count;
            }
        }

        let taint_count = taint_findings.len();

        AnalysisResult {
            unused_functions,
            unused_methods,
            unused_imports,
            unused_classes,
            unused_variables,
            unused_parameters,
            secrets: all_secrets.clone(),
            danger: all_danger.clone(),
            quality: all_quality.clone(),
            taint_findings,
            parse_errors: all_parse_errors.clone(),
            clones: Vec::new(),
            analysis_summary: AnalysisSummary {
                total_files,
                secrets_count: all_secrets.len(),
                danger_count: all_danger.len(),
                quality_count: all_quality.len(),
                taint_count,
                parse_errors_count: all_parse_errors.len(),
                total_lines_analyzed: self.total_lines_analyzed,
                total_definitions,
                average_complexity: if files_with_quality_metrics > 0 {
                    total_complexity / f64::from(files_with_quality_metrics)
                } else {
                    0.0
                },
                average_mi: if files_with_quality_metrics > 0 {
                    total_mi / f64::from(files_with_quality_metrics)
                } else {
                    0.0
                },
                total_directories,
                total_size: total_size_bytes as f64 / 1024.0,
                functions_count,
                classes_count,
                raw_metrics: all_raw_metrics,
                halstead_metrics: all_halstead_metrics,
            },
            file_metrics,
        }
    }
}
