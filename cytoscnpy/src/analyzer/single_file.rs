//! Single file analysis logic.

use super::{AnalysisResult, AnalysisSummary, CytoScnPy, ParseError};
#[cfg(feature = "cfg")]
use crate::cfg::flow::analyze_reaching_definitions;
#[cfg(feature = "cfg")]
use crate::cfg::Cfg;
use crate::framework::FrameworkAwareVisitor;
use crate::halstead::{analyze_halstead, HalsteadMetrics};
use crate::metrics::mi_compute;
use crate::raw_metrics::{analyze_raw, RawMetrics};
use crate::rules::secrets::{scan_secrets, SecretFinding};
use crate::rules::Finding;
use crate::test_utils::TestAwareVisitor;
use crate::utils::LineIndex;
use crate::visitor::{CytoScnPyVisitor, Definition};

use ruff_python_parser::parse_module;
use rustc_hash::FxHashMap;
use std::fs;
use std::path::Path;

use super::traversal::{collect_docstring_lines, convert_byte_range_to_line};
use super::{apply_heuristics, apply_penalties};

impl CytoScnPy {
    /// Processes a single file (from disk or notebook) and returns analysis results.
    /// Used by the directory traversal for high-performance scanning.
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    pub(crate) fn process_single_file(
        &self,
        file_path: &Path,
        root_path: &Path,
    ) -> (
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
        usize, // File size in bytes
    ) {
        let is_notebook = file_path.extension().is_some_and(|e| e == "ipynb");

        if let Some(delay_ms) = self.debug_delay_ms {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }

        let mut file_complexity = 0.0;
        let mut file_mi = 0.0;

        let source = if is_notebook {
            match crate::ipynb::extract_notebook_code(file_path, Some(&self.analysis_root)) {
                Ok(code) => code,
                Err(e) => {
                    return (
                        Vec::new(),
                        FxHashMap::default(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        vec![ParseError {
                            file: file_path.to_path_buf(),
                            error: format!("Failed to parse notebook: {e}"),
                        }],
                        0,
                        RawMetrics::default(),
                        HalsteadMetrics::default(),
                        0.0,
                        0.0,
                        0,
                    );
                }
            }
        } else {
            match fs::read_to_string(file_path) {
                Ok(code) => code,
                Err(e) => {
                    return (
                        Vec::new(),
                        FxHashMap::default(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        vec![ParseError {
                            file: file_path.to_path_buf(),
                            error: format!("Failed to read file: {e}"),
                        }],
                        0,
                        RawMetrics::default(),
                        HalsteadMetrics::default(),
                        0.0,
                        0.0,
                        0,
                    );
                }
            }
        };

        let file_size = source.len();
        let line_index = LineIndex::new(&source);
        let ignored_lines = crate::utils::get_ignored_lines(&source);

        let relative_path = file_path.strip_prefix(root_path).unwrap_or(file_path);
        let components: Vec<&str> = relative_path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        let mut module_parts = Vec::new();
        for (i, part) in components.iter().enumerate() {
            if i == components.len() - 1 {
                if let Some(stem) = Path::new(part).file_stem() {
                    let stem_str = stem.to_string_lossy();
                    if stem_str != "__init__" {
                        module_parts.push(stem_str.to_string());
                    }
                }
            } else {
                module_parts.push((*part).to_owned());
            }
        }
        let module_name = module_parts.join(".");

        let mut visitor =
            CytoScnPyVisitor::new(file_path.to_path_buf(), module_name.clone(), &line_index);
        let mut framework_visitor = FrameworkAwareVisitor::new(&line_index);
        let mut test_visitor = TestAwareVisitor::new(file_path, &line_index);

        let mut secrets = Vec::new();
        let mut danger = Vec::new();
        let mut quality = Vec::new();
        let mut parse_errors = Vec::new();

        match parse_module(&source) {
            Ok(parsed) => {
                let module = parsed.into_syntax();

                let mut docstring_lines = rustc_hash::FxHashSet::default();
                if self.enable_secrets && self.config.cytoscnpy.secrets_config.skip_docstrings {
                    collect_docstring_lines(&module.body, &line_index, &mut docstring_lines, 0);
                }

                if self.enable_secrets {
                    secrets = scan_secrets(
                        &source,
                        &file_path.to_path_buf(),
                        &self.config.cytoscnpy.secrets_config,
                        Some(&docstring_lines),
                    );
                }

                let entry_point_calls = crate::entry_point::detect_entry_point_calls(&module.body);

                for stmt in &module.body {
                    framework_visitor.visit_stmt(stmt);
                    test_visitor.visit_stmt(stmt);
                    visitor.visit_stmt(stmt);
                }

                for call_name in &entry_point_calls {
                    visitor.add_ref(call_name.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{call_name}");
                        visitor.add_ref(qualified);
                    }
                }

                for fw_ref in &framework_visitor.framework_references {
                    visitor.add_ref(fw_ref.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{fw_ref}");
                        visitor.add_ref(qualified);
                    }
                }

                let exports = visitor.exports.clone();
                for export_name in &exports {
                    visitor.add_ref(export_name.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{export_name}");
                        visitor.add_ref(qualified);
                    }
                }

                // 1. Synchronize reference counts from visitor's bag before refinement
                let ref_counts = visitor.references.clone();
                for def in &mut visitor.definitions {
                    if let Some(count) = ref_counts.get(&def.full_name) {
                        def.references = *count;
                    } else if def.def_type != "variable" && def.def_type != "parameter" {
                        if let Some(count) = ref_counts.get(&def.simple_name) {
                            def.references = *count;
                        }
                    }
                }

                // 1.5. Populate is_captured and mark as used if captured
                for def in &mut visitor.definitions {
                    if visitor.captured_definitions.contains(&def.full_name) {
                        def.is_captured = true;
                        def.references += 1;
                    }
                }

                // 2. Dynamic code handling
                if visitor.is_dynamic {
                    for def in &mut visitor.definitions {
                        def.references += 1;
                    }
                }

                // 3. Flow-sensitive refinement
                #[cfg(feature = "cfg")]
                if !visitor.is_dynamic {
                    Self::refine_flow_sensitive(&source, &mut visitor.definitions);
                }

                // 3. Apply penalties and heuristics
                for def in &mut visitor.definitions {
                    apply_penalties(
                        def,
                        &framework_visitor,
                        &test_visitor,
                        &ignored_lines,
                        self.include_tests,
                    );
                    apply_heuristics(def);

                    if def.def_type == "method"
                        && visitor.self_referential_methods.contains(&def.full_name)
                    {
                        def.is_self_referential = true;
                    }
                }

                let mut rules = Vec::new();
                if self.enable_danger {
                    rules.extend(crate::rules::danger::get_danger_rules());
                }
                if self.enable_quality {
                    rules.extend(crate::rules::quality::get_quality_rules(&self.config));
                }

                if !rules.is_empty() {
                    let mut linter = crate::linter::LinterVisitor::new(
                        rules,
                        file_path.to_path_buf(),
                        line_index.clone(),
                        self.config.clone(),
                    );
                    for stmt in &module.body {
                        linter.visit_stmt(stmt);
                    }

                    for finding in linter.findings {
                        if ignored_lines.contains(&finding.line) {
                            continue;
                        }
                        if finding.rule_id.starts_with("CSP-D") {
                            danger.push(finding);
                        } else if finding.rule_id.starts_with("CSP-Q")
                            || finding.rule_id.starts_with("CSP-L")
                            || finding.rule_id.starts_with("CSP-C")
                        {
                            quality.push(finding);
                        }
                    }
                }

                if self.enable_quality {
                    let raw = analyze_raw(&source);
                    let h_metrics = analyze_halstead(&ruff_python_ast::Mod::Module(module.clone()));
                    let volume = h_metrics.volume;
                    let complexity =
                        crate::complexity::calculate_module_complexity(&source).unwrap_or(1);
                    file_complexity = complexity as f64;
                    file_mi = mi_compute(volume, complexity, raw.sloc, raw.comments);

                    if let Some(min_mi) = self.config.cytoscnpy.min_mi {
                        if file_mi < min_mi {
                            quality.push(Finding {
                                message: format!(
                                    "Maintainability Index too low ({file_mi:.2} < {min_mi:.2})"
                                ),
                                rule_id: "CSP-Q303".to_owned(),
                                file: file_path.to_path_buf(),
                                line: 1,
                                col: 0,
                                severity: "HIGH".to_owned(),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                if self.enable_secrets {
                    secrets = scan_secrets(
                        &source,
                        &file_path.to_path_buf(),
                        &self.config.cytoscnpy.secrets_config,
                        None,
                    );
                }
                let error_msg = format!("{e}");
                let readable_error = convert_byte_range_to_line(&error_msg, &source);
                parse_errors.push(ParseError {
                    file: file_path.to_path_buf(),
                    error: readable_error,
                });
            }
        }

        if let Some(ref pb) = self.progress_bar {
            pb.inc(1);
        }

        (
            visitor.definitions,
            visitor.references,
            secrets,
            danger,
            quality,
            parse_errors,
            source.lines().count(),
            if self.enable_quality {
                analyze_raw(&source)
            } else {
                RawMetrics::default()
            },
            HalsteadMetrics::default(), // Computed later if needed
            file_complexity,
            file_mi,
            file_size,
        )
    }

    /// Analyzes a single string of code (mostly for testing).
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    #[must_use]
    pub fn analyze_code(&self, code: &str, file_path: &Path) -> AnalysisResult {
        let source = code.to_owned();
        let line_index = LineIndex::new(&source);
        let ignored_lines = crate::utils::get_ignored_lines(&source);

        let module_name = file_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut visitor = CytoScnPyVisitor::new(file_path.to_path_buf(), module_name, &line_index);
        let mut framework_visitor = FrameworkAwareVisitor::new(&line_index);
        let mut test_visitor = TestAwareVisitor::new(file_path, &line_index);

        let mut parse_errors = Vec::new();
        let mut secrets_res = Vec::new();
        let mut danger_res = Vec::new();
        let mut quality_res = Vec::new();

        match parse_module(&source) {
            Ok(parsed) => {
                let module = parsed.into_syntax();

                for stmt in &module.body {
                    framework_visitor.visit_stmt(stmt);
                    test_visitor.visit_stmt(stmt);
                    visitor.visit_stmt(stmt);
                }

                // Sync and Refine
                let ref_counts = visitor.references.clone();
                for def in &mut visitor.definitions {
                    if let Some(count) = ref_counts.get(&def.full_name) {
                        def.references = *count;
                    } else if def.def_type != "variable" && def.def_type != "parameter" {
                        if let Some(count) = ref_counts.get(&def.simple_name) {
                            def.references = *count;
                        }
                    }
                }

                // 1.5. Populate is_captured and mark as used if captured
                for def in &mut visitor.definitions {
                    if visitor.captured_definitions.contains(&def.full_name) {
                        def.is_captured = true;
                        def.references += 1;
                    }
                }

                // 1.5. Dynamic code handling
                if visitor.is_dynamic {
                    for def in &mut visitor.definitions {
                        def.references += 1;
                    }
                }

                #[cfg(feature = "cfg")]
                if !visitor.is_dynamic {
                    Self::refine_flow_sensitive(&source, &mut visitor.definitions);
                }

                for def in &mut visitor.definitions {
                    apply_penalties(
                        def,
                        &framework_visitor,
                        &test_visitor,
                        &ignored_lines,
                        self.include_tests,
                    );
                    apply_heuristics(def);
                }

                if self.enable_secrets {
                    let mut docstring_lines = rustc_hash::FxHashSet::default();
                    if self.config.cytoscnpy.secrets_config.skip_docstrings {
                        collect_docstring_lines(&module.body, &line_index, &mut docstring_lines, 0);
                    }
                    secrets_res = scan_secrets(
                        &source,
                        &file_path.to_path_buf(),
                        &self.config.cytoscnpy.secrets_config,
                        Some(&docstring_lines),
                    );
                }

                // Run LinterVisitor with enabled rules.
                let mut rules = Vec::new();
                if self.enable_danger {
                    rules.extend(crate::rules::danger::get_danger_rules());
                }
                if self.enable_quality {
                    rules.extend(crate::rules::quality::get_quality_rules(&self.config));
                }

                if !rules.is_empty() {
                    let mut linter = crate::linter::LinterVisitor::new(
                        rules,
                        file_path.to_path_buf(),
                        line_index.clone(),
                        self.config.clone(),
                    );
                    for stmt in &module.body {
                        linter.visit_stmt(stmt);
                    }

                    for finding in linter.findings {
                        if ignored_lines.contains(&finding.line) {
                            continue;
                        }
                        if finding.rule_id.starts_with("CSP-D") {
                            danger_res.push(finding);
                        } else if finding.rule_id.starts_with("CSP-Q")
                            || finding.rule_id.starts_with("CSP-L")
                            || finding.rule_id.starts_with("CSP-C")
                        {
                            quality_res.push(finding);
                        }
                    }
                }
            }
            Err(e) => {
                if self.enable_secrets {
                    secrets_res = scan_secrets(
                        &source,
                        &file_path.to_path_buf(),
                        &self.config.cytoscnpy.secrets_config,
                        None,
                    );
                }
                parse_errors.push(ParseError {
                    file: file_path.to_path_buf(),
                    error: format!("{e}"),
                });
            }
        }

        // Final aggregation
        let total_definitions = visitor.definitions.len();
        let mut unused_functions = Vec::new();
        let mut unused_methods = Vec::new();
        let mut unused_classes = Vec::new();
        let mut unused_imports = Vec::new();
        let mut unused_variables = Vec::new();
        let mut unused_parameters = Vec::new();

        for def in visitor.definitions {
            if def.confidence >= self.confidence_threshold && def.references == 0 {
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

        AnalysisResult {
            unused_functions,
            unused_methods,
            unused_imports,
            unused_classes,
            unused_variables,
            unused_parameters,
            secrets: secrets_res,
            danger: danger_res,
            quality: quality_res,
            taint_findings: Vec::new(),
            parse_errors,
            clones: Vec::new(),
            analysis_summary: AnalysisSummary {
                total_files: 1,
                total_lines_analyzed: source.lines().count(),
                total_definitions,
                ..AnalysisSummary::default()
            },
            file_metrics: Vec::new(),
        }
    }

    #[cfg(feature = "cfg")]
    fn refine_flow_sensitive(source: &str, definitions: &mut [Definition]) {
        let mut function_scopes: FxHashMap<String, (usize, usize)> = FxHashMap::default();
        for def in definitions.iter() {
            if def.def_type == "function" || def.def_type == "method" {
                function_scopes.insert(def.full_name.clone(), (def.line, def.end_line));
            }
        }

        for (func_name, (start_line, end_line)) in function_scopes {
            let lines: Vec<&str> = source
                .lines()
                .skip(start_line.saturating_sub(1))
                .take(end_line.saturating_sub(start_line) + 1)
                .collect();
            let func_source = lines.join("\n");
            let simple_name = func_name.split('.').next_back().unwrap_or(&func_name);

            if let Some(cfg) = Cfg::from_source(&func_source, simple_name) {
                let flow_results = analyze_reaching_definitions(&cfg);
                for def in definitions.iter_mut() {
                    if (def.def_type == "variable" || def.def_type == "parameter")
                        && def.full_name.starts_with(&func_name)
                    {
                        let relative_name = &def.full_name[func_name.len()..];
                        if let Some(var_name) = relative_name.strip_prefix('.') {
                            let rel_line = def.line.saturating_sub(start_line) + 1;
                            if !flow_results.is_def_used(&cfg, var_name, rel_line)
                                && def.references > 0
                                && !def.is_captured
                            {
                                def.references = 0;
                            }
                        }
                    }
                }
            }
        }
    }
}
