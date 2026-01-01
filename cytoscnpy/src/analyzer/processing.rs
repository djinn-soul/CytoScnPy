//! Processing methods for CytoScnPy analyzer.
//!
//! Contains: `process_single_file`, `aggregate_results`, `analyze`, `analyze_code`

use super::{
    apply_heuristics, apply_penalties, AnalysisResult, AnalysisSummary, CytoScnPy, ParseError,
};
use crate::framework::FrameworkAwareVisitor;
use crate::halstead::{analyze_halstead, HalsteadMetrics};
use crate::metrics::mi_compute;
use crate::raw_metrics::{analyze_raw, RawMetrics};
use crate::rules::secrets::{scan_secrets, SecretFinding};
use crate::rules::Finding;
use crate::test_utils::TestAwareVisitor;
use crate::utils::LineIndex;
use crate::visitor::{CytoScnPyVisitor, Definition};

use ruff_python_ast::{Expr, Stmt};

use rayon::prelude::*;
use ruff_python_parser::parse_module;
use rustc_hash::FxHashMap;
use std::fs;
use std::path::Path;

/// Number of files to process per chunk in parallel processing.
/// Prevents OOM on very large projects (5000+ files) by limiting concurrent memory usage.
/// Set to 500 to balance memory safety with minimal overhead (~1-2% slower).
const CHUNK_SIZE: usize = 500;

impl CytoScnPy {
    /// Runs the analysis on multiple paths (files or directories).
    ///
    /// This method intelligently handles different input types:
    /// - Single directory: delegates to `analyze()` for full directory traversal
    /// - Multiple paths: processes each file/directory and merges results
    /// - Individual files: analyzes only the specified Python files
    ///
    /// This is the preferred entry point when accepting CLI input that may
    /// include multiple file paths (e.g., from pre-commit hooks).
    pub fn analyze_paths(&mut self, paths: &[std::path::PathBuf]) -> AnalysisResult {
        // If no paths provided, analyze current directory
        if paths.is_empty() {
            return self.analyze(Path::new("."));
        }

        // If single path that is a directory, use the standard analyze method
        if paths.len() == 1 && paths[0].is_dir() {
            return self.analyze(&paths[0]);
        }

        // For multiple paths or individual files, collect all Python files
        let mut all_files: Vec<std::path::PathBuf> = Vec::new();
        let mut total_directories = 0;

        for path in paths {
            if path.is_file() {
                // Direct file path - check if it's a Python file
                if path
                    .extension()
                    .is_some_and(|ext| ext == "py" || (self.include_ipynb && ext == "ipynb"))
                {
                    all_files.push(path.clone());
                }
            } else if path.is_dir() {
                // Directory - collect all Python files from it
                let (dir_files, dir_count) = self.collect_python_files(path);
                all_files.extend(dir_files);
                total_directories += dir_count;
            }
        }

        // Analyze the collected files
        self.analyze_file_list(
            &all_files,
            paths.first().map(std::path::PathBuf::as_path),
            total_directories,
        )
    }

    /// Collects all Python files from a directory, respecting exclusion rules.
    /// Uses gitignore-aware walking (respects .gitignore files) IN ADDITION to hardcoded defaults.
    /// Collects all Python files from a directory, respecting exclusion rules.
    /// Uses gitignore-aware walking (respects .gitignore files) IN ADDITION to hardcoded defaults.
    fn collect_python_files(&self, root_path: &Path) -> (Vec<std::path::PathBuf>, usize) {
        crate::utils::collect_python_files_gitignore(
            root_path,
            &self.exclude_folders,
            &self.include_folders,
            self.include_ipynb,
            self.verbose,
        )
    }

    /// Analyzes a specific list of files.
    ///
    /// This is used internally when processing multiple paths or individual files.
    fn analyze_file_list(
        &mut self,
        files: &[std::path::PathBuf],
        root_hint: Option<&Path>,
        total_directories: usize,
    ) -> AnalysisResult {
        let total_files = files.len();
        self.total_files_analyzed = total_files;

        // Determine root path for relative path calculation
        let root_path = root_hint.unwrap_or(&self.analysis_root);

        // Process files in chunks to prevent OOM on large projects.
        // Each chunk is processed in parallel, then results are merged.
        let mut all_results = Vec::with_capacity(total_files);
        for chunk in files.chunks(CHUNK_SIZE) {
            let chunk_results: Vec<(
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
            )> = chunk
                .par_iter()
                .map(|file_path| self.process_single_file(file_path, root_path))
                .collect();
            all_results.extend(chunk_results);
            // Memory from previous chunk is released here before next iteration
        }

        // Aggregate and return results (same as analyze method)
        self.aggregate_results(all_results, files, total_files, total_directories)
    }

    /// Processes a single file and returns its analysis results.
    #[allow(clippy::too_many_lines)]
    fn process_single_file(
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
                    collect_docstring_lines(&module.body, &line_index, &mut docstring_lines);
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
        let mut ref_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
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

            use crate::analyzer::types::FileMetrics;
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

    /// Runs the analysis on the specified path.
    ///
    /// This method:
    /// 1. Walks the directory tree to find Python files.
    /// 2. Processes files in parallel using `rayon`.
    /// 3. Parses each file into an AST.
    /// 4. Runs visitors to collect definitions, references, and findings.
    /// 5. Aggregates results from all files.
    /// 6. Calculates cross-file usage to identify unused code.
    /// 7. Returns the final `AnalysisResult`.
    #[allow(clippy::too_many_lines)]
    pub fn analyze(&mut self, root_path: &Path) -> AnalysisResult {
        // Collect files and count directories using shared logic
        let (files, dir_count) = self.collect_python_files(root_path);

        self.total_files_analyzed = files.len();

        // Analyze the collected files
        self.analyze_file_list(&files, Some(root_path), dir_count)
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
                    collect_docstring_lines(&module.body, &line_index, &mut docstring_lines);
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

/// Collects line numbers that belong to docstrings by traversing the AST.
fn collect_docstring_lines(
    body: &[Stmt],
    line_index: &LineIndex,
    docstrings: &mut rustc_hash::FxHashSet<usize>,
) {
    if let Some(Stmt::Expr(expr_stmt)) = body.first() {
        if let Expr::StringLiteral(string_lit) = &*expr_stmt.value {
            let start_line = line_index.line_index(string_lit.range.start());
            let end_line = line_index.line_index(string_lit.range.end());
            for i in start_line..=end_line {
                docstrings.insert(i);
            }
        }
    }

    for stmt in body {
        match stmt {
            Stmt::FunctionDef(f) => collect_docstring_lines(&f.body, line_index, docstrings),
            Stmt::ClassDef(c) => collect_docstring_lines(&c.body, line_index, docstrings),
            _ => {}
        }
    }
}

/// Converts byte range references in error messages to line numbers.
///
/// Ruff parser errors include "at byte range X..Y" which is not user-friendly.
/// This function replaces them with "at line N" for better readability.
fn convert_byte_range_to_line(error_msg: &str, source: &str) -> String {
    use regex::Regex;

    // Match "at byte range X..Y" or "byte range X..Y"
    let Ok(re) = Regex::new(r"(?:at )?byte range (\d+)\.\.(\d+)") else {
        return error_msg.to_owned();
    };

    re.replace_all(error_msg, |caps: &regex::Captures| {
        if let Ok(start_byte) = caps[1].parse::<usize>() {
            // Count newlines up to start_byte to find line number
            let line = source[..start_byte.min(source.len())].matches('\n').count() + 1;
            format!("at line {line}")
        } else {
            caps[0].to_string()
        }
    })
    .to_string()
}
