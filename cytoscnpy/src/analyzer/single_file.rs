//! Single file analysis logic.

use super::{AnalysisResult, AnalysisSummary, CytoScnPy, ParseError};
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

// use crate::constants::CHUNK_SIZE;

use super::{apply_heuristics, apply_penalties};
// Helper functions (these should be in a common place or in this file)
use super::traversal::{collect_docstring_lines, convert_byte_range_to_line};

impl CytoScnPy {
    /// Processes a single file and returns its analysis results.
    #[allow(clippy::too_many_lines)]
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
        // Check if this is a notebook file
        let is_notebook = file_path.extension().is_some_and(|e| e == "ipynb");

        // Debug delay for testing progress bar visibility
        if let Some(delay_ms) = self.debug_delay_ms {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }

        // Update progress bar (thread-safe)
        if let Some(ref pb) = self.progress_bar {
            pb.inc(1);
        }

        let mut file_complexity = 0.0;
        let mut file_mi = 0.0;

        // Get source code (from .py file or extracted from .ipynb)
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

        // Determine the module name from the file path
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

                // Advanced Secrets Scanning:
                // If skip_docstrings is enabled, we need to identify lines that are part of docstrings.
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

                if visitor.is_dynamic {
                    for def in &mut visitor.definitions {
                        def.references += 1;
                    }
                }

                for fw_ref in &framework_visitor.framework_references {
                    visitor.add_ref(fw_ref.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{fw_ref}");
                        visitor.add_ref(qualified);
                    }
                }

