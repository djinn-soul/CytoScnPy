//! Processing methods for CytoScnPy analyzer.
//!
//! Contains: `process_single_file`, `aggregate_results`, `analyze`, `analyze_code`

use super::{AnalysisResult, CytoScnPy};
use crate::rules::secrets::{validate_secrets_config, SecretFinding};
use std::path::Path;

use crate::constants::{CHUNK_SIZE, CONFIG_FILENAME};
use rayon::prelude::*;

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

        // Validate secrets config once (not per-file)
        let mut config_errors: Vec<SecretFinding> = Vec::new();
        if self.enable_secrets {
            let config_file = self
                .config
                .config_file_path
                .clone()
                .unwrap_or_else(|| self.analysis_root.join(CONFIG_FILENAME));
            config_errors =
                validate_secrets_config(&self.config.cytoscnpy.secrets_config, &config_file);
        }

        // Process files in chunks to prevent OOM on large projects.
        // Each chunk is processed in parallel, then results are merged.
        // Process files in chunks to prevent OOM on large projects.
        // Each chunk is processed in parallel, then results are merged.
        let mut all_results = Vec::with_capacity(total_files);
        for chunk in files.chunks(CHUNK_SIZE) {
            // Check for cancellation signal
            if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let chunk_results: Vec<_> = chunk
                .par_iter()
                .map(|file_path| {
                    if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
                        // Return empty result if cancelled to finish quickly
                        return (
                            Vec::new(),
                            rustc_hash::FxHashMap::default(),
                            rustc_hash::FxHashMap::default(), // protocol methods
                            Vec::new(),
                            Vec::new(),
                            Vec::new(),
                            Vec::new(),
                            0,
                            crate::raw_metrics::RawMetrics::default(),
                            crate::halstead::HalsteadMetrics::default(),
                            0.0,
                            0.0,
                            0,
                        );
                    }
                    self.process_single_file(file_path, root_path)
                })
                .collect();
            all_results.extend(chunk_results);
        }

        // Aggregate and return results (same as analyze method)
        let mut result = self.aggregate_results(all_results, files, total_files, total_directories);

        // Prepend config validation errors to secrets (reported once, not per-file)
        if !config_errors.is_empty() {
            config_errors.extend(result.secrets);
            result.secrets = config_errors;
        }

        result
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
    pub fn analyze(&mut self, root_path: &Path) -> AnalysisResult {
        // Collect files and count directories using shared logic
        let (files, dir_count) = self.collect_python_files(root_path);

        self.total_files_analyzed = files.len();

        // Analyze the collected files
        self.analyze_file_list(&files, Some(root_path), dir_count)
    }
}

// Re-export utility functions for use in other analyzer modules
pub(crate) use super::utils::{collect_docstring_lines, convert_byte_range_to_line};
