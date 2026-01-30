//! Aggregation of analysis results.

use super::{apply_heuristics, AnalysisResult, AnalysisSummary, CytoScnPy, FileAnalysisResult};
use crate::analyzer::types::FileMetrics;
use crate::halstead::HalsteadMetrics;
use crate::raw_metrics::RawMetrics;
use crate::visitor::Definition;

use crate::taint::call_graph::CallGraph;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fs;

impl CytoScnPy {
    /// Aggregates results from multiple file analyses.
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    pub(crate) fn aggregate_results(
        &mut self,
        results: Vec<FileAnalysisResult>,
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

        let mut all_protocols: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
        let mut global_call_graph = CallGraph::new();

        for (i, res) in results.into_iter().enumerate() {
            let FileAnalysisResult {
                definitions: defs,
                references: refs,
                protocol_methods: proto_methods,
                secrets,
                danger,
                quality,
                parse_errors,
                line_count: lines,
                raw_metrics: raw,
                halstead_metrics: halstead,
                complexity,
                mi,
                file_size: size,
                call_graph,
            } = res;
            global_call_graph.merge(call_graph);
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
            // Merge protocol definitions
            for (proto, methods) in proto_methods {
                all_protocols.entry(proto).or_default().extend(methods);
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

        // --- Phase 2: Duck Typing Logic ---
        // 1. Map Class -> Function Names
        let mut class_methods: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
        for def in &all_defs {
            if def.def_type == "method" {
                if let Some(parent) = def.full_name.rfind('.').map(|i| &def.full_name[..i]) {
                    class_methods
                        .entry(parent.to_owned())
                        .or_default()
                        .insert(def.simple_name.clone());
                }
            }
        }

        // 2. Identification of implicit implementations
        // Optimization: Inverted Index (Method -> Protocols)
        let mut method_to_protocols: FxHashMap<String, Vec<&String>> = FxHashMap::default();
        for (proto_name, methods) in &all_protocols {
            for method in methods {
                method_to_protocols
                    .entry(method.clone())
                    .or_default()
                    .push(proto_name);
            }
        }

        let mut implicitly_used_methods: FxHashSet<String> = FxHashSet::default();

        for (class_name, methods) in &class_methods {
            // Find candidate protocols
            let mut candidate_protocols: FxHashSet<&String> = FxHashSet::default();
            for method in methods {
                if let Some(protos) = method_to_protocols.get(method) {
                    for proto in protos {
                        candidate_protocols.insert(proto);
                    }
                }
            }

            for proto_name in candidate_protocols {
                if let Some(proto_methods) = all_protocols.get(proto_name) {
                    let intersection_count = methods.intersection(proto_methods).count();
                    let proto_len = proto_methods.len();

                    // Heuristic: >= 70% overlap and at least 3 methods matches
                    if proto_len > 0 && intersection_count >= 3 {
                        let ratio = intersection_count as f64 / proto_len as f64;
                        if ratio >= 0.7 {
                            // Match! Mark overlapping methods as implicitly used
                            for method in methods.intersection(proto_methods) {
                                implicitly_used_methods.insert(format!("{class_name}.{method}"));
                            }
                        }
                    }
                }
            }
        }

        // --- Phase 3: Whole-Program Reachability ---
        let mut roots = FxHashSet::default();
        let mut method_simple_to_full: FxHashMap<String, Vec<String>> = FxHashMap::default();

        for def in &all_defs {
            if def.def_type == "method" {
                method_simple_to_full
                    .entry(def.simple_name.clone())
                    .or_default()
                    .push(def.full_name.clone());
            }

            if def.is_exported
                || def.is_framework_managed
                || def.confidence == 0
                || implicitly_used_methods.contains(&def.full_name)
            {
                roots.insert(def.full_name.clone());

                // If this is a class, also treat its non-internal methods as roots (Public API)
                if def.def_type == "class" {
                    if let Some(methods) = class_methods.get(&def.full_name) {
                        for method in methods {
                            // Don't treat clearly internal methods as roots even if the class is exported.
                            if !method.starts_with('_') {
                                roots.insert(format!("{}.{}", def.full_name, method));
                            }
                        }
                    }
                }
            }
        }

        // Add roots from call graph (e.g., module-level entry points)
        for (name, node) in &global_call_graph.nodes {
            if node.is_root {
                roots.insert(name.clone());
            }
        }

        let mut reachable_nodes = FxHashSet::default();
        let mut stack: Vec<String> = roots.into_iter().collect();

        while let Some(current) = stack.pop() {
            if !reachable_nodes.insert(current.clone()) {
                continue;
            }

            if let Some(node) = global_call_graph.nodes.get(&current) {
                for call in &node.calls {
                    if let Some(attr_name) = call.strip_prefix('.') {
                        // Loose attribute hint - visit all matching methods
                        if let Some(methods) = method_simple_to_full.get(attr_name) {
                            for method_full in methods {
                                if !reachable_nodes.contains(method_full) {
                                    stack.push(method_full.clone());
                                }
                            }
                        }
                    } else if global_call_graph.nodes.contains_key(call) {
                        // Explicit node call
                        if !reachable_nodes.contains(call) {
                            stack.push(call.clone());
                        }
                    }
                }
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
                // For variables and parameters, if the cross-file count is 0,
                // stay at 0 to avoid false negatives from flow-sensitive analysis.
                // FIX: Previously checked def.references == 0 which blocked ALL
                // variables since references starts at 0. Now check *count == 0.
                if (def.def_type == "variable" || def.def_type == "parameter")
                    && !def.is_enum_member
                    && *count == 0
                {
                    // Stay at 0 - no references found
                } else {
                    def.references = *count;
                }
            } else {
                // full_name didn't match - try fallback strategies
                let mut matched = false;

                // For enum members, prefer qualified class.member matching
                if def.is_enum_member {
                    if let Some(dot_idx) = def.full_name.rfind('.') {
                        let parent = &def.full_name[..dot_idx];
                        if let Some(class_dot) = parent.rfind('.') {
                            let class_member =
                                format!("{}.{}", &parent[class_dot + 1..], def.simple_name);
                            if let Some(count) = ref_counts.get(&class_member) {
                                def.references = *count;
                                matched = true;
                            }
                        }
                    }
                    // For enum members, do NOT use bare-name fallback to prevent
                    // unrelated attributes from marking enum members as used
                }

                // Fallback to simple name for all non-enum types
                // This fixes cross-file references like `module.CONSTANT` where the
                // reference is tracked as simple name but def has full qualified path
                //
                // EXCEPTION: Do not do this for variables or imports to avoid conflating
                // local/scoped items (e.g. 'a', 'i', 'x', or local 'import re')
                // with global references to the same name.
                if !matched && !def.is_enum_member {
                    let should_fallback = def.def_type != "variable"
                        && def.def_type != "parameter"
                        && def.def_type != "import";

                    if should_fallback {
                        if let Some(count) = ref_counts.get(&def.simple_name) {
                            def.references = *count;
                        }
                    }
                }
            }

            apply_heuristics(&mut def);

            // 3. Duck Typing / Implicit Implementation Check
            if implicitly_used_methods.contains(&def.full_name) {
                def.references = std::cmp::max(def.references, 1);
            }

            // --- Phase 4: Reachability Refinement ---
            // If it's a function or class, it must be reachable from roots.
            // UNLESS it's an entry point itself.
            if (def.def_type == "function" || def.def_type == "method" || def.def_type == "class")
                && !reachable_nodes.contains(&def.full_name)
            {
                // Mark as unreachable
                def.is_unreachable = true;
                // It's not reachable. zero out references
                def.references = 0;
            }

            // Collect methods with references for class-method linking
            if def.def_type == "method" && def.references > 0 {
                methods_with_refs.push(def.clone());
            }

            if def.references == 0 {
                // Only filter by confidence if it's actually unused and we are about to report it.
                // Suggestions (confidence < threshold) are often still valuable in JSON, but benchmark usually focuses on thresholded items.
                if def.confidence >= self.confidence_threshold {
                    // Customize message if unreachable
                    if def.is_unreachable {
                        let type_label = match def.def_type.as_str() {
                            "function" => "function",
                            "method" => "method",
                            "class" => "class",
                            _ => &def.def_type,
                        };
                        def.message =
                            Some(format!("Unreachable {}: `{}`", type_label, def.simple_name));
                    }

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
        }
        let unused_class_names: std::collections::HashSet<_> =
            unused_classes.iter().map(|c| c.full_name.clone()).collect();
        let unreachable_class_names: std::collections::HashSet<_> = unused_classes
            .iter()
            .filter(|c| c.is_unreachable)
            .map(|c| c.full_name.clone())
            .collect();

        for def in &methods_with_refs {
            if def.confidence >= self.confidence_threshold {
                // Skip visitor pattern methods
                if def.simple_name.starts_with("visit_")
                    || def.simple_name.starts_with("leave_")
                    || def.simple_name.starts_with("transform_")
                {
                    continue;
                }

                // Skip lifecycle methods (framework callbacks)
                if def.simple_name.starts_with("on_")
                    || def.simple_name.starts_with("watch_")
                    || def.simple_name == "compose"
                {
                    continue;
                }

                if let Some(last_dot) = def.full_name.rfind('.') {
                    let parent_class = &def.full_name[..last_dot];
                    if unused_class_names.contains(parent_class) {
                        let mut m_def = def.clone();
                        // If the class is unreachable, the method is too.
                        if unreachable_class_names.contains(parent_class) {
                            m_def.is_unreachable = true;
                        }
                        unused_methods.push(m_def);
                    }
                }
            }
        }

        // Run taint analysis if enabled
        let taint_findings = if self.enable_danger
            && self
                .config
                .cytoscnpy
                .danger_config
                .enable_taint
                .unwrap_or(crate::constants::TAINT_ENABLED_DEFAULT)
        {
            let custom_sources = self
                .config
                .cytoscnpy
                .danger_config
                .custom_sources
                .clone()
                .unwrap_or_default();
            let custom_sinks = self
                .config
                .cytoscnpy
                .danger_config
                .custom_sinks
                .clone()
                .unwrap_or_default();
            let taint_config =
                crate::taint::analyzer::TaintConfig::with_custom(custom_sources, custom_sinks);
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

            let file_taint = file_sources
                .iter()
                .flat_map(|(path, source)| {
                    let path_ignored = crate::utils::get_ignored_lines(source);
                    let findings = taint_analyzer.analyze_file(source, path);
                    findings.into_iter().filter(move |f| {
                        !crate::utils::is_line_suppressed(&path_ignored, f.sink_line, &f.rule_id)
                    })
                })
                .collect::<Vec<_>>();

            file_taint
        } else {
            Vec::new()
        };

        // Update file_metrics
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