                // Mark names in __all__ as used (explicitly exported)
                let exports = visitor.exports.clone();
                for export_name in &exports {
                    visitor.add_ref(export_name.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{export_name}");
                        visitor.add_ref(qualified);
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
                        // Skip findings on pragma lines (# pragma: no cytoscnpy)
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

                // Calculate metrics if quality is enabled
                if self.enable_quality {
                    let raw = analyze_raw(&source);
                    let h_metrics = analyze_halstead(&ruff_python_ast::Mod::Module(module.clone()));
                    let volume = h_metrics.volume;
                    let complexity =
                        crate::complexity::calculate_module_complexity(&source).unwrap_or(1);

                    #[allow(clippy::cast_precision_loss)]
                    {
                        file_complexity = complexity as f64;
                    }
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
                // If we have a parse error but secrets scanning is enabled,
                // we should still try to scan for secrets (without docstring skipping).
                if self.enable_secrets {
                    secrets = scan_secrets(
                        &source,
                        &file_path.to_path_buf(),
                        &self.config.cytoscnpy.secrets_config,
                        None,
                    );
                }

                // Convert byte-based error to line-based for readability
                let error_msg = format!("{e}");
                let readable_error = convert_byte_range_to_line(&error_msg, &source);

                parse_errors.push(ParseError {
                    file: file_path.to_path_buf(),
                    error: readable_error,
                });
            }
        }

        for def in &mut visitor.definitions {
            apply_penalties(
                def,
                &framework_visitor,
                &test_visitor,
                &ignored_lines,
                self.include_tests,
            );

            // Mark self-referential methods (recursive methods)
            if def.def_type == "method" && visitor.self_referential_methods.contains(&def.full_name)
            {
                def.is_self_referential = true;
            }
        }

        let has_parse_errors = !parse_errors.is_empty();

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
            if self.enable_quality && has_parse_errors {
                HalsteadMetrics::default() // Cannot compute halstead if parse error
            } else if self.enable_quality {
                if let Ok(parsed) = parse_module(&source) {
                    analyze_halstead(&ruff_python_ast::Mod::Module(parsed.into_syntax()))
                } else {
                    HalsteadMetrics::default()
                }
            } else {
                HalsteadMetrics::default()
            },
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

        // Mock module name
        let module_name = file_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut visitor =
            CytoScnPyVisitor::new(file_path.to_path_buf(), module_name.clone(), &line_index);
        let mut framework_visitor = FrameworkAwareVisitor::new(&line_index);
        let mut test_visitor = TestAwareVisitor::new(file_path, &line_index);

        let mut secrets = Vec::new();
        let mut danger = Vec::new();

        let mut quality = Vec::new();
        let mut parse_errors = Vec::new();

        // Parse using ruff
        match ruff_python_parser::parse_module(&source) {
            Ok(parsed) => {
                let module = parsed.into_syntax();

                // Docstring extraction
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

                for stmt in &module.body {
                    framework_visitor.visit_stmt(stmt);
                    test_visitor.visit_stmt(stmt);
                    visitor.visit_stmt(stmt);
                }

                if visitor.is_dynamic {
                    for def in &mut visitor.definitions {
                        def.references += 1;
                    }
                }

                // Add framework-referenced functions/classes as used.
                for fw_ref in &framework_visitor.framework_references {
                    visitor.add_ref(fw_ref.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{fw_ref}");
                        visitor.add_ref(qualified);
                    }
                }

                // Mark names in __all__ as used (explicitly exported)
                let exports = visitor.exports.clone();
                for export_name in &exports {
                    visitor.add_ref(export_name.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{export_name}");
                        visitor.add_ref(qualified);
                    }
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

                    // Separate findings
                    for finding in linter.findings {
                        // Skip findings on pragma lines (# pragma: no cytoscnpy)
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
                parse_errors.push(ParseError {
                    file: file_path.to_path_buf(),
                    error: format!("{e}"),
                });
            }
        }

        for def in &mut visitor.definitions {
            apply_penalties(
                def,
                &framework_visitor,
                &test_visitor,
                &ignored_lines,
                self.include_tests,
            );
        }

        // Aggregate (single file)
        let total_definitions = visitor.definitions.len();

        let functions_count = visitor
            .definitions
            .iter()
            .filter(|d| d.def_type == "function" || d.def_type == "method")
            .count();
        let classes_count = visitor
            .definitions
            .iter()
            .filter(|d| d.def_type == "class")
            .count();

        let all_defs = visitor.definitions;
        // References are already counted by the visitor
        let ref_counts = visitor.references;

        let mut unused_functions = Vec::new();
        let mut unused_methods = Vec::new();
        let mut unused_classes = Vec::new();
        let mut unused_imports = Vec::new();
        let mut unused_variables = Vec::new();
        let mut unused_parameters = Vec::new();
        let mut methods_with_refs: Vec<Definition> = Vec::new();

        for mut def in all_defs {
            if let Some(count) = ref_counts.get(&def.full_name) {
                def.references = *count;
            } else if let Some(count) = ref_counts.get(&def.simple_name) {
                def.references = *count;
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

                if let Some(last_dot) = def.full_name.rfind('.') {
                    let parent_class = &def.full_name[..last_dot];
                    if unused_class_names.contains(parent_class) {
                        unused_methods.push(def.clone());
                    }
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
            secrets: secrets.clone(),
            danger: danger.clone(),
            quality: quality.clone(),
            taint_findings: Vec::new(),
            parse_errors: parse_errors.clone(),
            clones: Vec::new(),
            analysis_summary: AnalysisSummary {
                total_files: 1,
                secrets_count: secrets.len(),
                danger_count: danger.len(),
                quality_count: quality.len(),
                taint_count: 0,
                parse_errors_count: parse_errors.len(),
                total_lines_analyzed: source.lines().count(),
                total_definitions,
                average_complexity: 0.0,
                average_mi: 0.0,
                total_directories: 0,
                total_size: source.len() as f64 / 1024.0,
                functions_count,
                classes_count,
                raw_metrics: RawMetrics::default(),
                halstead_metrics: HalsteadMetrics::default(),
            },
            file_metrics: vec![crate::analyzer::types::FileMetrics {
                file: file_path.to_path_buf(),
                loc: source.lines().count(),
                sloc: source.lines().count(),
                complexity: 0.0,
                mi: 0.0,
                total_issues: danger.len() + quality.len() + secrets.len(),
            }],
        }
    }
}
