//! Processing methods for CytoScnPy analyzer.
//!
//! Contains: `process_single_file`, `aggregate_results`, `analyze`, `analyze_code`

use super::{
    apply_heuristics, apply_penalties, AnalysisResult, AnalysisSummary, CytoScnPy, ParseError,
};
use crate::framework::FrameworkAwareVisitor;
use crate::halstead::analyze_halstead;
use crate::metrics::mi_compute;
use crate::raw_metrics::analyze_raw;
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
use walkdir::WalkDir;

use crate::constants::DEFAULT_EXCLUDE_FOLDERS;

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
                let dir_files = self.collect_python_files(path);
                all_files.extend(dir_files);
            }
        }

        // Analyze the collected files
        self.analyze_file_list(&all_files, paths.first().map(std::path::PathBuf::as_path))
    }

    /// Collects all Python files from a directory, respecting exclusion rules.
    fn collect_python_files(&self, root_path: &Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        let mut it = WalkDir::new(root_path).into_iter();

        while let Some(res) = it.next() {
            if let Ok(entry) = res {
                let name = entry.file_name().to_string_lossy();

                // Check if this folder is explicitly included
                let is_force_included =
                    entry.file_type().is_dir() && self.include_folders.iter().any(|f| f == &name);

                // Check against both default excludes and user-provided excludes
                let should_exclude = entry.file_type().is_dir()
                    && !is_force_included
                    && (DEFAULT_EXCLUDE_FOLDERS().iter().any(|&f| f == name)
                        || self.exclude_folders.iter().any(|f| f == &name));

                if should_exclude {
                    it.skip_current_dir();
                    continue;
                }

                if entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "py" || (self.include_ipynb && ext == "ipynb"))
                {
                    files.push(entry.path().to_path_buf());
                }
            }
        }

        files
    }

    /// Analyzes a specific list of files.
    ///
    /// This is used internally when processing multiple paths or individual files.
    fn analyze_file_list(
        &mut self,
        files: &[std::path::PathBuf],
        root_hint: Option<&Path>,
    ) -> AnalysisResult {
        let total_files = files.len();
        self.total_files_analyzed = total_files;

        // Determine root path for relative path calculation
        let root_path = root_hint.unwrap_or(Path::new("."));

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
                f64,
                f64,
            )> = chunk
                .par_iter()
                .map(|file_path| self.process_single_file(file_path, root_path))
                .collect();
            all_results.extend(chunk_results);
            // Memory from previous chunk is released here before next iteration
        }

        // Aggregate and return results (same as analyze method)
        self.aggregate_results(all_results, files, total_files)
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
        f64,
        f64,
    ) {
        // Check if this is a notebook file
        let is_notebook = file_path.extension().is_some_and(|e| e == "ipynb");

        let mut file_complexity = 0.0;
        let mut file_mi = 0.0;

        // Get source code (from .py file or extracted from .ipynb)
        let source = if is_notebook {
            match crate::ipynb::extract_notebook_code(file_path) {
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
                        0.0,
                        0.0,
                    );
                }
            }
        } else {
            fs::read_to_string(file_path).unwrap_or_default()
        };

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

            // Mark self-referential methods (recursive methods)
            if def.def_type == "method" && visitor.self_referential_methods.contains(&def.full_name)
            {
                def.is_self_referential = true;
            }
        }

        (
            visitor.definitions,
            visitor.references,
            secrets,
            danger,
            quality,
            parse_errors,
            source.lines().count(),
            file_complexity,
            file_mi,
        )
    }

    /// Aggregates results from multiple file analyses.
    #[allow(clippy::too_many_lines)]
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
            f64,
            f64,
        )>,
        files: &[std::path::PathBuf],
        total_files: usize,
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
        let mut files_with_quality_metrics = 0;

        for (defs, refs, secrets, danger, quality, parse_errors, lines, complexity, mi) in results {
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

        // Class-method linking: Methods of unused classes should also be flagged
        // BUT ONLY if they are self-referential (recursive) and have no external references.
        let unused_class_names: std::collections::HashSet<_> =
            unused_classes.iter().map(|c| c.full_name.clone()).collect();

        for def in &methods_with_refs {
            if def.confidence >= self.confidence_threshold {
                // Only process if the method is marked as self-referential
                // This means the only reference comes from itself (recursion)
                if !def.is_self_referential {
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
                        crate::ipynb::extract_notebook_code(file_path).ok()
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

        // Class-method linking: Methods of unused classes should also be flagged
        // BUT ONLY if they are self-referential (recursive) and have no external references.
        let unused_class_names: std::collections::HashSet<_> =
            unused_classes.iter().map(|c| c.full_name.clone()).collect();

        for def in &methods_with_refs {
            if def.confidence >= self.confidence_threshold {
                // Only process if the method is marked as self-referential
                // This means the only reference comes from itself (recursion)
                if !def.is_self_referential {
                    continue;
                }

                if let Some(last_dot) = def.full_name.rfind('.') {
                    let parent_class = &def.full_name[..last_dot];
                    let is_unused = unused_class_names.contains(parent_class);
                    if is_unused {
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
            secrets: all_secrets.clone(),
            danger: all_danger.clone(),
            quality: all_quality.clone(),
            taint_findings,
            parse_errors: all_parse_errors.clone(),
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
            },
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
        // Find all Python files in the given path.
        // We use WalkDir to recursively traverse directories.
        let mut files: Vec<walkdir::DirEntry> = Vec::new();

        let mut it = WalkDir::new(root_path).into_iter();

        while let Some(res) = it.next() {
            if let Ok(entry) = res {
                let name = entry.file_name().to_string_lossy();

                // Check if this folder is explicitly included
                let is_force_included =
                    entry.file_type().is_dir() && self.include_folders.iter().any(|f| f == &name);

                // Check against both default excludes and user-provided excludes
                // BUT skip exclusion if the folder is force-included
                let should_exclude = entry.file_type().is_dir()
                    && !is_force_included
                    && (DEFAULT_EXCLUDE_FOLDERS().iter().any(|&f| f == name)
                        || self.exclude_folders.iter().any(|f| f == &name));

                if should_exclude {
                    it.skip_current_dir();
                    continue;
                }

                if entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "py" || (self.include_ipynb && ext == "ipynb"))
                {
                    files.push(entry);
                }
            }
        }

        let total_files = files.len();
        self.total_files_analyzed = total_files;

        // Process files in chunks on large projects (1000+ files).
        // Each chunk is processed in parallel, results are merged before next chunk.
        // This limits peak memory usage to approximately CHUNK_SIZE * avg_file_size.
        let mut all_defs = Vec::new();
        let mut ref_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut all_secrets = Vec::new();
        let mut all_danger = Vec::new();
        let mut all_quality = Vec::new();
        let mut all_parse_errors = Vec::new();

        let mut total_complexity = 0.0;
        let mut total_mi = 0.0;
        let mut files_with_quality_metrics = 0;

        for chunk in files.chunks(CHUNK_SIZE) {
            let chunk_results: Vec<(
                Vec<Definition>,
                FxHashMap<String, usize>,
                Vec<SecretFinding>,
                Vec<Finding>,
                Vec<Finding>,
                Vec<ParseError>,
                usize, // Line count
                f64,   // complexity
                f64,   // mi
            )> = chunk
                .par_iter()
                .map(|entry| self.process_single_file(entry.path(), root_path))
                .collect();

            // Aggregate chunk results immediately to release chunk memory
            for (defs, refs, secrets, danger, quality, parse_errors, lines, complexity, mi) in
                chunk_results
            {
                all_defs.extend(defs);
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
            // Memory from chunk_results is released here before next chunk
        }

        // Categorize unused definitions.
        let mut unused_functions = Vec::new();
        let mut unused_methods = Vec::new();
        let mut unused_classes = Vec::new();
        let mut unused_imports = Vec::new();
        let mut unused_variables = Vec::new();
        let mut unused_parameters = Vec::new();

        let total_definitions = all_defs.len();
        let mut methods_with_refs: Vec<Definition> = Vec::new();

        for mut def in all_defs {
            // Update the reference count for the definition.
            if let Some(count) = ref_counts.get(&def.full_name) {
                def.references = *count;
            }
            // Fallback: check simple name count if full name count is missing (for local vars/imports)
            else if let Some(count) = ref_counts.get(&def.simple_name) {
                def.references = *count;
            }

            // Apply advanced heuristics
            apply_heuristics(&mut def);

            // Filter out low confidence items based on the threshold.
            if def.confidence < self.confidence_threshold {
                continue;
            }

            // If reference count is 0, it is unused.
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
            } else if def.def_type == "method" {
                // Collect methods with references for possible class-method linking (recursion check)
                methods_with_refs.push(def.clone());
            }
        }

        // Class-method linking: Methods of unused classes should also be flagged
        // BUT ONLY if they are self-referential (recursive) and have no external references.
        let unused_class_names: std::collections::HashSet<_> =
            unused_classes.iter().map(|c| c.full_name.clone()).collect();

        for def in &methods_with_refs {
            if def.confidence >= self.confidence_threshold {
                // Only process if the method is marked as self-referential
                // This means the only reference comes from itself (recursion)
                if !def.is_self_referential {
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

            // Collect file sources that were successfully parsed
            let file_sources: Vec<_> = files
                .iter()
                .filter_map(|entry| {
                    let file_path = entry.path();
                    let is_notebook = file_path.extension().is_some_and(|e| e == "ipynb");
                    let source = if is_notebook {
                        crate::ipynb::extract_notebook_code(file_path).ok()
                    } else {
                        fs::read_to_string(file_path).ok()
                    };
                    source.map(|s| (file_path.to_path_buf(), s))
                })
                .collect();

            // Run taint analysis on each file
            file_sources
                .iter()
                .flat_map(|(path, source)| taint_analyzer.analyze_file(source, path))
                .collect()
        } else {
            Vec::new()
        };

        let taint_count = taint_findings.len();

        // Construct and return the final result.
        // Class-method linking: Methods of unused classes should also be flagged
        // BUT ONLY if they are self-referential (recursive) and have no external references.
        let unused_class_names: std::collections::HashSet<_> =
            unused_classes.iter().map(|c| c.full_name.clone()).collect();

        for def in &methods_with_refs {
            if def.confidence >= self.confidence_threshold {
                // Only process if the method is marked as self-referential
                // This means the only reference comes from itself (recursion)
                if !def.is_self_referential {
                    continue;
                }

                if let Some(last_dot) = def.full_name.rfind('.') {
                    let parent_class = &def.full_name[..last_dot];
                    let is_unused = unused_class_names.contains(parent_class);
                    if is_unused {
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
            secrets: all_secrets.clone(),
            danger: all_danger.clone(),
            quality: all_quality.clone(),
            taint_findings,
            parse_errors: all_parse_errors.clone(),
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
            },
        }
    }

    /// Analyzes a single string of code (mostly for testing).
    #[allow(clippy::too_many_lines)]
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

        // Class-method linking: Methods of unused classes should also be flagged
        // BUT ONLY if they are self-referential (recursive) and have no external references.
        let unused_class_names: std::collections::HashSet<_> =
            unused_classes.iter().map(|c| c.full_name.clone()).collect();

        for def in &methods_with_refs {
            if def.confidence >= self.confidence_threshold {
                // Only process if the method is marked as self-referential
                // This means the only reference comes from itself (recursion)
                if !def.is_self_referential {
                    continue;
                }

                if let Some(last_dot) = def.full_name.rfind('.') {
                    let parent_class = &def.full_name[..last_dot];
                    let is_unused = unused_class_names.contains(parent_class);
                    if is_unused {
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
            },
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
